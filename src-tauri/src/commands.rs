use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager, State, Wry};

use crate::cli::{commands as cli_commands, config as cli_config, CliService, ProbeEnv, PROVIDER_SYNC};
use crate::contracts::{
    events, AutosubmitConfigRequest, CliActionResult, CliInstallResult, CliResolutionInfo,
    CompanionPreferences, CustomPricingEntry, CustomPricingState, DashboardNavigateEvent,
    DashboardQuery, DashboardSnapshot, DashboardSources, ModelPriceLookupResult,
    ProviderActionRequest, ProvidersStatusPayload, RangePreset, SessionDetail,
    SessionTitlesUpdatedEvent, SessionsPayload, SessionsQuery, SettingsSection, SnapshotEvent,
    SocialActionRequest, SocialStatusPayload, SourceWarning, SyncMode, SyncProgressEvent,
    SyncStatus, TokscaleSettingsPatch, TokscaleSettingsState, TraySnapshot,
};
use crate::data::source::CliRunner;
use crate::data::DataService;
use crate::shell::dashboard::{plan_dashboard_open, take_pending_handoff, OpenAction};
use crate::shell::prefs::{load_preferences, patch_preferences};
use crate::shell::{ShellController, ShellHooks, ShellState, TraySnapshotLite};

pub struct CliServiceRunner(pub Arc<CliService>);

#[async_trait::async_trait]
impl CliRunner for CliServiceRunner {
    async fn run_json(&self, args: &[String]) -> Result<serde_json::Value, String> {
        let timeout = match args.first().map(String::as_str) {
            Some("usage") | Some("--version") => cli_commands::TIMEOUT_SHORT,
            _ if args.get(1).map(String::as_str) == Some("sync")
                || args.first().map(String::as_str) == Some("sync") =>
            {
                cli_commands::TIMEOUT_LONG
            }
            _ => cli_commands::TIMEOUT_DATA,
        };

        self.0
            .run_json::<serde_json::Value>(args.to_vec(), timeout)
            .await
            .map_err(|err| err.to_string())
    }

    async fn cli_info(&self) -> CliResolutionInfo {
        self.0.resolve().await
    }
}

pub struct AppState {
    pub cli: Arc<CliService>,
    pub data: Arc<DataService>,
    pub prefs: Arc<RwLock<CompanionPreferences>>,
    pub shell_state: Arc<RwLock<ShellState>>,
    pub shell: RwLock<Option<ShellController>>,
    pub sync_status: Arc<RwLock<SyncStatus>>,
    pub sync_inflight: Arc<tokio::sync::Mutex<()>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let cli = Arc::new(CliService::with_env(
            Arc::clone(&prefs),
            ProbeEnv::from_process(),
        ));
        let runner: Arc<dyn CliRunner> = Arc::new(CliServiceRunner(Arc::clone(&cli)));
        let data = Arc::new(DataService::new(runner));
        let shell_state = Arc::new(RwLock::new(ShellState::default()));

        Self::new_for_test(cli, data, prefs, shell_state)
    }

    pub fn new_for_test(
        cli: Arc<CliService>,
        data: Arc<DataService>,
        prefs: Arc<RwLock<CompanionPreferences>>,
        shell_state: Arc<RwLock<ShellState>>,
    ) -> Self {
        Self {
            cli,
            data,
            prefs,
            shell_state,
            shell: RwLock::new(None),
            sync_status: Arc::new(RwLock::new(SyncStatus::default())),
            sync_inflight: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub fn tray_snapshot(&self, range: RangePreset) -> TraySnapshot {
        let mut snapshot = self.data.tray_snapshot(range);
        self.merge_sync_status(&mut snapshot.sync_status);
        snapshot
    }

    pub fn dashboard_snapshot(&self, query: &DashboardQuery) -> DashboardSnapshot {
        let shell_pending = take_pending_handoff(&self.shell_state);
        let data_pending = self.data.take_pending_handoff();
        let pending = shell_pending.or(data_pending);

        let mut snapshot = self.data.dashboard_snapshot(query);
        self.merge_sync_status(&mut snapshot.sync_status);
        snapshot.pending_handoff = pending;
        snapshot
    }

    fn merge_sync_status(&self, target: &mut SyncStatus) {
        let status = self
            .sync_status
            .read()
            .unwrap_or_else(|e| e.into_inner());
        target.syncing = target.syncing || status.syncing;
        if let Some(ts) = status.last_synced_at {
            target.last_synced_at = Some(target.last_synced_at.map_or(ts, |prev| prev.max(ts)));
        }
        for w in &status.warnings {
            if !target.warnings.contains(w) {
                target.warnings.push(w.clone());
            }
        }
    }

    pub fn open_dashboard_headless(&self, handoff: DashboardNavigateEvent) -> OpenAction {
        let action = plan_dashboard_open(&self.shell_state, handoff);
        if matches!(action, OpenAction::ColdCreate) {
            if let Some(pending) = crate::shell::dashboard::peek_pending_handoff(&self.shell_state)
            {
                self.data.set_pending_handoff(Some(pending));
            }
        }
        action
    }

    pub fn get_preferences(&self) -> CompanionPreferences {
        if let Some(ctrl) = self
            .shell
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .cloned()
        {
            return ctrl.get_preferences();
        }
        self.prefs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn update_preferences(&self, patch: serde_json::Value) -> Result<CompanionPreferences, String> {
        let controller = self
            .shell
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        let previous_display = self
            .prefs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .menubar_display
            .clone();

        let updated = if let Some(ref ctrl) = controller {
            ctrl.set_preferences(patch).map_err(|err| err.to_string())?
        } else {
            let config_dir = self
                .shell_state
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .config_dir
                .clone();

            if let Some(dir) = config_dir {
                let next = patch_preferences(&dir, &patch).map_err(|err| err.to_string())?;
                self.shell_state
                    .write()
                    .unwrap_or_else(|e| e.into_inner())
                    .preferences = next.clone();
                next
            } else {
                merge_preferences_in_memory(&self.prefs, &self.shell_state, &patch)?
            }
        };

        *self.prefs.write().unwrap_or_else(|e| e.into_inner()) = updated.clone();
        self.cli.invalidate_resolution();

        if previous_display != updated.menubar_display {
            if let Some(ctrl) = controller {
                let active_range = self
                    .shell_state
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .active_range;
                let tray = self.data.tray_snapshot(active_range);
                ctrl.update_readout(&TraySnapshotLite::from(&tray));
            }
        }

        Ok(updated)
    }

    pub async fn sync_headless(&self, mode: SyncMode) -> Result<SyncStatus, String> {
        self.execute_sync_internal(mode, None).await
    }

    async fn execute_sync_internal(
        &self,
        mode: SyncMode,
        app: Option<&AppHandle<Wry>>,
    ) -> Result<SyncStatus, String> {
        let _guard = match &mode {
            SyncMode::LocalOffline => match self.sync_inflight.try_lock() {
                Ok(guard) => guard,
                Err(_) => {
                    return Ok(self
                        .sync_status
                        .read()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone());
                }
            },
            SyncMode::FullManual | SyncMode::RetryClient(_) => self.sync_inflight.lock().await,
        };

        struct SyncStateGuard {
            data: Arc<DataService>,
            sync_status: Arc<RwLock<SyncStatus>>,
            armed: bool,
        }

        impl SyncStateGuard {
            fn disarm(&mut self) {
                self.armed = false;
            }
        }

        impl Drop for SyncStateGuard {
            fn drop(&mut self) {
                if self.armed {
                    self.data.set_syncing(false);
                    let mut status = self
                        .sync_status
                        .write()
                        .unwrap_or_else(|e| e.into_inner());
                    status.syncing = false;
                }
            }
        }

        self.data.set_syncing(true);
        {
            let mut status = self
                .sync_status
                .write()
                .unwrap_or_else(|e| e.into_inner());
            status.syncing = true;
        }
        let mut sync_guard = SyncStateGuard {
            data: Arc::clone(&self.data),
            sync_status: Arc::clone(&self.sync_status),
            armed: true,
        };

        let mut warnings: Vec<SourceWarning> = Vec::new();
        let mut any_success = false;

        match mode {
            SyncMode::LocalOffline => {
                let total_sources = 2_u32;
                emit_sync_progress(
                    app,
                    SyncProgressEvent {
                        syncing: true,
                        completed_sources: 0,
                        total_sources,
                        message: "Refreshing local usage…".to_string(),
                    },
                );

                let hydrate = self.data.hydrate_from_disk();
                if !hydrate.fallback_required {
                    any_success = true;
                } else {
                    warnings.extend(hydrate.warnings);
                }

                emit_sync_progress(
                    app,
                    SyncProgressEvent {
                        syncing: true,
                        completed_sources: 1,
                        total_sources,
                        message: "Rescanning local CLI logs…".to_string(),
                    },
                );

                match self.data.refresh_from_cli_fallback().await {
                    Ok(()) => {
                        any_success = true;
                        warnings.retain(|w| w.source != "cache");
                    }
                    Err(err) => {
                        push_unique_warning(
                            &mut warnings,
                            SourceWarning {
                                source: "cli".to_string(),
                                message: err,
                            },
                        );
                    }
                }

                let _ = self.data.refresh_cli_info().await;

                emit_sync_progress(
                    app,
                    SyncProgressEvent {
                        syncing: false,
                        completed_sources: total_sources,
                        total_sources,
                        message: "Local refresh complete".to_string(),
                    },
                );
            }
            SyncMode::FullManual => {
                let credentialed: Vec<&str> = PROVIDER_SYNC
                    .iter()
                    .copied()
                    .filter(|provider| self.cli.is_provider_credentialed(provider))
                    .collect();
                let total_sources = (credentialed.len() as u32) + 3;
                let mut completed_sources = 0_u32;

                emit_sync_progress(
                    app,
                    SyncProgressEvent {
                        syncing: true,
                        completed_sources,
                        total_sources,
                        message: "Starting sync…".to_string(),
                    },
                );

                if credentialed.is_empty() {
                    let sync_warnings = self.cli.sync_providers().await;
                    for w in sync_warnings {
                        push_unique_warning(&mut warnings, w);
                    }
                } else {
                    for provider in credentialed {
                        emit_sync_progress(
                            app,
                            SyncProgressEvent {
                                syncing: true,
                                completed_sources,
                                total_sources,
                                message: format!("Syncing {provider}…"),
                            },
                        );
                        match self.cli.sync_single_provider(provider).await {
                            Ok(()) => {
                                any_success = true;
                            }
                            Err(w) => {
                                push_unique_warning(&mut warnings, w);
                            }
                        }
                        completed_sources += 1;
                    }
                }

                emit_sync_progress(
                    app,
                    SyncProgressEvent {
                        syncing: true,
                        completed_sources,
                        total_sources,
                        message: "Refreshing subscription quotas…".to_string(),
                    },
                );
                match self
                    .cli
                    .run_json::<serde_json::Value>(
                        cli_commands::usage(),
                        cli_commands::TIMEOUT_SHORT,
                    )
                    .await
                {
                    Ok(raw_usage) => {
                        if let Err(err) = self.data.update_usage_json(&raw_usage) {
                            push_unique_warning(
                                &mut warnings,
                                SourceWarning {
                                    source: "usage".to_string(),
                                    message: err,
                                },
                            );
                        } else {
                            any_success = true;
                        }
                    }
                    Err(err) => {
                        push_unique_warning(
                            &mut warnings,
                            SourceWarning {
                                source: "usage".to_string(),
                                message: err.to_string(),
                            },
                        );
                    }
                }
                completed_sources += 1;

                emit_sync_progress(
                    app,
                    SyncProgressEvent {
                        syncing: true,
                        completed_sources,
                        total_sources,
                        message: "Scanning local usage logs…".to_string(),
                    },
                );
                let hydrate = self.data.hydrate_from_disk();
                if !hydrate.fallback_required {
                    any_success = true;
                }
                match self.data.refresh_from_cli_fallback().await {
                    Ok(()) => {
                        any_success = true;
                    }
                    Err(err) => {
                        if hydrate.fallback_required {
                            warnings.extend(hydrate.warnings);
                        }
                        push_unique_warning(
                            &mut warnings,
                            SourceWarning {
                                source: "cli".to_string(),
                                message: err,
                            },
                        );
                    }
                }
                completed_sources += 1;

                emit_sync_progress(
                    app,
                    SyncProgressEvent {
                        syncing: true,
                        completed_sources,
                        total_sources,
                        message: "Inspecting client sources…".to_string(),
                    },
                );
                if self.data.refresh_clients().await.is_ok() {
                    any_success = true;
                }
                let _ = self.data.refresh_cli_info().await;
                completed_sources = total_sources;

                emit_sync_progress(
                    app,
                    SyncProgressEvent {
                        syncing: false,
                        completed_sources,
                        total_sources,
                        message: "Sync complete".to_string(),
                    },
                );
            }
            SyncMode::RetryClient(client) => {
                let total_sources = 2_u32;
                emit_sync_progress(
                    app,
                    SyncProgressEvent {
                        syncing: true,
                        completed_sources: 0,
                        total_sources,
                        message: format!("Retrying {client}…"),
                    },
                );

                if self.cli.is_api_backed_provider(&client) {
                    match self.cli.sync_single_provider(&client).await {
                        Ok(()) => {
                            any_success = true;
                        }
                        Err(w) => {
                            push_unique_warning(&mut warnings, w);
                        }
                    }
                }

                emit_sync_progress(
                    app,
                    SyncProgressEvent {
                        syncing: true,
                        completed_sources: 1,
                        total_sources,
                        message: "Refreshing local snapshot…".to_string(),
                    },
                );

                let hydrate = self.data.hydrate_from_disk();
                if !hydrate.fallback_required {
                    any_success = true;
                }
                match self.data.refresh_from_cli_fallback().await {
                    Ok(()) => {
                        any_success = true;
                    }
                    Err(err) => {
                        push_unique_warning(
                            &mut warnings,
                            SourceWarning {
                                source: client,
                                message: err,
                            },
                        );
                    }
                }
                let _ = self.data.refresh_cli_info().await;

                emit_sync_progress(
                    app,
                    SyncProgressEvent {
                        syncing: false,
                        completed_sources: total_sources,
                        total_sources,
                        message: "Retry complete".to_string(),
                    },
                );
            }
        }

        let now = current_unix_secs();
        self.data.set_syncing(false);

        let final_status = {
            let mut status = self
                .sync_status
                .write()
                .unwrap_or_else(|e| e.into_inner());
            status.syncing = false;
            if any_success {
                status.last_synced_at = Some(now);
            }
            status.warnings = warnings.clone();
            status.clone()
        };
        sync_guard.disarm();

        update_tray_and_notify(self);

        if let Some(app_handle) = app {
            let _ = app_handle.emit(
                events::SNAPSHOT_UPDATED,
                SnapshotEvent {
                    timestamp: now,
                    partial: !warnings.is_empty(),
                    warnings,
                },
            );
        }

        Ok(final_status)
    }
}

pub fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();
    let state = app.state::<AppState>();

    // Step 1: Load preferences.json (defaults if missing)
    if let Ok(config_dir) = app.path().app_config_dir() {
        match load_preferences(&config_dir) {
            Ok(loaded) => {
                *state.prefs.write().unwrap_or_else(|e| e.into_inner()) = loaded.clone();
                let mut shell_guard = state
                    .shell_state
                    .write()
                    .unwrap_or_else(|e| e.into_inner());
                shell_guard.config_dir = Some(config_dir);
                shell_guard.preferences = loaded;
            }
            Err(err) => {
                let mut shell_guard = state
                    .shell_state
                    .write()
                    .unwrap_or_else(|e| e.into_inner());
                shell_guard.config_dir = Some(config_dir);
                shell_guard.warnings.push(SourceWarning {
                    source: "preferences".to_string(),
                    message: format!("Failed to load preferences.json, using defaults: {err}"),
                });
            }
        }
    }

    // Step 2: Hydrate from local cache (<5ms) and emit snapshot-updated
    let hydrate_report = state.data.hydrate_from_disk();
    let partial = hydrate_report.fallback_required || !hydrate_report.warnings.is_empty();
    {
        let tray = state.data.tray_snapshot(RangePreset::Today);
        let mut status = state
            .sync_status
            .write()
            .unwrap_or_else(|e| e.into_inner());
        status.warnings = hydrate_report.warnings.clone();
        status.last_synced_at = tray.sync_status.last_synced_at;
    }
    let _ = app.emit(
        events::SNAPSHOT_UPDATED,
        SnapshotEvent {
            timestamp: current_unix_secs(),
            partial,
            warnings: hydrate_report.warnings.clone(),
        },
    );

    // Step 3: Install shell (tray, popover, timers) with immediate cached readout
    let hooks = ShellHooks {
        on_sync_requested: Some(Arc::new({
            let handle = app_handle.clone();
            move |mode| {
                let h = handle.clone();
                tauri::async_runtime::spawn(async move {
                    let st = h.state::<AppState>();
                    let _ = st.execute_sync_internal(mode, Some(&h)).await;
                });
            }
        })),
        on_quota_refresh: Some(Arc::new({
            let handle = app_handle.clone();
            move || {
                let h = handle.clone();
                tauri::async_runtime::spawn(async move {
                    let st = h.state::<AppState>();
                    refresh_quota_and_notify(&h, &st).await;
                });
            }
        })),
        on_range_changed: Some(Arc::new({
            let handle = app_handle.clone();
            move |next_range| {
                let st = handle.state::<AppState>();
                let tray_snap = st.data.tray_snapshot(next_range);
                let ctrl = st
                    .shell
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if let Some(ctrl) = ctrl {
                    ctrl.update_readout(&TraySnapshotLite::from(&tray_snap));
                }
            }
        })),
        on_quit_requested: None,
    };

    let controller = ShellController::install_with_hooks(
        app,
        Arc::clone(&state.shell_state),
        hooks,
        default_watch_paths(),
    )?;
    *state.shell.write().unwrap_or_else(|e| e.into_inner()) = Some(controller.clone());

    let active_range = state
        .shell_state
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .active_range;
    let initial_tray = state.data.tray_snapshot(active_range);
    controller.update_readout(&TraySnapshotLite::from(&initial_tray));

    // Steps 4 & 5: Background CLI resolution, local offline sync when cache is missing/stale, and watcher paths
    let fallback_required = hydrate_report.fallback_required;
    let startup_handle = app_handle;
    tauri::async_runtime::spawn(async move {
        let st = startup_handle.state::<AppState>();
        let cli_info = st.data.refresh_cli_info().await;

        if cli_info.resolved_via != "none" && (fallback_required || !st.data.has_today_data()) {
            let _ = st
                .execute_sync_internal(SyncMode::LocalOffline, Some(&startup_handle))
                .await;
        } else if cli_info.resolved_via != "none" {
            let _ = st.data.refresh_clients().await;
        }

        let active_range = st
            .shell_state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .active_range;
        let tray = st.data.tray_snapshot(active_range);
        let mut watch_paths = default_watch_paths();
        for src in &tray.sources {
            if src.sessions_path_exists && !src.sessions_path.trim().is_empty() {
                watch_paths.push(expand_tilde_path(&src.sessions_path));
            }
        }
        if let Some(ctrl) = st
            .shell
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .cloned()
        {
            ctrl.set_watch_paths(watch_paths);
        };
    });

    Ok(())
}

#[tauri::command]
pub async fn get_tray_snapshot(
    state: State<'_, AppState>,
    range: RangePreset,
) -> Result<TraySnapshot, String> {
    Ok(state.tray_snapshot(range))
}

#[tauri::command]
pub async fn get_dashboard_snapshot(
    state: State<'_, AppState>,
    query: DashboardQuery,
) -> Result<DashboardSnapshot, String> {
    Ok(state.dashboard_snapshot(&query))
}

#[tauri::command]
pub async fn get_sessions_view(
    app: AppHandle,
    state: State<'_, AppState>,
    query: SessionsQuery,
) -> Result<SessionsPayload, String> {
    if !state.data.sessions_scope_covers(&query.query) {
        state
            .data
            .refresh_sessions(
                query.query.range,
                query.query.custom_since.as_deref(),
                query.query.custom_until.as_deref(),
            )
            .await?;
    }

    let payload = state.data.sessions_payload(&query);

    let enrich_enabled = {
        let prefs = state.prefs.read().unwrap_or_else(|e| e.into_inner());
        query.enrich_titles && prefs.session_title_enrichment
    };
    let bounded_enrich_range = matches!(
        query.query.range,
        RangePreset::Today | RangePreset::SevenDays
    );

    if enrich_enabled && bounded_enrich_range {
        let data = Arc::clone(&state.data);
        let app_handle = app.clone();
        let range = query.query.range;
        let custom_since = query.query.custom_since.clone();
        let custom_until = query.query.custom_until.clone();

        tauri::async_runtime::spawn(async move {
            if let Ok(updated_count) = data
                .enrich_session_titles(range, custom_since.as_deref(), custom_until.as_deref())
                .await
            {
                if updated_count > 0 {
                    let _ = app_handle.emit(
                        events::SESSION_TITLES_UPDATED,
                        SessionTitlesUpdatedEvent { updated_count },
                    );
                }
            }
        });
    }

    Ok(payload)
}

#[tauri::command]
pub async fn get_session_detail(
    state: State<'_, AppState>,
    client: String,
    session_id: String,
) -> Result<SessionDetail, String> {
    if let Some(detail) = state.data.session_detail(&client, &session_id) {
        return Ok(detail);
    }

    state
        .data
        .refresh_sessions(RangePreset::All, None, None)
        .await?;

    state
        .data
        .session_detail(&client, &session_id)
        .ok_or_else(|| format!("Session detail not found for {client}:{session_id}"))
}

#[tauri::command]
pub async fn trigger_sync(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: SyncMode,
) -> Result<SyncStatus, String> {
    state.execute_sync_internal(mode, Some(&app)).await
}

#[tauri::command]
pub async fn open_dashboard(
    state: State<'_, AppState>,
    handoff: DashboardNavigateEvent,
) -> Result<(), String> {
    let controller = state
        .shell
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();

    if let Some(ctrl) = controller {
        ctrl.dismiss_tray();
        ctrl.open_dashboard(handoff).map_err(|err| err.to_string())?;
    } else {
        state.open_dashboard_headless(handoff);
    }

    Ok(())
}

#[tauri::command]
pub async fn open_settings(
    state: State<'_, AppState>,
    section: Option<SettingsSection>,
) -> Result<(), String> {
    let target_section = section.unwrap_or(SettingsSection::App);
    let controller = state
        .shell
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();

    if let Some(ctrl) = controller {
        ctrl.dismiss_tray();
        ctrl.open_settings(target_section)
            .map_err(|err| err.to_string())?;
    } else {
        let _ = crate::shell::settings::plan_settings_open(&state.shell_state, target_section);
    }

    Ok(())
}

#[tauri::command]
pub async fn take_pending_settings_section(
    state: State<'_, AppState>,
) -> Result<Option<SettingsSection>, String> {
    let controller = state
        .shell
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if let Some(ctrl) = controller {
        Ok(ctrl.take_pending_settings_section())
    } else {
        Ok(crate::shell::settings::take_pending_settings_section(
            &state.shell_state,
        ))
    }
}

#[tauri::command]
pub async fn dismiss_tray(state: State<'_, AppState>) -> Result<(), String> {
    let controller = state
        .shell
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if let Some(ctrl) = controller {
        ctrl.dismiss_tray();
    } else {
        state
            .shell_state
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .tray_visible = false;
    }
    Ok(())
}

#[tauri::command]
pub async fn resize_tray(
    app: AppHandle,
    state: State<'_, AppState>,
    height: f64,
) -> Result<(), String> {
    crate::shell::popover::resize_popover(&app, &state.shell_state, height)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn quit_app(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let controller = state
        .shell
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if let Some(ctrl) = controller {
        ctrl.shutdown();
    }
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn get_preferences(state: State<'_, AppState>) -> Result<CompanionPreferences, String> {
    Ok(state.get_preferences())
}

#[tauri::command]
pub async fn update_preferences(
    state: State<'_, AppState>,
    patch: serde_json::Value,
) -> Result<CompanionPreferences, String> {
    state.update_preferences(patch)
}

#[tauri::command]
pub async fn get_cli_info(state: State<'_, AppState>) -> Result<CliResolutionInfo, String> {
    Ok(state.data.refresh_cli_info().await)
}

#[tauri::command]
pub async fn install_or_upgrade_cli(state: State<'_, AppState>) -> Result<CliInstallResult, String> {
    let result = state.cli.install_or_upgrade().await;
    let info = state.cli.resolve().await;
    state.data.set_cli_info(info);
    Ok(result)
}

#[tauri::command]
pub async fn export_json(
    app: AppHandle,
    state: State<'_, AppState>,
    query: DashboardQuery,
) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;

    let snapshot = state.data.dashboard_snapshot(&query);
    let json = serde_json::to_string_pretty(&snapshot)
        .map_err(|err| format!("Failed to serialize snapshot: {err}"))?;

    let dialog_app = app.clone();
    let chosen = tokio::task::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_file_name("tokscale-export.json")
            .add_filter("JSON", &["json"])
            .blocking_save_file()
    })
    .await
    .map_err(|err| format!("Export dialog task failed: {err}"))?;

    if let Some(file_path) = chosen {
        let path = file_path
            .into_path()
            .map_err(|err| format!("Invalid export path: {err}"))?;
        tokio::fs::write(&path, json.as_bytes())
            .await
            .map_err(|err| format!("Failed to write export file: {err}"))?;
    }

    Ok(json)
}

#[tauri::command]
pub async fn get_sources_health(state: State<'_, AppState>) -> Result<DashboardSources, String> {
    let query = DashboardQuery {
        range: RangePreset::ThirtyDays,
        custom_since: None,
        custom_until: None,
        group_by: "model".to_string(),
        active_clients: Vec::new(),
        search: None,
        sort: None,
    };
    Ok(state.data.dashboard_snapshot(&query).sources)
}

#[tauri::command]
pub async fn get_tokscale_settings(
    state: State<'_, AppState>,
) -> Result<TokscaleSettingsState, String> {
    let app_prefs_path = state
        .shell_state
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .config_dir
        .as_ref()
        .map(|d| crate::shell::prefs::preferences_file_path(d));

    cli_config::read_tokscale_settings(state.cli.env(), app_prefs_path.as_deref())
}

#[tauri::command]
pub async fn update_tokscale_settings(
    state: State<'_, AppState>,
    patch: TokscaleSettingsPatch,
) -> Result<TokscaleSettingsState, String> {
    let app_prefs_path = state
        .shell_state
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .config_dir
        .as_ref()
        .map(|d| crate::shell::prefs::preferences_file_path(d));

    cli_config::patch_tokscale_settings(state.cli.env(), app_prefs_path.as_deref(), &patch)
}

#[tauri::command]
pub async fn recover_tokscale_timezone(
    state: State<'_, AppState>,
) -> Result<CliActionResult, String> {
    Ok(cli_config::recover_timezone(&state.cli).await)
}

#[tauri::command]
pub async fn get_custom_pricing(state: State<'_, AppState>) -> Result<CustomPricingState, String> {
    cli_config::read_custom_pricing(state.cli.env())
}

#[tauri::command]
pub async fn save_custom_pricing(
    state: State<'_, AppState>,
    entries: Vec<CustomPricingEntry>,
) -> Result<CustomPricingState, String> {
    cli_config::save_custom_pricing(state.cli.env(), &state.cli, &entries).await
}

#[tauri::command]
pub async fn lookup_model_pricing(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<ModelPriceLookupResult, String> {
    cli_config::lookup_model_pricing(&state.cli, &model_id).await
}

#[tauri::command]
pub async fn refresh_pricing_catalog(
    state: State<'_, AppState>,
) -> Result<CliActionResult, String> {
    Ok(cli_config::refresh_pricing_catalog(&state.cli).await)
}

#[tauri::command]
pub async fn get_providers_status(
    state: State<'_, AppState>,
) -> Result<ProvidersStatusPayload, String> {
    Ok(cli_config::get_providers_status(&state.cli).await)
}

#[tauri::command]
pub async fn run_provider_action(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ProviderActionRequest,
) -> Result<CliActionResult, String> {
    let result = cli_config::run_provider_action(&state.cli, &request).await;
    if result.ok && (request.action == "sync" || request.action == "purge_cache") {
        let _ = state.execute_sync_internal(SyncMode::LocalOffline, Some(&app)).await;
    }
    Ok(result)
}

#[tauri::command]
pub async fn get_social_status(state: State<'_, AppState>) -> Result<SocialStatusPayload, String> {
    Ok(cli_config::get_social_status(&state.cli).await)
}

#[tauri::command]
pub async fn run_social_action(
    state: State<'_, AppState>,
    request: SocialActionRequest,
) -> Result<CliActionResult, String> {
    Ok(cli_config::run_social_action(&state.cli, &request).await)
}

#[tauri::command]
pub async fn configure_autosubmit(
    state: State<'_, AppState>,
    request: AutosubmitConfigRequest,
) -> Result<CliActionResult, String> {
    Ok(cli_config::configure_autosubmit(&state.cli, &request).await)
}

#[tauri::command]
pub async fn reveal_config_path(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path cannot be empty".to_string());
    }
    let pb = PathBuf::from(trimmed);
    tokio::task::spawn_blocking(move || cli_config::reveal_path(&pb))
        .await
        .map_err(|err| format!("Reveal path task failed: {err}"))?
}

#[tauri::command]
pub async fn pick_path(app: AppHandle, kind: String) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let dialog_app = app.clone();
    let is_folder = kind == "folder";
    let chosen = tokio::task::spawn_blocking(move || {
        if is_folder {
            dialog_app.dialog().file().blocking_pick_folder()
        } else {
            dialog_app.dialog().file().blocking_pick_file()
        }
    })
    .await
    .map_err(|err| format!("Picker dialog task failed: {err}"))?;

    match chosen {
        Some(fp) => fp
            .into_path()
            .map(|p| Some(p.to_string_lossy().into_owned()))
            .map_err(|err| format!("Invalid selected path: {err}")),
        None => Ok(None),
    }
}

async fn refresh_quota_and_notify(app: &AppHandle<Wry>, state: &AppState) {
    if let Ok(raw_usage) = state
        .cli
        .run_json::<serde_json::Value>(cli_commands::usage(), cli_commands::TIMEOUT_SHORT)
        .await
    {
        let _ = state.data.update_usage_json(&raw_usage);
    }
    let report = state.data.hydrate_from_disk();
    update_tray_and_notify(state);

    let _ = app.emit(
        events::SNAPSHOT_UPDATED,
        SnapshotEvent {
            timestamp: current_unix_secs(),
            partial: report.fallback_required || !report.warnings.is_empty(),
            warnings: report.warnings,
        },
    );
}

fn update_tray_and_notify(state: &AppState) {
    let controller = state
        .shell
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let Some(ctrl) = controller else {
        return;
    };

    let active_range = state
        .shell_state
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .active_range;

    let active_tray = state.data.tray_snapshot(active_range);
    ctrl.update_readout(&TraySnapshotLite::from(&active_tray));

    let today_tray = if active_range == RangePreset::Today {
        active_tray
    } else {
        state.data.tray_snapshot(RangePreset::Today)
    };
    let today_dash = state.data.dashboard_snapshot(&DashboardQuery {
        range: RangePreset::Today,
        custom_since: None,
        custom_until: None,
        group_by: "model".to_string(),
        active_clients: Vec::new(),
        search: None,
        sort: None,
    });
    ctrl.evaluate_notifications(today_tray.total.cost, &today_dash.usage);
}

fn emit_sync_progress(app: Option<&AppHandle<Wry>>, payload: SyncProgressEvent) {
    if let Some(app_handle) = app {
        let _ = app_handle.emit(events::SYNC_PROGRESS, payload);
    }
}

fn push_unique_warning(warnings: &mut Vec<SourceWarning>, warning: SourceWarning) {
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

fn merge_preferences_in_memory(
    prefs_lock: &Arc<RwLock<CompanionPreferences>>,
    shell_state: &Arc<RwLock<ShellState>>,
    patch: &serde_json::Value,
) -> Result<CompanionPreferences, String> {
    let patch_obj = patch
        .as_object()
        .ok_or_else(|| "preferences patch must be a JSON object".to_string())?;
    let current = prefs_lock
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let mut base = serde_json::to_value(&current)
        .map_err(|err| err.to_string())?
        .as_object()
        .cloned()
        .unwrap_or_default();
    for (k, v) in patch_obj {
        base.insert(k.clone(), v.clone());
    }
    let next: CompanionPreferences =
        serde_json::from_value(serde_json::Value::Object(base)).map_err(|err| err.to_string())?;
    shell_state
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .preferences = next.clone();
    Ok(next)
}

fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn expand_tilde_path(raw: &str) -> PathBuf {
    if let Some(stripped) = raw.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(raw)
}

fn default_watch_paths() -> Vec<PathBuf> {
    let mut paths = vec![crate::data::cache::tokscale_config_dir().join("cache")];
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".claude").join("projects"));
        paths.push(home.join(".codex").join("sessions"));
        paths.push(home.join(".local/share/opencode/storage"));
        paths.push(home.join(".gemini").join("tmp"));
    }
    paths
}
