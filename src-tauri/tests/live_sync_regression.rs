#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use chrono::NaiveDate;
use tempfile::TempDir;
use tokscale_ui_lib::cli::{CliService, ProbeEnv};
use tokscale_ui_lib::commands::{AppState, CliServiceRunner};
use tokscale_ui_lib::contracts::{CompanionPreferences, DashboardQuery, RangePreset, SyncMode};
use tokscale_ui_lib::data::schema::EXPECTED_SCHEMA_VERSION;
use tokscale_ui_lib::data::source::CliRunner;
use tokscale_ui_lib::data::DataService;
use tokscale_ui_lib::shell::ShellState;

#[cfg(unix)]
fn write_exec_script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create script parent");
    }
    fs::write(path, body).expect("write script");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod +x script");
}

fn fixed_today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 21).expect("valid date")
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
async fn live_sync_recovers_today_when_tui_cache_ends_yesterday_and_survives_rehydrate() {
    let tmp = TempDir::new().expect("tempdir");
    let home = tmp.path().join("home");
    let bin_dir = tmp.path().join("bin");
    let tui_cache = tmp.path().join("tui-data-cache.json");
    let usage_cache = tmp.path().join("subscription-usage-cache.json");

    // Stale TUI cache ending yesterday (2026-11-20) with schemaVersion 12 and agent rows.
    let stale_cache = serde_json::json!({
        "schemaVersion": EXPECTED_SCHEMA_VERSION,
        "parserGeneration": 3431528046474800600_u64,
        "timestamp": 1795100000_u64,
        "enabledClients": ["opencode", "claude"],
        "groupBy": "model",
        "reportScope": { "since": null, "until": null, "year": null },
        "data": {
            "models": [
                {
                    "model": "claude-sonnet-4-5",
                    "colorKey": "claude-sonnet-4-5",
                    "provider": "anthropic",
                    "client": "opencode",
                    "tokens": { "input": 100000, "output": 20000, "cacheRead": 300000, "cacheWrite": 0, "reasoning": 0 },
                    "cost": 7.85,
                    "sessionCount": 2,
                    "groupKey": "claude-sonnet-4-5"
                }
            ],
            "agents": [
                {
                    "agent": "Build",
                    "clients": "opencode",
                    "tokens": { "input": 100000, "output": 20000, "cacheRead": 300000, "cacheWrite": 0, "reasoning": 0 },
                    "cost": 7.85,
                    "messageCount": 14
                }
            ],
            "daily": [
                {
                    "date": "2026-11-20",
                    "tokens": { "input": 100000, "output": 20000, "cacheRead": 300000, "cacheWrite": 0, "reasoning": 0 },
                    "cost": 7.85,
                    "models": [],
                    "sourceBreakdown": [
                        [
                            "opencode",
                            {
                                "tokens": { "input": 100000, "output": 20000, "cacheRead": 300000, "cacheWrite": 0, "reasoning": 0 },
                                "cost": 7.85,
                                "models": [
                                    [
                                        "claude-sonnet-4-5",
                                        {
                                            "client": "opencode",
                                            "provider": "anthropic",
                                            "displayName": "claude-sonnet-4-5",
                                            "colorKey": "claude-sonnet-4-5",
                                            "tokens": { "input": 100000, "output": 20000, "cacheRead": 300000, "cacheWrite": 0, "reasoning": 0 },
                                            "cost": 7.85,
                                            "messages": 14
                                        }
                                    ]
                                ]
                            }
                        ]
                    ],
                    "messageCount": 14,
                    "turnCount": 7
                }
            ],
            "hourly": [
                {
                    "datetime": "2026-11-20 18:00:00",
                    "tokens": { "input": 100000, "output": 20000, "cacheRead": 300000, "cacheWrite": 0, "reasoning": 0 },
                    "cost": 7.85,
                    "clients": ["opencode"],
                    "models": [
                        [
                            "claude-sonnet-4-5",
                            {
                                "provider": "anthropic",
                                "displayName": "claude-sonnet-4-5",
                                "colorKey": "claude-sonnet-4-5",
                                "tokens": { "input": 100000, "output": 20000, "cacheRead": 300000, "cacheWrite": 0, "reasoning": 0 },
                                "cost": 7.85
                            }
                        ]
                    ],
                    "messageCount": 14,
                    "turnCount": 7
                }
            ],
            "graph": { "weeks": [] },
            "totalTokens": 420000,
            "undatedTokens": 0,
            "totalCost": 7.85,
            "currentStreak": 1,
            "longestStreak": 1
        }
    });
    fs::write(&tui_cache, serde_json::to_vec_pretty(&stale_cache).unwrap())
        .expect("write stale tui cache");

    // Realistic fake tokscale v4.23.0 binary: `daily` does not exist and exits 2,
    // while `graph` and `hourly` return v4.23.0 JSON including 2026-11-21 ($37.36).
    let fake_cli = bin_dir.join("tokscale");
    write_exec_script(
        &fake_cli,
        r#"#!/bin/sh
case "$1" in
  --version)
    echo "tokscale 4.23.0"
    ;;
  daily)
    echo "error: unrecognized subcommand 'daily'" >&2
    exit 2
    ;;
  graph)
    cat <<'EOF'
{
  "meta": { "version": "4.23.0" },
  "summary": { "totalCost": 45.21, "totalTokens": 1825000 },
  "years": [2026],
  "contributions": [
    {
      "date": "2026-11-20",
      "totals": { "tokens": 420000, "cost": 7.85, "messages": 14 },
      "intensity": 1,
      "tokenBreakdown": { "input": 100000, "output": 20000, "cacheRead": 300000, "cacheWrite": 0, "reasoning": 0 },
      "clients": [
        {
          "client": "opencode",
          "modelId": "claude-sonnet-4-5",
          "providerId": "anthropic",
          "tokens": { "input": 100000, "output": 20000, "cacheRead": 300000, "cacheWrite": 0, "reasoning": 0 },
          "cost": 7.85,
          "messages": 14
        }
      ]
    },
    {
      "date": "2026-11-21",
      "totals": { "tokens": 1405000, "cost": 37.36, "messages": 32 },
      "intensity": 4,
      "tokenBreakdown": { "input": 400000, "output": 70000, "cacheRead": 900000, "cacheWrite": 25000, "reasoning": 10000 },
      "clients": [
        {
          "client": "opencode",
          "modelId": "claude-sonnet-4-5",
          "providerId": "anthropic",
          "tokens": { "input": 400000, "output": 70000, "cacheRead": 900000, "cacheWrite": 25000, "reasoning": 10000 },
          "cost": 37.36,
          "messages": 32
        }
      ]
    }
  ]
}
EOF
    ;;
  hourly)
    cat <<'EOF'
{
  "entries": [
    {
      "hour": "2026-11-21 22:00",
      "clients": ["opencode"],
      "models": ["claude-sonnet-4-5"],
      "input": 400000,
      "output": 70000,
      "cacheRead": 900000,
      "cacheWrite": 25000,
      "messageCount": 32,
      "turnCount": 16,
      "cost": 37.36
    }
  ],
  "totalCost": 37.36,
  "processingTimeMs": 42
}
EOF
    ;;
  models)
    echo '[{"model":"claude-sonnet-4-5","provider":"anthropic","client":"opencode","tokens":{"input":500000,"output":90000,"cacheRead":1200000,"cacheWrite":25000,"reasoning":10000},"cost":45.21,"sessionCount":4}]'
    ;;
  monthly)
    echo '[{"month":"2026-11","models":["claude-sonnet-4-5"],"tokens":{"input":500000,"output":90000,"cacheRead":1200000,"cacheWrite":25000,"reasoning":10000},"messageCount":46,"sessionCount":4,"activeDays":2,"cost":45.21}]'
    ;;
  clients)
    echo '[{"client":"opencode","sessionsPath":"/Users/dev/.local/share/opencode/storage","sessionsPathExists":true,"messageCount":46,"status":"ready"}]'
    ;;
  usage)
    echo '[{"provider":"Copilot","plan":"Individual","metrics":[{"label":"Chat","remainingLabel":"90% left","remainingPercent":90.0,"usedPercent":10.0,"resetsAt":"2026-12-01"}]}]'
    ;;
  *)
    echo '[]'
    ;;
esac
"#,
    );

    let env = ProbeEnv::isolated(home).with_path([
        bin_dir,
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ]);
    let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
    let cli = Arc::new(CliService::with_env(Arc::clone(&prefs), env));
    let runner: Arc<dyn CliRunner> = Arc::new(CliServiceRunner(Arc::clone(&cli)));
    let data = Arc::new(DataService::with_paths(runner, tui_cache, usage_cache));
    let shell_state = Arc::new(RwLock::new(ShellState::default()));
    let state = AppState::new_for_test(cli, Arc::clone(&data), prefs, shell_state);

    let hydrate_report = state.data.hydrate_from_disk();
    assert!(!hydrate_report.fallback_required);

    // Before sync, Today is $0.00 because tui-data-cache.json ended yesterday.
    let pre_sync = state.data.tray_snapshot_at(RangePreset::Today, fixed_today());
    assert!((pre_sync.total.cost - 0.0).abs() < 1e-9);

    // Run LocalOffline sync: must populate Today via `graph` + `hourly` even though `daily` exits 2.
    let sync_status = state
        .sync_headless(SyncMode::LocalOffline)
        .await
        .expect("LocalOffline sync succeeds");
    assert!(!sync_status.syncing, "syncing flag must be false after completion");

    let today_snap = state.data.tray_snapshot_at(RangePreset::Today, fixed_today());
    assert!(
        today_snap.total.cost > 37.0,
        "expected Today cost > 37.0 after sync, got {}",
        today_snap.total.cost
    );
    assert!(
        !today_snap.bars.is_empty(),
        "expected non-empty Today bars"
    );
    assert!(
        today_snap.bars.iter().any(|b| b.slot == "22:00" && b.total_cost > 37.0),
        "expected 22:00 hourly bar with cost > 37.0, got {:?}",
        today_snap.bars
    );
    assert!(
        !today_snap.top_models.is_empty(),
        "expected non-empty top_models for Today"
    );
    assert!(
        !today_snap.sync_status.syncing,
        "tray snapshot sync_status.syncing must be false"
    );

    // Re-hydrating from older tui-data-cache.json on disk must NOT wipe out fresher CLI-synced Today data.
    state.data.hydrate_from_disk();
    let after_rehydrate = state.data.tray_snapshot_at(RangePreset::Today, fixed_today());
    assert!(
        after_rehydrate.total.cost > 37.0,
        "hydrate_from_disk must not clobber fresher CLI-synced Today data, got {}",
        after_rehydrate.total.cost
    );
    assert!(
        !after_rehydrate.bars.is_empty(),
        "bars must survive hydrate_from_disk"
    );
    assert!(
        !after_rehydrate.top_models.is_empty(),
        "top_models must survive hydrate_from_disk"
    );

    // Agents from tui-data-cache.json must remain intact after CLI fallback merge,
    // and rebuilt graph / streaks must reflect today's activity.
    let dash_all = state
        .data
        .dashboard_snapshot_at(&base_query(RangePreset::All), fixed_today());
    assert_eq!(dash_all.agents.len(), 1);
    assert_eq!(dash_all.agents[0].agent, "Build");
    assert!(!dash_all.graph.weeks.is_empty());
    assert_eq!(dash_all.stats.current_streak, 2);
    assert_eq!(dash_all.stats.longest_streak, 2);

    // FullManual sync must return syncing: false and populate usage from `usage --json`.
    let manual_status = state
        .sync_headless(SyncMode::FullManual)
        .await
        .expect("FullManual sync succeeds");
    assert!(!manual_status.syncing);
    let dash_after_manual = state
        .data
        .dashboard_snapshot_at(&base_query(RangePreset::Today), fixed_today());
    assert_eq!(dash_after_manual.usage.len(), 1);
    assert_eq!(dash_after_manual.usage[0].provider, "Copilot");
}

#[cfg(unix)]
#[tokio::test]
async fn sync_locking_and_cancellation_guard_behave_correctly() {
    use std::time::Duration;

    let tmp = TempDir::new().expect("tempdir");
    let home = tmp.path().join("home");
    let bin_dir = tmp.path().join("bin");
    let tui_cache = tmp.path().join("tui-data-cache.json");
    let usage_cache = tmp.path().join("subscription-usage-cache.json");
    let fake_cli = bin_dir.join("tokscale");
    let usage_calls = tmp.path().join("usage-calls.log");

    write_exec_script(
        &fake_cli,
        &format!(
            r#"#!/bin/sh
case "$1" in
  --version)
    echo "tokscale 4.23.0"
    ;;
  usage)
    echo called >> "{}"
    sleep 0.25
    echo '[{{"provider":"Copilot","plan":"Individual","metrics":[{{"label":"Chat","remainingPercent":88.0,"usedPercent":12.0}}]}}]'
    ;;
  models)
    echo '[{{"model":"claude-sonnet-4-5","provider":"anthropic","client":"opencode","tokens":{{"input":100,"output":20,"cacheRead":300,"cacheWrite":0,"reasoning":0}},"cost":1.25,"sessionCount":1}}]'
    ;;
  graph)
    echo '{{"contributions":[{{"date":"2026-11-21","totals":{{"tokens":420,"cost":1.25,"messages":3}},"intensity":4,"tokenBreakdown":{{"input":100,"output":20,"cacheRead":300,"cacheWrite":0,"reasoning":0}},"clients":[{{"client":"opencode","modelId":"claude-sonnet-4-5","providerId":"anthropic","tokens":{{"input":100,"output":20,"cacheRead":300,"cacheWrite":0,"reasoning":0}},"cost":1.25,"messages":3}}]}}]}}'
    ;;
  hourly)
    echo '{{"entries":[{{"hour":"2026-11-21 22:00","clients":["opencode"],"models":["claude-sonnet-4-5"],"input":100,"output":20,"cacheRead":300,"cacheWrite":0,"messageCount":3,"turnCount":1,"cost":1.25}}],"totalCost":1.25,"processingTimeMs":5}}'
    ;;
  *)
    echo '[]'
    ;;
esac
"#,
            usage_calls.display()
        ),
    );

    let env = ProbeEnv::isolated(home).with_path([
        bin_dir,
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ]);
    let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
    let cli = Arc::new(CliService::with_env(Arc::clone(&prefs), env));
    let runner: Arc<dyn CliRunner> = Arc::new(CliServiceRunner(Arc::clone(&cli)));
    let data = Arc::new(DataService::with_paths(runner, tui_cache, usage_cache));
    let shell_state = Arc::new(RwLock::new(ShellState::default()));
    let state = Arc::new(AppState::new_for_test(cli, data, prefs, shell_state));

    // 1. LocalOffline skips without blocking when sync_inflight is already held.
    {
        let _held = state.sync_inflight.lock().await;
        let skipped = tokio::time::timeout(
            Duration::from_millis(50),
            state.sync_headless(SyncMode::LocalOffline),
        )
        .await
        .expect("LocalOffline must not block when sync_inflight is held")
        .expect("LocalOffline returns current status");
        assert!(!skipped.syncing);
    }

    // 2. FullManual awaits sync_inflight release and completes with syncing == false.
    let held_for_manual = state.sync_inflight.clone().lock_owned().await;
    let state_for_task = Arc::clone(&state);
    let release_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(80)).await;
        drop(held_for_manual);
    });
    let manual_status = state_for_task
        .sync_headless(SyncMode::FullManual)
        .await
        .expect("FullManual waits for lock and completes");
    release_handle.await.expect("release task");
    assert!(!manual_status.syncing);
    assert_eq!(
        state
            .data
            .dashboard_snapshot_at(&base_query(RangePreset::Today), fixed_today())
            .usage
            .len(),
        1
    );

    // 3. Dropping/cancelling an in-flight sync resets syncing to false via RAII Drop guard.
    let cancel_res = tokio::time::timeout(
        Duration::from_millis(50),
        state.sync_headless(SyncMode::FullManual),
    )
    .await;
    assert!(cancel_res.is_err(), "expected timeout while usage sleeps 0.25s");
    assert!(
        !state.sync_status.read().unwrap().syncing,
        "sync_status.syncing must be reset to false on cancellation"
    );
    assert!(
        !state.tray_snapshot(RangePreset::Today).sync_status.syncing,
        "tray_snapshot sync_status.syncing must be reset to false on cancellation"
    );
}

