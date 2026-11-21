use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use tempfile::TempDir;
use tokscale_ui_lib::cli::{CliService, ProbeEnv};
use tokscale_ui_lib::commands::{AppState, CliServiceRunner};
use tokscale_ui_lib::contracts::{
    events, CompanionPreferences, DashboardNavigateEvent, DashboardQuery, DashboardTab, RangePreset,
};
use tokscale_ui_lib::data::source::CliRunner;
use tokscale_ui_lib::data::DataService;
use tokscale_ui_lib::shell::dashboard::{peek_pending_handoff, OpenAction};
use tokscale_ui_lib::shell::ShellState;

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("tui-data-cache.v4.23.0.json")
}

fn default_query() -> DashboardQuery {
    DashboardQuery {
        range: RangePreset::ThirtyDays,
        custom_since: None,
        custom_until: None,
        group_by: "model".to_string(),
        active_clients: Vec::new(),
        search: None,
        sort: Some("cost".to_string()),
    }
}

#[test]
fn cold_open_attaches_and_consumes_pending_handoff_and_warm_open_plans_navigate_emit() {
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
        dashboard_open: false,
        ..ShellState::default()
    }));
    let state = AppState::new_for_test(cli, data, prefs, shell_state);

    let report = state.data.hydrate_from_disk();
    assert!(!report.fallback_required);

    // Case A: Cold open (dashboard closed) stores pending_handoff and first snapshot consumes it.
    let cold_handoff = DashboardNavigateEvent {
        tab: DashboardTab::Models,
        range: Some(RangePreset::SevenDays),
        query: Some("sonnet".to_string()),
    };

    let cold_action = state.open_dashboard_headless(cold_handoff.clone());
    assert_eq!(cold_action, OpenAction::ColdCreate);
    assert_eq!(
        peek_pending_handoff(&state.shell_state),
        Some(cold_handoff.clone())
    );

    let first_snapshot = state.dashboard_snapshot(&default_query());
    assert_eq!(first_snapshot.pending_handoff, Some(cold_handoff));
    assert_eq!(peek_pending_handoff(&state.shell_state), None);

    // Subsequent snapshot fetches must not re-deliver the consumed handoff.
    let second_snapshot = state.dashboard_snapshot(&default_query());
    assert_eq!(second_snapshot.pending_handoff, None);

    // Case B: Warm open (dashboard already open) selects WarmEmit for tokscale://dashboard-navigate.
    let warm_handoff = DashboardNavigateEvent {
        tab: DashboardTab::Sessions,
        range: Some(RangePreset::SevenDays),
        query: Some("tokscale".to_string()),
    };
    let warm_action = state.open_dashboard_headless(warm_handoff.clone());
    assert_eq!(warm_action, OpenAction::WarmEmit(warm_handoff.clone()));
    assert_eq!(peek_pending_handoff(&state.shell_state), None);

    assert_eq!(events::DASHBOARD_NAVIGATE, "tokscale://dashboard-navigate");
    let json = serde_json::to_value(&warm_handoff).expect("serialize navigate event");
    assert_eq!(json["tab"], "sessions");
    assert_eq!(json["range"], "7d");
    assert_eq!(json["query"], "tokscale");
}
