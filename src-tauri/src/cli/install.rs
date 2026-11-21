use crate::contracts::CliInstallResult;

use super::commands;
use super::locate::resolve_tool;
use super::CliService;

pub(crate) async fn install_or_upgrade(service: &CliService) -> CliInstallResult {
    let env = service.env();

    let (tool_label, program, args) = if let Some(bun) = resolve_tool("bun", env) {
        (
            "bun",
            bun.path,
            vec![
                "add".to_string(),
                "-g".to_string(),
                "tokscale@latest".to_string(),
            ],
        )
    } else if let Some(npm) = resolve_tool("npm", env) {
        (
            "npm",
            npm.path,
            vec![
                "i".to_string(),
                "-g".to_string(),
                "tokscale@latest".to_string(),
            ],
        )
    } else {
        return CliInstallResult {
            ok: false,
            message: "Install Bun or Node.js first".to_string(),
        };
    };

    if let Err(err) = service
        .queue()
        .run_tool(program, args, commands::TIMEOUT_LONG, env.clone())
        .await
    {
        return CliInstallResult {
            ok: false,
            message: format!("Failed to install tokscale via {tool_label}: {err}"),
        };
    }

    service.invalidate_resolution();
    let resolved = service.resolve().await;
    let message = match resolved.version {
        Some(ver) => format!("tokscale {ver} installed via {tool_label}"),
        None => format!("tokscale installed via {tool_label}"),
    };

    CliInstallResult { ok: true, message }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Arc, RwLock};
    use tempfile::TempDir;

    use super::*;
    use crate::cli::locate::{write_test_script, ProbeEnv};
    use crate::contracts::CompanionPreferences;

    #[tokio::test]
    async fn returns_documented_message_when_neither_bun_nor_npm_exists() {
        let tmp = TempDir::new().unwrap();
        let env = ProbeEnv::isolated(tmp.path().join("home"));
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(prefs, env);

        let result = service.install_or_upgrade().await;
        assert_eq!(
            result,
            CliInstallResult {
                ok: false,
                message: "Install Bun or Node.js first".to_string(),
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn picks_bun_before_npm_and_refreshes_version_cache() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let bin_dir = tmp.path().join("bin");
        let bun_bin = home.join(".bun/bin");
        let log_file = tmp.path().join("installer.log");
        let tokscale_target = bun_bin.join("tokscale");

        write_test_script(
            &bin_dir.join("bun"),
            &format!(
                "#!/bin/sh\necho \"bun:$*\" >> \"{}\"\nmkdir -p \"{}\"\ncat > \"{}\" << 'EOF'\n#!/bin/sh\necho 'tokscale 4.24.0'\nEOF\nchmod +x \"{}\"\n",
                log_file.display(),
                bun_bin.display(),
                tokscale_target.display(),
                tokscale_target.display(),
            ),
        );
        write_test_script(
            &bin_dir.join("npm"),
            &format!("#!/bin/sh\necho \"npm:$*\" >> \"{}\"\n", log_file.display()),
        );

        let env = ProbeEnv::isolated(home).with_test_path([bin_dir]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(prefs, env);

        let before = service.resolve().await;
        assert_eq!(before.resolved_via, "none");

        let res = service.install_or_upgrade().await;
        assert!(res.ok, "expected ok, got: {:?}", res);
        assert!(res.message.contains("4.24.0"));
        assert!(res.message.contains("bun"));

        let log = fs::read_to_string(&log_file).unwrap();
        assert_eq!(log.trim(), "bun:add -g tokscale@latest");

        let after = service.resolve().await;
        assert_eq!(after.resolved_via, "probe");
        assert_eq!(after.version.as_deref(), Some("4.24.0"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn falls_back_to_npm_when_bun_is_missing() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let bin_dir = tmp.path().join("bin");
        let log_file = tmp.path().join("npm.log");
        let tokscale_target = bin_dir.join("tokscale");

        write_test_script(
            &bin_dir.join("npm"),
            &format!(
                "#!/bin/sh\necho \"npm:$*\" >> \"{}\"\ncat > \"{}\" << 'EOF'\n#!/bin/sh\necho '4.23.1'\nEOF\nchmod +x \"{}\"\n",
                log_file.display(),
                tokscale_target.display(),
                tokscale_target.display(),
            ),
        );

        let env = ProbeEnv::isolated(home).with_test_path([bin_dir]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(prefs, env);

        let res = service.install_or_upgrade().await;
        assert!(res.ok);
        assert!(res.message.contains("4.23.1"));
        assert!(res.message.contains("npm"));

        let log = fs::read_to_string(&log_file).unwrap();
        assert_eq!(log.trim(), "npm:i -g tokscale@latest");
    }
}
