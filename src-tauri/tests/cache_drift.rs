use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use tempfile::TempDir;
use tokscale_ui_lib::cli::{CliService, ProbeEnv};
use tokscale_ui_lib::commands::{AppState, CliServiceRunner};
use tokscale_ui_lib::contracts::{CompanionPreferences, DashboardQuery, RangePreset};
use tokscale_ui_lib::data::cache::CacheOutcome;
use tokscale_ui_lib::data::schema::EXPECTED_SCHEMA_VERSION;
use tokscale_ui_lib::data::source::{CliRunner, DataOrigin};
use tokscale_ui_lib::data::DataService;
use tokscale_ui_lib::shell::ShellState;

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("tui-data-cache.v4.23.0.json")
}

#[cfg(unix)]
fn write_exec_script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create script parent");
    }
    fs::write(path, body).expect("write script");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod +x script");
}

fn base_query(range: RangePreset) -> DashboardQuery {
    DashboardQuery {
        range,
        custom_since: None,
        custom_until: None,
        group_by: "model".to_string(),
        active_clients: Vec::new(),
        search: None,
        sort: Some("cost".to_string()),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn schema_version_plus_one_triggers_cli_fallback_and_returns_warning_without_panic() {
    let tmp = TempDir::new().expect("tempdir");
    let home = tmp.path().join("home");
    let bin_dir = tmp.path().join("bin");
    let tui_cache = tmp.path().join("tui-data-cache.json");
    let usage_cache = tmp.path().join("subscription-usage-cache.json");

    let raw = fs::read_to_string(fixture_path()).expect("read v4.23.0 fixture");
    let drifted_version = EXPECTED_SCHEMA_VERSION + 1;
    let mutated = raw.replacen(
        &format!("\"schemaVersion\": {}", EXPECTED_SCHEMA_VERSION),
        &format!("\"schemaVersion\": {drifted_version}"),
        1,
    );
    fs::write(&tui_cache, mutated).expect("write drifted cache");

    let fake_cli = bin_dir.join("tokscale");
    write_exec_script(
        &fake_cli,
        r#"#!/bin/sh
case "$1" in
  --version)
    echo "tokscale 4.24.0"
    ;;
  models)
    echo '[{"model":"claude-sonnet-4-5","provider":"anthropic","client":"claude","tokens":{"input":200,"output":40,"cacheRead":600,"cacheWrite":0,"reasoning":0},"cost":2.5,"sessionCount":3}]'
    ;;
  daily)
    echo '[{"date":"2026-11-21","tokens":{"input":200,"output":40,"cacheRead":600,"cacheWrite":0,"reasoning":0},"cost":2.5,"messageCount":8}]'
    ;;
  monthly)
    echo '[{"month":"2026-11","models":["claude-sonnet-4-5"],"tokens":{"input":200,"output":40,"cacheRead":600,"cacheWrite":0,"reasoning":0},"messageCount":8,"sessionCount":3,"activeDays":1,"cost":2.5}]'
    ;;
  clients)
    echo '[{"client":"claude","sessionsPath":"/Users/dev/.claude/projects","sessionsPathExists":true,"messageCount":8,"status":"ready"}]'
    ;;
  *)
    echo '[]'
    ;;
esac
"#,
    );

    let env = ProbeEnv::isolated(home).with_path([bin_dir, PathBuf::from("/usr/bin"), PathBuf::from("/bin")]);
    let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
    let cli = Arc::new(CliService::with_env(Arc::clone(&prefs), env));
    let runner: Arc<dyn CliRunner> = Arc::new(CliServiceRunner(Arc::clone(&cli)));
    let data = Arc::new(DataService::with_paths(runner, tui_cache, usage_cache));
    let shell_state = Arc::new(RwLock::new(ShellState::default()));
    let state = AppState::new_for_test(cli, Arc::clone(&data), prefs, shell_state);

    let report = state.data.hydrate_from_disk();
    assert!(report.fallback_required);
    assert!(matches!(
        report.cache_outcome,
        CacheOutcome::Drift { found, expected }
            if found == drifted_version && expected == EXPECTED_SCHEMA_VERSION
    ));
    assert!(report
        .warnings
        .iter()
        .any(|w| w.source == "cache" && w.message.contains("drift")));

    // Before CLI fallback finishes, snapshot does not panic and preserves the drift warning (partial state).
    let pre_fallback_snap = state.dashboard_snapshot(&base_query(RangePreset::All));
    let is_partial = !pre_fallback_snap.sync_status.warnings.is_empty();
    assert!(is_partial);
    assert!(pre_fallback_snap
        .sync_status
        .warnings
        .iter()
        .any(|w| w.source == "cache" && w.message.contains("drift")));

    // Execute CLI fallback path via CliServiceRunner.
    state
        .data
        .refresh_from_cli_fallback()
        .await
        .expect("CLI fallback should recover from schema drift");
    assert!(!state.data.fallback_required());
    assert_eq!(state.data.origin(), DataOrigin::CliFallback);

    let recovered = state.dashboard_snapshot(&base_query(RangePreset::All));
    assert_eq!(recovered.models.len(), 1);
    assert_eq!(recovered.models[0].model, "claude-sonnet-4-5");
    assert!((recovered.kpis.total_cost - 2.5).abs() < 1e-9);
    assert_eq!(recovered.sources.clients.len(), 1);
    assert_eq!(recovered.sources.clients[0].client, "claude");
}

#[tokio::test]
async fn schema_drift_with_missing_cli_keeps_drift_warning_without_panic() {
    let tmp = TempDir::new().expect("tempdir");
    let home = tmp.path().join("home");
    let tui_cache = tmp.path().join("tui-data-cache.json");
    let usage_cache = tmp.path().join("subscription-usage-cache.json");

    let raw = fs::read_to_string(fixture_path()).expect("read v4.23.0 fixture");
    let drifted_version = EXPECTED_SCHEMA_VERSION + 1;
    let mutated = raw.replacen(
        &format!("\"schemaVersion\": {}", EXPECTED_SCHEMA_VERSION),
        &format!("\"schemaVersion\": {drifted_version}"),
        1,
    );
    fs::write(&tui_cache, mutated).expect("write drifted cache");

    let env = ProbeEnv::isolated(home);
    let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
    let cli = Arc::new(CliService::with_env(Arc::clone(&prefs), env));
    let runner: Arc<dyn CliRunner> = Arc::new(CliServiceRunner(Arc::clone(&cli)));
    let data = Arc::new(DataService::with_paths(runner, tui_cache, usage_cache));
    let shell_state = Arc::new(RwLock::new(ShellState::default()));
    let state = AppState::new_for_test(cli, data, prefs, shell_state);

    let report = state.data.hydrate_from_disk();
    assert!(report.fallback_required);

    let status = state
        .sync_headless(tokscale_ui_lib::contracts::SyncMode::LocalOffline)
        .await
        .expect("offline sync should return status rather than panic");
    assert!(!status.warnings.is_empty());
    assert!(status.warnings.iter().any(|w| w.source == "cache"));

    let snap = state.dashboard_snapshot(&base_query(RangePreset::All));
    assert!(!snap.sync_status.warnings.is_empty());
}
