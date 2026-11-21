use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

use super::error::CliError;
use super::locate::{existing_probe_bin_dirs, CliTarget, ProbeEnv};

/// Maximum captured stdout per subprocess invocation (64 MiB).
pub const MAX_STDOUT_BYTES: usize = 64 * 1024 * 1024;
const MAX_STDERR_BYTES: usize = 4 * 1024 * 1024;

type WaiterList = Vec<oneshot::Sender<Result<String, CliError>>>;
type InFlightMap = Arc<Mutex<HashMap<String, WaiterList>>>;

struct QueueJob {
    dedup_key: String,
    program: PathBuf,
    args: Vec<String>,
    timeout: Duration,
    env: ProbeEnv,
}

/// Serialized subprocess queue with NUL-joined argv deduplication and per-job timeouts.
#[derive(Clone)]
pub struct CliQueue {
    tx: mpsc::UnboundedSender<QueueJob>,
    in_flight: InFlightMap,
}

impl CliQueue {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<QueueJob>();
        let in_flight: InFlightMap = Arc::new(Mutex::new(HashMap::new()));
        let worker_in_flight = Arc::clone(&in_flight);

        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.spawn(worker_loop(rx, worker_in_flight));
            }
            Err(_) => {
                std::thread::Builder::new()
                    .name("tokscale-cli-queue".to_string())
                    .spawn(move || {
                        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                        {
                            rt.block_on(worker_loop(rx, worker_in_flight));
                        }
                    })
                    .expect("spawn tokscale-cli-queue thread");
            }
        }

        Self { tx, in_flight }
    }

    /// Execute a `tokscale` command through the serialized queue.
    pub async fn run_tokscale(
        &self,
        target: CliTarget,
        argv: Vec<String>,
        timeout: Duration,
        env: ProbeEnv,
    ) -> Result<String, CliError> {
        let (program, args) = match target {
            CliTarget::Direct { path, .. } => (path, argv.clone()),
            CliTarget::Runner { runner_path, .. } => {
                let mut full = Vec::with_capacity(argv.len() + 1);
                full.push("tokscale".to_string());
                full.extend(argv.iter().cloned());
                (runner_path, full)
            }
            CliTarget::None => return Err(CliError::NotFound),
        };

        let dedup_key = argv.join("\0");
        self.enqueue(dedup_key, program, args, timeout, env).await
    }

    /// Execute a resolved external tool (e.g. `bun`, `npm`) through the same serialized queue.
    pub async fn run_tool(
        &self,
        program: PathBuf,
        args: Vec<String>,
        timeout: Duration,
        env: ProbeEnv,
    ) -> Result<String, CliError> {
        let dedup_key = format!(
            "\u{0001}tool\0{}\0{}",
            program.to_string_lossy(),
            args.join("\0")
        );
        self.enqueue(dedup_key, program, args, timeout, env).await
    }

    async fn enqueue(
        &self,
        dedup_key: String,
        program: PathBuf,
        args: Vec<String>,
        timeout: Duration,
        env: ProbeEnv,
    ) -> Result<String, CliError> {
        let (tx_waiter, rx_waiter) = oneshot::channel();

        {
            let mut guard = self
                .in_flight
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            if let Some(waiters) = guard.get_mut(&dedup_key) {
                waiters.push(tx_waiter);
            } else {
                guard.insert(dedup_key.clone(), vec![tx_waiter]);
                let job = QueueJob {
                    dedup_key: dedup_key.clone(),
                    program,
                    args,
                    timeout,
                    env,
                };
                if self.tx.send(job).is_err() {
                    guard.remove(&dedup_key);
                    return Err(CliError::NonZeroExit {
                        code: None,
                        stderr: "CLI queue worker stopped".to_string(),
                    });
                }
            }
        }

        rx_waiter.await.unwrap_or_else(|_| {
            Err(CliError::NonZeroExit {
                code: None,
                stderr: "CLI queue worker dropped response".to_string(),
            })
        })
    }
}

async fn worker_loop(mut rx: mpsc::UnboundedReceiver<QueueJob>, in_flight: InFlightMap) {
    while let Some(job) = rx.recv().await {
        let result = execute_job(&job.program, &job.args, job.timeout, &job.env).await;

        let waiters = {
            let mut guard = in_flight
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.remove(&job.dedup_key).unwrap_or_default()
        };

        for waiter in waiters {
            let _ = waiter.send(result.clone());
        }
    }
}

async fn execute_job(
    program: &Path,
    args: &[String],
    timeout: Duration,
    env: &ProbeEnv,
) -> Result<String, CliError> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    configure_child_env(&mut cmd, program, env);

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Err(CliError::NotFound),
        Err(err) => {
            return Err(CliError::NonZeroExit {
                code: None,
                stderr: err.to_string(),
            });
        }
    };

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let run_fut = async {
        let stdout_task = tokio::spawn(read_capped(stdout_pipe, MAX_STDOUT_BYTES));
        let stderr_task = tokio::spawn(read_capped(stderr_pipe, MAX_STDERR_BYTES));

        let status = child.wait().await;
        let stdout_res = stdout_task.await.unwrap_or(Ok((Vec::new(), false)));
        let stderr_res = stderr_task.await.unwrap_or(Ok((Vec::new(), false)));
        (status, stdout_res, stderr_res)
    };

    let Ok((status_res, stdout_res, stderr_res)) = tokio::time::timeout(timeout, run_fut).await
    else {
        let _ = child.kill().await;
        let _ = child.wait().await;
        return Err(CliError::Timeout);
    };

    let (stdout_bytes, stdout_truncated) = match stdout_res {
        Ok(val) => val,
        Err(err) => {
            return Err(CliError::NonZeroExit {
                code: None,
                stderr: err.to_string(),
            });
        }
    };

    if stdout_truncated {
        return Err(CliError::OutputTooLarge);
    }

    let stderr_bytes = stderr_res.map(|(b, _)| b).unwrap_or_default();

    let status = match status_res {
        Ok(status) => status,
        Err(err) => {
            return Err(CliError::NonZeroExit {
                code: None,
                stderr: err.to_string(),
            });
        }
    };

    if !status.success() {
        return Err(CliError::NonZeroExit {
            code: status.code(),
            stderr: String::from_utf8_lossy(&stderr_bytes).trim().to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&stdout_bytes).into_owned())
}

async fn read_capped<R: tokio::io::AsyncRead + Unpin>(
    pipe: Option<R>,
    limit: usize,
) -> Result<(Vec<u8>, bool), std::io::Error> {
    let Some(mut reader) = pipe else {
        return Ok((Vec::new(), false));
    };

    let mut out = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut exceeded = false;

    loop {
        let n = reader.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        if !exceeded {
            if out.len().saturating_add(n) > limit {
                exceeded = true;
            } else {
                out.extend_from_slice(&chunk[..n]);
            }
        }
    }

    Ok((out, exceeded))
}

fn configure_child_env(cmd: &mut Command, program: &Path, env: &ProbeEnv) {
    if let Some(ref home) = env.home {
        cmd.env("HOME", home);
    }

    if let Some(ref cfg_dir) = env.tokscale_config_dir {
        cmd.env("TOKSCALE_CONFIG_DIR", cfg_dir);
    } else if env.clear_tokscale_config_dir {
        cmd.env_remove("TOKSCALE_CONFIG_DIR");
    }

    let mut path_entries: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    if let Some(parent) = program.parent().filter(|p| !p.as_os_str().is_empty()) {
        let pb = parent.to_path_buf();
        if seen.insert(pb.clone()) {
            path_entries.push(pb);
        }
    }

    if let Some(ref existing_path) = env.path {
        for dir in std::env::split_paths(existing_path) {
            if seen.insert(dir.clone()) {
                path_entries.push(dir);
            }
        }
    }

    for probe_dir in existing_probe_bin_dirs(env) {
        if seen.insert(probe_dir.clone()) {
            path_entries.push(probe_dir);
        }
    }

    if let Ok(joined) = std::env::join_paths(&path_entries) {
        cmd.env("PATH", joined);
    }

    cmd.env("NO_COLOR", "1");
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Arc, RwLock};
    use tempfile::TempDir;

    use super::*;
    use crate::cli::locate::write_test_script;
    use crate::cli::CliService;
    use crate::contracts::CompanionPreferences;

    #[cfg(unix)]
    #[tokio::test]
    async fn identical_concurrent_calls_dedupe_to_single_process() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let bin_dir = tmp.path().join("bin");
        let count_file = tmp.path().join("invocations.log");
        let script = bin_dir.join("tokscale");

        write_test_script(
            &script,
            &format!(
                "#!/bin/sh\necho x >> \"{}\"\nsleep 0.15\necho '{{\"ok\":true}}'\n",
                count_file.display()
            ),
        );

        let env = ProbeEnv::isolated(home).with_test_path([bin_dir]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(prefs, env);

        let argv = vec!["clients".to_string(), "--json".to_string()];
        let (r1, r2) = tokio::join!(
            service.run_json::<serde_json::Value>(argv.clone(), Duration::from_secs(10)),
            service.run_json::<serde_json::Value>(argv, Duration::from_secs(10)),
        );

        assert_eq!(r1.unwrap()["ok"], true);
        assert_eq!(r2.unwrap()["ok"], true);

        let log = fs::read_to_string(&count_file).unwrap();
        assert_eq!(log.lines().count(), 1, "expected 1 spawned process, got: {log}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn different_concurrent_jobs_serialize_with_no_overlap() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let bin_dir = tmp.path().join("bin");
        let timeline_file = tmp.path().join("timeline.log");
        let script = bin_dir.join("tokscale");

        write_test_script(
            &script,
            &format!(
                "#!/bin/sh\nnow_ms() {{\n  if command -v python3 >/dev/null 2>&1; then\n    python3 -c \"import time; print(int(time.time()*1000))\"\n  else\n    perl -MTime::HiRes=time -e 'printf \"%d\\n\", time()*1000'\n  fi\n}}\nSTART=$(now_ms)\nsleep 0.15\nEND=$(now_ms)\necho \"$1 $START $END\" >> \"{}\"\necho '{{\"job\":\"'\"$1\"'\"}}'\n",
                timeline_file.display()
            ),
        );

        let env = ProbeEnv::isolated(home).with_test_path([bin_dir]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(prefs, env);

        let (r1, r2) = tokio::join!(
            service.run_json::<serde_json::Value>(
                vec!["clients".to_string(), "--json".to_string()],
                Duration::from_secs(10)
            ),
            service.run_json::<serde_json::Value>(
                vec!["usage".to_string(), "--json".to_string()],
                Duration::from_secs(10)
            ),
        );

        assert!(r1.is_ok());
        assert!(r2.is_ok());

        let log = fs::read_to_string(&timeline_file).unwrap();
        let entries: Vec<(String, u128, u128)> = log
            .lines()
            .map(|line| {
                let mut parts = line.split_whitespace();
                let name = parts.next().unwrap().to_string();
                let start: u128 = parts.next().unwrap().parse().unwrap();
                let end: u128 = parts.next().unwrap().parse().unwrap();
                (name, start, end)
            })
            .collect();

        assert_eq!(entries.len(), 2, "expected 2 timeline rows, got: {log}");
        assert!(
            entries[1].1 >= entries[0].2,
            "second job started ({}) before first ended ({})",
            entries[1].1,
            entries[0].2
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn timeout_returns_cli_error_timeout() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let bin_dir = tmp.path().join("bin");
        let script = bin_dir.join("tokscale");
        write_test_script(&script, "#!/bin/sh\nsleep 5\n");

        let env = ProbeEnv::isolated(home).with_test_path([bin_dir]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(prefs, env);

        let res = service
            .run_raw(vec!["--version".to_string()], Duration::from_millis(80))
            .await;
        assert_eq!(res, Err(CliError::Timeout));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn non_zero_exit_and_invalid_json_are_reported() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let bin_dir = tmp.path().join("bin");
        let script = bin_dir.join("tokscale");
        write_test_script(
            &script,
            "#!/bin/sh\nif [ \"$1\" = \"fail\" ]; then\n  echo 'boom line 1' >&2\n  exit 2\nfi\necho 'not-json'\n",
        );

        let env = ProbeEnv::isolated(home).with_test_path([bin_dir]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(prefs, env);

        let err_exit = service
            .run_raw(vec!["fail".to_string()], Duration::from_secs(5))
            .await;
        assert_eq!(
            err_exit,
            Err(CliError::NonZeroExit {
                code: Some(2),
                stderr: "boom line 1".to_string(),
            })
        );

        let err_json = service
            .run_json::<serde_json::Value>(vec!["clients".to_string()], Duration::from_secs(5))
            .await;
        assert!(matches!(err_json, Err(CliError::InvalidJson(_))));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn child_env_inherits_home_config_dir_no_color_and_prepends_probe_bin() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let cfg = tmp.path().join("custom-cfg");
        let probe_bin = home.join(".bun/bin");
        let script = probe_bin.join("tokscale");
        write_test_script(
            &script,
            "#!/bin/sh\nprintf '{\"home\":\"%s\",\"cfg\":\"%s\",\"noColor\":\"%s\",\"firstPath\":\"%s\"}\\n' \"$HOME\" \"$TOKSCALE_CONFIG_DIR\" \"$NO_COLOR\" \"${PATH%%:*}\"\n",
        );

        let env = ProbeEnv::isolated(home.clone())
            .with_test_path([])
            .with_tokscale_config_dir(cfg.clone());
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(prefs, env);

        let out: serde_json::Value = service
            .run_json(vec!["clients".to_string()], Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(out["home"], home.to_string_lossy().as_ref());
        assert_eq!(out["cfg"], cfg.to_string_lossy().as_ref());
        assert_eq!(out["noColor"], "1");
        assert_eq!(out["firstPath"], probe_bin.to_string_lossy().as_ref());
    }

    #[tokio::test]
    async fn read_capped_flags_oversized_stdout() {
        use tokio::io::AsyncWriteExt;

        let (mut writer, reader) = tokio::io::duplex(64);
        tokio::spawn(async move {
            let _ = writer.write_all(&[b'a'; 128]).await;
        });

        let (bytes, exceeded) = read_capped(Some(reader), 64).await.unwrap();
        assert!(exceeded);
        assert!(bytes.len() <= 64);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn bunx_fallback_invokes_runner_with_tokscale_subcommand() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let bun_bin = home.join(".bun/bin");
        let bunx_script = bun_bin.join("bunx");
        write_test_script(
            &bunx_script,
            "#!/bin/sh\nprintf '{\"argv\":\"%s\"}\\n' \"$*\"\n",
        );

        let env = ProbeEnv::isolated(home).with_test_path([]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(prefs, env);

        let out: serde_json::Value = service
            .run_json(
                vec!["clients".to_string(), "--json".to_string()],
                Duration::from_secs(5),
            )
            .await
            .unwrap();
        assert_eq!(out["argv"], "tokscale clients --json");
    }
}
