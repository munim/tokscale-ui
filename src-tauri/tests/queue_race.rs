#[cfg(unix)]
mod unix_queue_race_tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, RwLock};

    use tempfile::TempDir;
    use tokio::task::JoinSet;
    use tokscale_ui_lib::cli::{CliService, ProbeEnv};
    use tokscale_ui_lib::commands::CliServiceRunner;
    use tokscale_ui_lib::contracts::CompanionPreferences;
    use tokscale_ui_lib::data::source::CliRunner;

    fn write_exec_script(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create script parent");
        }
        fs::write(path, body).expect("write script");
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod +x script");
    }

    #[tokio::test]
    async fn eight_concurrent_identical_run_json_calls_spawn_single_process() {
        let tmp = TempDir::new().expect("tempdir");
        let home = tmp.path().join("home");
        let bin_dir = tmp.path().join("bin");
        let count_file = tmp.path().join("invocations.log");
        let script = bin_dir.join("tokscale");

        write_exec_script(
            &script,
            &format!(
                "#!/bin/sh\necho invoked >> \"{}\"\nsleep 0.15\necho '{{\"ok\":true}}'\n",
                count_file.display()
            ),
        );

        let env = ProbeEnv::isolated(home).with_path([
            bin_dir,
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
        ]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let cli = Arc::new(CliService::with_env(prefs, env));
        let runner = Arc::new(CliServiceRunner(cli));

        let argv = vec!["clients".to_string(), "--json".to_string()];
        let mut set = JoinSet::new();
        for _ in 0..8 {
            let r = Arc::clone(&runner);
            let args = argv.clone();
            set.spawn(async move { r.run_json(&args).await });
        }

        let mut results = Vec::new();
        while let Some(res) = set.join_next().await {
            results.push(res.expect("task should not panic").expect("run_json ok"));
        }

        assert_eq!(results.len(), 8);
        for val in results {
            assert_eq!(val["ok"], true);
        }

        let log = fs::read_to_string(&count_file).expect("read invocation count log");
        assert_eq!(
            log.lines().count(),
            1,
            "expected exactly 1 spawned child process for 8 identical concurrent calls, got: {log}"
        );
    }

    #[tokio::test]
    async fn concurrent_distinct_run_json_calls_serialize_with_zero_overlap() {
        let tmp = TempDir::new().expect("tempdir");
        let home = tmp.path().join("home");
        let bin_dir = tmp.path().join("bin");
        let timeline_file = tmp.path().join("timeline.log");
        let script = bin_dir.join("tokscale");

        write_exec_script(
            &script,
            &format!(
                "#!/bin/sh\nnow_ms() {{\n  if command -v python3 >/dev/null 2>&1; then\n    python3 -c \"import time; print(int(time.time()*1000))\"\n  else\n    perl -MTime::HiRes=time -e 'printf \"%d\\n\", time()*1000'\n  fi\n}}\nSTART=$(now_ms)\nsleep 0.12\nEND=$(now_ms)\necho \"$1 $START $END\" >> \"{}\"\necho '{{\"cmd\":\"'\"$1\"'\"}}'\n",
                timeline_file.display()
            ),
        );

        let env = ProbeEnv::isolated(home).with_path([
            bin_dir,
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
        ]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let cli = Arc::new(CliService::with_env(prefs, env));
        let runner = Arc::new(CliServiceRunner(cli));

        let commands = [
            vec!["clients".to_string(), "--json".to_string()],
            vec!["usage".to_string(), "--json".to_string()],
            vec!["daily".to_string(), "--json".to_string()],
            vec!["monthly".to_string(), "--json".to_string()],
        ];

        let mut set = JoinSet::new();
        for args in commands {
            let r = Arc::clone(&runner);
            set.spawn(async move { r.run_json(&args).await });
        }

        while let Some(res) = set.join_next().await {
            assert!(res.expect("task should not panic").is_ok());
        }

        let log = fs::read_to_string(&timeline_file).expect("read timeline log");
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

        assert_eq!(entries.len(), 4, "expected 4 serialized runs, got: {log}");
        for pair in entries.windows(2) {
            assert!(
                pair[1].1 >= pair[0].2,
                "overlapping execution detected: {} ({}..{}) vs {} ({}..{})",
                pair[0].0,
                pair[0].1,
                pair[0].2,
                pair[1].0,
                pair[1].1,
                pair[1].2
            );
        }
    }
}
