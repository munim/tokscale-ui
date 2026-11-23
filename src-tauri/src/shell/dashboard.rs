use std::sync::{Arc, RwLock};

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry};

use crate::contracts::events;
use crate::shell::{read_state, write_state, DashboardHandoff, DockPolicy, ShellState};

pub const DASHBOARD_WINDOW_LABEL: &str = "dashboard";
pub const DASHBOARD_URL: &str = "dashboard.html";
pub const DASHBOARD_WIDTH: f64 = 1280.0;
pub const DASHBOARD_HEIGHT: f64 = 820.0;
pub const DASHBOARD_MIN_WIDTH: f64 = 900.0;
pub const DASHBOARD_MIN_HEIGHT: f64 = 600.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAction {
    ColdCreate,
    WarmEmit(DashboardHandoff),
}

pub fn apply_dock_policy(app: &AppHandle<Wry>, policy: DockPolicy) {
    #[cfg(target_os = "macos")]
    {
        let native_policy = match policy {
            DockPolicy::Accessory => tauri::ActivationPolicy::Accessory,
            DockPolicy::Regular => tauri::ActivationPolicy::Regular,
        };
        let _ = app.set_activation_policy(native_policy);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, policy);
    }
}

pub fn apply_startup_dock_policy(app: &AppHandle<Wry>, state: &Arc<RwLock<ShellState>>) {
    apply_dock_policy(app, DockPolicy::Accessory);
    write_state(state).dock_policy = DockPolicy::Accessory;
}

pub fn plan_dashboard_open(
    state: &Arc<RwLock<ShellState>>,
    handoff: DashboardHandoff,
) -> OpenAction {
    let mut guard = write_state(state);
    guard.dock_policy = DockPolicy::Regular;

    if guard.dashboard_open {
        OpenAction::WarmEmit(handoff)
    } else {
        guard.dashboard_open = true;
        guard.pending_handoff = Some(handoff);
        OpenAction::ColdCreate
    }
}

pub fn record_dashboard_close(state: &Arc<RwLock<ShellState>>) -> DockPolicy {
    let mut guard = write_state(state);
    guard.dashboard_open = false;
    guard.pending_handoff = None;
    let next_policy = if guard.settings_open {
        DockPolicy::Regular
    } else {
        DockPolicy::Accessory
    };
    guard.dock_policy = next_policy;
    next_policy
}

pub fn take_pending_handoff(state: &Arc<RwLock<ShellState>>) -> Option<DashboardHandoff> {
    write_state(state).pending_handoff.take()
}

pub fn peek_pending_handoff(state: &Arc<RwLock<ShellState>>) -> Option<DashboardHandoff> {
    read_state(state).pending_handoff.clone()
}

pub fn open_dashboard(
    app: &AppHandle<Wry>,
    state: &Arc<RwLock<ShellState>>,
    handoff: DashboardHandoff,
) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    }

    if let Some(window) = app.get_webview_window(DASHBOARD_WINDOW_LABEL) {
        {
            let mut guard = write_state(state);
            guard.dashboard_open = true;
            guard.dock_policy = DockPolicy::Regular;
        }
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit(events::DASHBOARD_NAVIGATE, &handoff);
        return Ok(());
    }

    {
        let mut guard = write_state(state);
        guard.dashboard_open = true;
        guard.dock_policy = DockPolicy::Regular;
        guard.pending_handoff = Some(handoff);
    }

    let builder = WebviewWindowBuilder::new(
        app,
        DASHBOARD_WINDOW_LABEL,
        WebviewUrl::App(DASHBOARD_URL.into()),
    )
    .title("Tokscale")
    .inner_size(DASHBOARD_WIDTH, DASHBOARD_HEIGHT)
    .min_inner_size(DASHBOARD_MIN_WIDTH, DASHBOARD_MIN_HEIGHT)
    .visible(true);

    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);

    let window = match builder.build() {
        Ok(win) => win,
        Err(err) => {
            let next_policy = record_dashboard_close(state);
            apply_dock_policy(app, next_policy);
            return Err(err);
        }
    };

    let event_state = Arc::clone(state);
    let app_handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed => {
            let next_policy = record_dashboard_close(&event_state);
            apply_dock_policy(&app_handle, next_policy);
        }
        _ => {}
    });

    let _ = window.show();
    let _ = window.set_focus();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{DashboardTab, RangePreset};

    #[test]
    fn cold_open_stores_pending_handoff_and_warm_open_emits() {
        let state = Arc::new(RwLock::new(ShellState {
            tray_alive: true,
            ..ShellState::default()
        }));

        let cold_handoff = DashboardHandoff {
            tab: DashboardTab::Models,
            range: Some(RangePreset::SevenDays),
            query: Some("sonnet".to_string()),
        };

        let action = plan_dashboard_open(&state, cold_handoff.clone());
        assert_eq!(action, OpenAction::ColdCreate);
        assert_eq!(read_state(&state).dock_policy, DockPolicy::Regular);
        assert!(read_state(&state).dashboard_open);
        assert_eq!(peek_pending_handoff(&state), Some(cold_handoff.clone()));

        let warm_handoff = DashboardHandoff {
            tab: DashboardTab::Sessions,
            range: Some(RangePreset::ThirtyDays),
            query: None,
        };
        let action2 = plan_dashboard_open(&state, warm_handoff.clone());
        assert_eq!(action2, OpenAction::WarmEmit(warm_handoff));

        // First cold snapshot consumes pending handoff.
        assert_eq!(take_pending_handoff(&state), Some(cold_handoff));
        assert_eq!(take_pending_handoff(&state), None);

        // Simulated CloseRequested drops dashboard state and reverts Dock policy while tray stays alive.
        record_dashboard_close(&state);
        let guard = read_state(&state);
        assert!(!guard.dashboard_open);
        assert_eq!(guard.dock_policy, DockPolicy::Accessory);
        assert!(guard.tray_alive);
        assert!(guard.pending_handoff.is_none());
    }
}
