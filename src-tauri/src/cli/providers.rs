use std::path::PathBuf;

use crate::contracts::SourceWarning;

use super::commands;
use super::error::CliError;
use super::locate::ProbeEnv;
use super::CliService;

/// API-backed providers that support `tokscale <provider> sync`.
pub const PROVIDER_SYNC: &[&str] = &["cursor", "trae", "warp", "capy", "antigravity"];

/// Returns `true` if `provider` is one of the API-backed providers requiring sync.
pub fn is_api_backed_provider(provider: &str) -> bool {
    PROVIDER_SYNC.contains(&provider)
}

/// Returns candidate credential file paths for `provider` without reading file contents.
pub fn credential_paths_for(provider: &str, env: &ProbeEnv) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let config_dir = env.tokscale_config_dir();
    let home = env.home.as_ref();

    match provider {
        "cursor" => {
            if let Some(ref cfg) = config_dir {
                paths.push(cfg.join("cursor-credentials.json"));
            }
            if let Some(h) = home {
                paths.push(h.join(".tokscale").join("cursor-credentials.json"));
            }
        }
        "trae" => {
            if let Some(ref cfg) = config_dir {
                let cache = cfg.join("trae-cache");
                paths.push(cache.join("credentials-solo.json"));
                paths.push(cache.join("credentials-ide.json"));
            }
        }
        "warp" => {
            if let Some(ref cfg) = config_dir {
                paths.push(cfg.join("warp-cache").join("credentials.json"));
            }
        }
        "capy" => {
            if let Some(ref cfg) = config_dir {
                paths.push(cfg.join("capy-cache").join("credentials.json"));
            }
        }
        "antigravity" => {
            if let Some(ref cfg) = config_dir {
                let cache = cfg.join("antigravity-cache");
                paths.push(cache.join("credentials.json"));
                paths.push(cache.join("manifest.json"));
            }
            if let Some(h) = home {
                paths.push(h.join(".gemini").join("antigravity").join("oauth_creds.json"));
                paths.push(h.join(".antigravity").join("credentials.json"));
            }
        }
        _ => {}
    }

    paths
}

/// Filesystem-only check for whether `provider` has a credential file on disk.
///
/// Never opens, reads, or logs secret contents.
pub fn is_provider_credentialed(provider: &str, env: &ProbeEnv) -> bool {
    credential_paths_for(provider, env)
        .into_iter()
        .any(|path| std::fs::metadata(path).map(|m| m.is_file()).unwrap_or(false))
}

/// Return the subset of `PROVIDER_SYNC` providers that have credential files present.
pub fn credentialed_providers(env: &ProbeEnv) -> Vec<&'static str> {
    PROVIDER_SYNC
        .iter()
        .copied()
        .filter(|provider| is_provider_credentialed(provider, env))
        .collect()
}

pub(crate) async fn sync_single_provider(
    service: &CliService,
    provider: &str,
) -> Result<(), SourceWarning> {
    let argv = commands::provider_sync(provider);
    match service.run_raw(argv, commands::TIMEOUT_LONG).await {
        Ok(_) => Ok(()),
        Err(err) => Err(SourceWarning {
            source: provider.to_string(),
            message: warning_message_from_error(&err),
        }),
    }
}

pub(crate) async fn sync_all_credentialed(service: &CliService) -> Vec<SourceWarning> {
    let mut warnings = Vec::new();
    for provider in credentialed_providers(service.env()) {
        if let Err(warning) = sync_single_provider(service, provider).await {
            warnings.push(warning);
        }
    }
    warnings
}

fn warning_message_from_error(err: &CliError) -> String {
    match err {
        CliError::NonZeroExit { stderr, .. } => {
            let first_line = stderr
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty());
            match first_line {
                Some(line) => line.to_string(),
                None => err.to_string(),
            }
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Arc, RwLock};
    use tempfile::TempDir;

    use super::*;
    use crate::cli::locate::write_test_script;
    use crate::contracts::CompanionPreferences;

    #[test]
    fn detects_only_credentialed_providers_without_reading_contents() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let cfg = home.join(".config/tokscale");
        fs::create_dir_all(cfg.join("warp-cache")).unwrap();
        fs::create_dir_all(cfg.join("trae-cache")).unwrap();

        // Write sentinel files; detection must only check metadata, never read contents.
        fs::write(cfg.join("cursor-credentials.json"), "SECRET_TOKEN_NEVER_LOG").unwrap();
        fs::write(cfg.join("warp-cache/credentials.json"), "WARP_SECRET").unwrap();
        fs::write(
            cfg.join("trae-cache/credentials-ide.json"),
            "TRAE_SECRET",
        )
        .unwrap();

        let env = ProbeEnv::isolated(home);
        assert_eq!(credentialed_providers(&env), vec!["cursor", "trae", "warp"]);
        assert!(is_provider_credentialed("cursor", &env));
        assert!(is_provider_credentialed("trae", &env));
        assert!(is_provider_credentialed("warp", &env));
        assert!(!is_provider_credentialed("capy", &env));
        assert!(!is_provider_credentialed("antigravity", &env));
        assert!(is_api_backed_provider("capy"));
        assert!(!is_api_backed_provider("opencode"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sync_providers_runs_only_credentialed_and_captures_first_stderr_line() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let cfg = home.join(".config/tokscale");
        let bin_dir = tmp.path().join("bin");
        let invoked_log = tmp.path().join("sync-calls.log");

        fs::create_dir_all(cfg.join("capy-cache")).unwrap();
        fs::write(cfg.join("cursor-credentials.json"), "CURSOR_SECRET_DO_NOT_LOG").unwrap();
        fs::write(cfg.join("capy-cache/credentials.json"), "CAPY_SECRET_DO_NOT_LOG").unwrap();

        let script = bin_dir.join("tokscale");
        write_test_script(
            &script,
            &format!(
                "#!/bin/sh\necho \"$*\" >> \"{}\"\nif [ \"$1\" = \"capy\" ]; then\n  echo 'capy rate limit reached' >&2\n  echo 'second diagnostic line' >&2\n  exit 1\nfi\necho 'ok'\n",
                invoked_log.display()
            ),
        );

        let env = ProbeEnv::isolated(home).with_path([bin_dir]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(prefs, env);

        let warnings = service.sync_providers().await;
        assert_eq!(
            warnings,
            vec![SourceWarning {
                source: "capy".to_string(),
                message: "capy rate limit reached".to_string(),
            }]
        );

        let calls = fs::read_to_string(&invoked_log).unwrap();
        let lines: Vec<&str> = calls.lines().collect();
        assert_eq!(lines, vec!["cursor sync", "capy sync"]);
    }
}
