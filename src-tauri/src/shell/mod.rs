//! Native tray/menubar controller, dashboard WebView lifecycle, watcher, and preferences.

pub mod autostart;
pub mod dashboard;
pub mod notify;
pub mod popover;
pub mod prefs;
pub mod settings;
pub mod shortcut;
pub mod timers;
pub mod tray;
pub mod updater;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::contracts::{
    events, CompanionPreferences, DashboardNavigateEvent, ProviderUsageView, RangePreset,
    SettingsSection, SourceWarning, SyncMode, TraySnapshot,
};

pub type DashboardHandoff = DashboardNavigateEvent;

#[derive(Debug, thiserror::Error)]
pub enum ShellError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Tauri error: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("preferences error: {0}")]
    Preferences(String),
    #[error("shortcut error: {0}")]
    Shortcut(String),
    #[error("autostart error: {0}")]
    Autostart(String),
    #[error("watcher error: {0}")]
    Watcher(String),
    #[error("notification error: {0}")]
    Notification(String),
}

impl From<ShellError> for String {
    fn from(err: ShellError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockPolicy {
    #[default]
    Accessory,
    Regular,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TraySnapshotLite {
    pub today_cost: f64,
    pub today_tokens: u64,
    pub range: RangePreset,
}

impl Default for TraySnapshotLite {
    fn default() -> Self {
        Self {
            today_cost: 0.0,
            today_tokens: 0,
            range: RangePreset::Today,
        }
    }
}

impl From<&TraySnapshot> for TraySnapshotLite {
    fn from(snapshot: &TraySnapshot) -> Self {
        Self {
            today_cost: snapshot.total.cost,
            today_tokens: snapshot.total.tokens,
            range: snapshot.range,
        }
    }
}

#[derive(Clone, Default)]
pub struct ShellHooks {
    pub on_sync_requested: Option<Arc<dyn Fn(SyncMode) + Send + Sync>>,
    pub on_quota_refresh: Option<Arc<dyn Fn() + Send + Sync>>,
    pub on_update_check: Option<Arc<dyn Fn() + Send + Sync>>,
    pub on_check_updates_requested: Option<Arc<dyn Fn() + Send + Sync>>,
    pub on_range_changed: Option<Arc<dyn Fn(RangePreset) + Send + Sync>>,
    pub on_quit_requested: Option<Arc<dyn Fn() + Send + Sync>>,
}

pub struct ShellState {
    pub preferences: CompanionPreferences,
    pub active_range: RangePreset,
    pub latest_readout: TraySnapshotLite,
    pub pending_handoff: Option<DashboardHandoff>,
    pub dashboard_open: bool,
    pub pending_settings_section: Option<SettingsSection>,
    pub settings_open: bool,
    pub tray_visible: bool,
    pub tray_alive: bool,
    pub last_tray_rect: Option<tauri::Rect>,
    pub dock_policy: DockPolicy,
    pub warnings: Vec<SourceWarning>,
    pub config_dir: Option<PathBuf>,
    pub notification_tracker: notify::NotificationTracker,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            preferences: CompanionPreferences::default(),
            active_range: RangePreset::Today,
            latest_readout: TraySnapshotLite::default(),
            pending_handoff: None,
            dashboard_open: false,
            pending_settings_section: None,
            settings_open: false,
            tray_visible: false,
            tray_alive: false,
            last_tray_rect: None,
            dock_policy: DockPolicy::Accessory,
            warnings: Vec::new(),
            config_dir: None,
            notification_tracker: notify::NotificationTracker::default(),
        }
    }
}

pub(crate) fn read_state(state: &Arc<RwLock<ShellState>>) -> RwLockReadGuard<'_, ShellState> {
    state.read().unwrap_or_else(|err| err.into_inner())
}

pub(crate) fn write_state(state: &Arc<RwLock<ShellState>>) -> RwLockWriteGuard<'_, ShellState> {
    state.write().unwrap_or_else(|err| err.into_inner())
}

struct ShellControllerInner {
    app: AppHandle<Wry>,
    state: Arc<RwLock<ShellState>>,
    tray: tray::TrayHandle,
    hooks: Arc<RwLock<ShellHooks>>,
    timers: Mutex<Option<timers::ShellTimers>>,
}

#[derive(Clone)]
pub struct ShellController {
    inner: Arc<ShellControllerInner>,
}

impl ShellController {
    pub fn install(app: &tauri::App, state: Arc<RwLock<ShellState>>) -> tauri::Result<Self> {
        Self::install_with_hooks(app, state, ShellHooks::default(), Vec::new())
    }

    pub fn install_with_hooks(
        app: &tauri::App,
        state: Arc<RwLock<ShellState>>,
        hooks: ShellHooks,
        watch_paths: Vec<PathBuf>,
    ) -> tauri::Result<Self> {
        let app_handle = app.app_handle().clone();

        if let Ok(config_dir) = app.path().app_config_dir() {
            let loaded = if prefs::preferences_file_path(&config_dir).exists() {
                match prefs::load_preferences(&config_dir) {
                    Ok(p) => Some(p),
                    Err(err) => {
                        write_state(&state).warnings.push(SourceWarning {
                            source: "preferences".to_string(),
                            message: format!("Failed to load preferences.json, using defaults: {err}"),
                        });
                        None
                    }
                }
            } else {
                None
            };

            let mut guard = write_state(&state);
            guard.config_dir = Some(config_dir);
            if let Some(p) = loaded {
                guard.preferences = p;
            }
        }

        dashboard::apply_startup_dock_policy(&app_handle, &state);
        popover::setup_popover(&app_handle, &state);

        let hooks_arc = Arc::new(RwLock::new(hooks));
        let timers = timers::ShellTimers::start(watch_paths, Arc::clone(&hooks_arc));
        let timers_arc = Arc::new(Mutex::new(Some(timers)));

        let stop_timers_on_quit = {
            let timers_ref = Arc::clone(&timers_arc);
            let quit_state = Arc::clone(&state);
            Arc::new(move || {
                if let Some(t) = timers_ref
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take()
                {
                    t.stop();
                }
                write_state(&quit_state).tray_alive = false;
            })
        };

        let tray = tray::setup_tray(&app_handle, &state, &hooks_arc, stop_timers_on_quit)?;

        let initial_prefs = read_state(&state).preferences.clone();
        if let Err(err) = autostart::sync_autostart(&app_handle, initial_prefs.launch_at_login) {
            write_state(&state).warnings.push(SourceWarning {
                source: "autostart".to_string(),
                message: format!("Failed to sync autostart preference: {err}"),
            });
        }

        let _ = shortcut::apply_global_shortcut(
            &app_handle,
            &state,
            initial_prefs.global_shortcut.as_deref(),
        );

        let timers_slot = timers_arc
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();

        Ok(Self {
            inner: Arc::new(ShellControllerInner {
                app: app_handle,
                state,
                tray,
                hooks: hooks_arc,
                timers: Mutex::new(timers_slot),
            }),
        })
    }

    pub fn update_readout(&self, snapshot: &TraySnapshotLite) {
        let display_mode = {
            let mut guard = write_state(&self.inner.state);
            guard.latest_readout = snapshot.clone();
            guard.active_range = snapshot.range;
            guard.preferences.menubar_display.clone()
        };

        self.inner.tray.apply_readout(&display_mode, snapshot);
        self.inner.tray.sync_range_checks(snapshot.range);
    }

    pub fn set_update_menu_label(&self, label: &str) {
        self.inner.tray.set_update_menu_label(label);
    }

    pub fn evaluate_notifications(&self, today_cost: f64, usage: &[ProviderUsageView]) {
        let today = notify::current_local_date();
        let alerts = {
            let mut guard = write_state(&self.inner.state);
            let prefs = guard.preferences.clone();
            guard
                .notification_tracker
                .evaluate(&prefs, today, today_cost, usage)
        };
        notify::dispatch_alerts(&self.inner.app, &alerts);
    }

    pub fn open_dashboard(&self, handoff: DashboardHandoff) -> tauri::Result<()> {
        dashboard::open_dashboard(&self.inner.app, &self.inner.state, handoff)
    }

    pub fn open_settings(&self, section: SettingsSection) -> tauri::Result<()> {
        settings::open_settings(&self.inner.app, &self.inner.state, section)
    }

    pub fn take_pending_settings_section(&self) -> Option<SettingsSection> {
        settings::take_pending_settings_section(&self.inner.state)
    }

    pub fn dismiss_tray(&self) {
        popover::dismiss_popover(&self.inner.app, &self.inner.state);
    }

    pub fn set_popover_height(&self, logical_height: f64) -> tauri::Result<()> {
        popover::set_popover_height(&self.inner.app, logical_height)
    }

    pub fn resize_popover(&self, logical_height: f64) -> tauri::Result<()> {
        popover::resize_popover(&self.inner.app, &self.inner.state, logical_height)
    }

    pub fn set_preferences(
        &self,
        patch: serde_json::Value,
    ) -> Result<CompanionPreferences, ShellError> {
        let (config_dir, previous_prefs) = {
            let guard = read_state(&self.inner.state);
            (guard.config_dir.clone(), guard.preferences.clone())
        };

        let resolved_dir = match config_dir {
            Some(dir) => dir,
            None => self.inner.app.path().app_config_dir()?,
        };

        let updated = prefs::patch_preferences(&resolved_dir, &patch)?;

        let latest_readout = {
            let mut guard = write_state(&self.inner.state);
            guard.config_dir = Some(resolved_dir);
            guard.preferences = updated.clone();
            guard.latest_readout.clone()
        };

        if previous_prefs.menubar_display != updated.menubar_display {
            self.inner
                .tray
                .apply_readout(&updated.menubar_display, &latest_readout);
        }

        if previous_prefs.launch_at_login != updated.launch_at_login {
            if let Err(err) = autostart::sync_autostart(&self.inner.app, updated.launch_at_login) {
                write_state(&self.inner.state).warnings.push(SourceWarning {
                    source: "autostart".to_string(),
                    message: format!("Failed to update launchAtLogin: {err}"),
                });
            }
        }

        if shortcut::normalize_shortcut(previous_prefs.global_shortcut.as_deref())
            != shortcut::normalize_shortcut(updated.global_shortcut.as_deref())
        {
            let _ = shortcut::apply_global_shortcut(
                &self.inner.app,
                &self.inner.state,
                updated.global_shortcut.as_deref(),
            );
        }

        let _ = self.inner.app.emit(events::PREFERENCES_UPDATED, &updated);
        Ok(updated)
    }

    pub fn get_preferences(&self) -> CompanionPreferences {
        read_state(&self.inner.state).preferences.clone()
    }

    pub fn take_pending_handoff(&self) -> Option<DashboardHandoff> {
        dashboard::take_pending_handoff(&self.inner.state)
    }

    pub fn set_hooks(&self, hooks: ShellHooks) {
        let mut guard = self
            .inner
            .hooks
            .write()
            .unwrap_or_else(|err| err.into_inner());
        *guard = hooks;
    }

    pub fn set_watch_paths(&self, paths: Vec<PathBuf>) {
        if let Some(timers) = self
            .inner
            .timers
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .as_ref()
        {
            timers.set_watch_paths(paths);
        }
    }

    pub fn is_tray_alive(&self) -> bool {
        read_state(&self.inner.state).tray_alive
    }

    pub fn is_dashboard_open(&self) -> bool {
        read_state(&self.inner.state).dashboard_open
    }

    pub fn warnings(&self) -> Vec<SourceWarning> {
        read_state(&self.inner.state).warnings.clone()
    }

    pub fn shutdown(&self) {
        if let Some(timers) = self
            .inner
            .timers
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .take()
        {
            timers.stop();
        }
        write_state(&self.inner.state).tray_alive = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{CliResolutionInfo, SyncStatus};

    #[test]
    fn tray_snapshot_lite_converts_from_tray_snapshot() {
        let full = TraySnapshot {
            range: RangePreset::SevenDays,
            total: crate::contracts::TrayTotal {
                cost: 48.12,
                tokens: 61_400_000,
                sessions: 7,
            },
            bars: vec![],
            harnesses: vec![],
            top_models: vec![],
            sources: vec![],
            sync_status: SyncStatus::default(),
            cli_info: CliResolutionInfo {
                resolved_via: "probe".to_string(),
                binary_path: None,
                version: None,
                fallback: None,
            },
        };

        let lite = TraySnapshotLite::from(&full);
        assert_eq!(lite.today_cost, 48.12);
        assert_eq!(lite.today_tokens, 61_400_000);
        assert_eq!(lite.range, RangePreset::SevenDays);
    }
}
