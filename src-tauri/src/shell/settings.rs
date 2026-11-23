use std::sync::{Arc, RwLock};

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry};

use crate::contracts::{events, SettingsNavigateEvent, SettingsSection};
use crate::shell::{dashboard, read_state, write_state, DockPolicy, ShellState};

pub const SETTINGS_WINDOW_LABEL: &str = "settings";
pub const SETTINGS_URL: &str = "settings.html";
pub const SETTINGS_WIDTH: f64 = 840.0;
pub const SETTINGS_HEIGHT: f64 = 580.0;
pub const SETTINGS_MIN_WIDTH: f64 = 720.0;
pub const SETTINGS_MIN_HEIGHT: f64 = 500.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsOpenAction {
    ColdCreate,
    WarmEmit(SettingsSection),
}

pub fn plan_settings_open(
    state: &Arc<RwLock<ShellState>>,
    section: SettingsSection,
) -> SettingsOpenAction {
    let mut guard = write_state(state);
    guard.dock_policy = DockPolicy::Regular;

    if guard.settings_open {
        SettingsOpenAction::WarmEmit(section)
    } else {
        guard.settings_open = true;
        guard.pending_settings_section = Some(section);
        SettingsOpenAction::ColdCreate
    }
}

pub fn record_settings_close(state: &Arc<RwLock<ShellState>>) -> DockPolicy {
    let mut guard = write_state(state);
    guard.settings_open = false;
    guard.pending_settings_section = None;
    let next_policy = if guard.dashboard_open {
        DockPolicy::Regular
    } else {
        DockPolicy::Accessory
    };
    guard.dock_policy = next_policy;
    next_policy
}

pub fn take_pending_settings_section(state: &Arc<RwLock<ShellState>>) -> Option<SettingsSection> {
    write_state(state).pending_settings_section.take()
}

pub fn open_settings(
    app: &AppHandle<Wry>,
    state: &Arc<RwLock<ShellState>>,
    section: SettingsSection,
) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    }

    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        {
            let mut guard = write_state(state);
            guard.settings_open = true;
            guard.dock_policy = DockPolicy::Regular;
        }
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit(events::SETTINGS_NAVIGATE, &SettingsNavigateEvent { section });
        return Ok(());
    }

    {
        let mut guard = write_state(state);
        guard.settings_open = true;
        guard.dock_policy = DockPolicy::Regular;
        guard.pending_settings_section = Some(section);
    }

    let builder = WebviewWindowBuilder::new(
        app,
        SETTINGS_WINDOW_LABEL,
        WebviewUrl::App(SETTINGS_URL.into()),
    )
    .title("Tokscale Settings")
    .inner_size(SETTINGS_WIDTH, SETTINGS_HEIGHT)
    .min_inner_size(SETTINGS_MIN_WIDTH, SETTINGS_MIN_HEIGHT)
    .center()
    .visible(true);

    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);

    let window = match builder.build() {
        Ok(win) => win,
        Err(err) => {
            let next_policy = record_settings_close(state);
            dashboard::apply_dock_policy(app, next_policy);
            return Err(err);
        }
    };

    let event_state = Arc::clone(state);
    let app_handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed => {
            let next_policy = record_settings_close(&event_state);
            dashboard::apply_dock_policy(&app_handle, next_policy);
        }
        _ => {}
    });

    let _ = window.show();
    let _ = window.set_focus();

    Ok(())
}

pub fn is_settings_open(state: &Arc<RwLock<ShellState>>) -> bool {
    read_state(state).settings_open
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::DashboardTab;
    use crate::shell::{dashboard::plan_dashboard_open, DashboardHandoff};

    #[test]
    fn settings_and_dashboard_coordinate_dock_policy_on_close() {
        let state = Arc::new(RwLock::new(ShellState {
            tray_alive: true,
            ..ShellState::default()
        }));

        // Opening settings moves Dock to Regular and stores pending section.
        let action = plan_settings_open(&state, SettingsSection::Providers);
        assert_eq!(action, SettingsOpenAction::ColdCreate);
        assert!(is_settings_open(&state));
        assert_eq!(read_state(&state).dock_policy, DockPolicy::Regular);
        assert_eq!(
            take_pending_settings_section(&state),
            Some(SettingsSection::Providers)
        );
        assert_eq!(take_pending_settings_section(&state), None);

        // Warm open emits navigation directly.
        let warm = plan_settings_open(&state, SettingsSection::Pricing);
        assert_eq!(warm, SettingsOpenAction::WarmEmit(SettingsSection::Pricing));

        // Open dashboard too.
        let _ = plan_dashboard_open(
            &state,
            DashboardHandoff {
                tab: DashboardTab::Overview,
                range: None,
                query: None,
            },
        );

        // Closing settings while dashboard is still open keeps Regular dock policy.
        let policy_after_settings_close = record_settings_close(&state);
        assert_eq!(policy_after_settings_close, DockPolicy::Regular);
        assert!(!is_settings_open(&state));
        assert_eq!(read_state(&state).dock_policy, DockPolicy::Regular);

        // Closing dashboard now reverts Dock to Accessory.
        let policy_after_dash_close = dashboard::record_dashboard_close(&state);
        assert_eq!(policy_after_dash_close, DockPolicy::Accessory);
        assert_eq!(read_state(&state).dock_policy, DockPolicy::Accessory);

        // Open settings again while dashboard is closed; closing settings reverts to Accessory.
        let _ = plan_settings_open(&state, SettingsSection::Cli);
        assert_eq!(record_settings_close(&state), DockPolicy::Accessory);
    }
}
