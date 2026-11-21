use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use tempfile::TempDir;
use tokscale_ui_lib::cli::{CliService, ProbeEnv};
use tokscale_ui_lib::commands::{AppState, CliServiceRunner};
use tokscale_ui_lib::contracts::{
    CompanionPreferences, DashboardNavigateEvent, DashboardQuery, DashboardTab, RangePreset,
};
use tokscale_ui_lib::data::source::CliRunner;
use tokscale_ui_lib::data::DataService;
use tokscale_ui_lib::shell::dashboard::record_dashboard_close;
use tokscale_ui_lib::shell::{DockPolicy, ShellState};

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("tui-data-cache.v4.23.0.json")
}

fn all_query() -> DashboardQuery {
    DashboardQuery {
        range: RangePreset::All,
        custom_since: None,
        custom_until: None,
        group_by: "model".to_string(),
        active_clients: Vec::new(),
        search: None,
        sort: Some("cost".to_string()),
    }
}

#[test]
fn close_requested_drops_dashboard_keeps_tray_alive_and_reopen_serves_from_memory() {
    let tmp = TempDir::new().expect("tempdir");
    let tui_cache = tmp.path().join("tui-data-cache.json");
    let usage_cache = tmp.path().join("subscription-usage-cache.json");
    std::fs::copy(fixture_path(), &tui_cache).expect("copy fixture");

    let env = ProbeEnv::isolated(tmp.path().join("home"));
    let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
    let cli = Arc::new(CliService::with_env(Arc::clone(&prefs), env));
    let runner: Arc<dyn CliRunner> = Arc::new(CliServiceRunner(Arc::clone(&cli)));
    let data = Arc::new(DataService::with_paths(runner, tui_cache, usage_cache));
    let shell_state = Arc::new(RwLock::new(ShellState {
        tray_alive: true,
        ..ShellState::default()
    }));
    let state = AppState::new_for_test(cli, Arc::clone(&data), prefs, Arc::clone(&shell_state));

    let report = state.data.hydrate_from_disk();
    assert!(!report.fallback_required);
    assert_eq!(state.data.disk_reads(), 1);

    // Open dashboard for the first time.
    state.open_dashboard_headless(DashboardNavigateEvent {
        tab: DashboardTab::Overview,
        range: Some(RangePreset::All),
        query: None,
    });
    {
        let guard = state.shell_state.read().expect("shell_state read");
        assert!(guard.dashboard_open);
        assert_eq!(guard.dock_policy, DockPolicy::Regular);
        assert!(guard.tray_alive);
    }

    let first_snap = state.dashboard_snapshot(&all_query());
    assert!((first_snap.kpis.total_cost - 36.80).abs() < 1e-6);
    assert_eq!(state.data.disk_reads(), 1);

    // Simulate WindowEvent::CloseRequested -> record_dashboard_close.
    record_dashboard_close(&state.shell_state);
    {
        let guard = state.shell_state.read().expect("shell_state read");
        assert!(!guard.dashboard_open);
        assert_eq!(guard.dock_policy, DockPolicy::Accessory);
        assert!(guard.tray_alive);
        assert!(guard.pending_handoff.is_none());
    }

    // Reopen dashboard and verify snapshot is served strictly from memory (no extra disk reads).
    state.open_dashboard_headless(DashboardNavigateEvent {
        tab: DashboardTab::Models,
        range: Some(RangePreset::All),
        query: None,
    });
    {
        let guard = state.shell_state.read().expect("shell_state read");
        assert!(guard.dashboard_open);
        assert_eq!(guard.dock_policy, DockPolicy::Regular);
        assert!(guard.tray_alive);
    }

    let reopened_snap = state.dashboard_snapshot(&all_query());
    assert!((reopened_snap.kpis.total_cost - 36.80).abs() < 1e-6);
    assert_eq!(reopened_snap.models.len(), 3);
    assert_eq!(
        state.data.disk_reads(),
        1,
        "reopening dashboard must serve cached state from memory without extra disk reads"
    );
}
