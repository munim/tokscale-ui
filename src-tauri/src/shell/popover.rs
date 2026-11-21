use std::sync::{Arc, RwLock};

use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, Position, Rect, Size, WebviewWindow,
    WindowEvent, Wry,
};

use crate::shell::{read_state, write_state, ShellState};

pub const TRAY_WINDOW_LABEL: &str = "tray";
pub const POPOVER_WIDTH_LOGICAL: f64 = 360.0;
pub const POPOVER_MIN_HEIGHT_LOGICAL: f64 = 120.0;
pub const POPOVER_MAX_HEIGHT_LOGICAL: f64 = 560.0;
pub const POPOVER_DEFAULT_HEIGHT_LOGICAL: f64 = 520.0;
const POPOVER_GAP_LOGICAL: f64 = 6.0;
const WORK_AREA_MARGIN_LOGICAL: f64 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverPlatform {
    MacOs,
    Windows,
    Linux,
}

impl PopoverPlatform {
    pub fn current() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::MacOs
        }
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Self::Linux
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkAreaBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayIconBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopoverSize {
    pub width: u32,
    pub height: u32,
}

pub fn clamp_popover_height(requested_logical_height: f64) -> f64 {
    if !requested_logical_height.is_finite() {
        return POPOVER_DEFAULT_HEIGHT_LOGICAL;
    }
    requested_logical_height.clamp(POPOVER_MIN_HEIGHT_LOGICAL, POPOVER_MAX_HEIGHT_LOGICAL)
}

pub fn calculate_popover_position(
    platform: PopoverPlatform,
    tray: TrayIconBounds,
    popover: PopoverSize,
    work_area: WorkAreaBounds,
    scale_factor: f64,
) -> (i32, i32) {
    let scale = if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };
    let gap = (POPOVER_GAP_LOGICAL * scale).round() as i32;
    let margin = (WORK_AREA_MARGIN_LOGICAL * scale).round() as i32;

    let tray_center_x = tray.x + (tray.width as i32 / 2);
    let ideal_x = tray_center_x - (popover.width as i32 / 2);

    let ideal_y = match platform {
        PopoverPlatform::MacOs | PopoverPlatform::Linux => tray.y + tray.height as i32 + gap,
        PopoverPlatform::Windows => {
            let work_mid_y = work_area.y + (work_area.height as i32 / 2);
            if tray.y >= work_mid_y {
                tray.y - popover.height as i32 - gap
            } else {
                tray.y + tray.height as i32 + gap
            }
        }
    };

    let min_x = work_area.x + margin;
    let max_x = (work_area.x + work_area.width as i32 - popover.width as i32 - margin).max(min_x);

    let min_y = work_area.y + margin;
    let max_y = (work_area.y + work_area.height as i32 - popover.height as i32 - margin).max(min_y);

    (ideal_x.clamp(min_x, max_x), ideal_y.clamp(min_y, max_y))
}

pub fn setup_popover(app: &AppHandle<Wry>, state: &Arc<RwLock<ShellState>>) {
    let Some(window) = app.get_webview_window(TRAY_WINDOW_LABEL) else {
        return;
    };

    #[cfg(target_os = "macos")]
    {
        if let Err(err) = window_vibrancy::apply_vibrancy(
            &window,
            window_vibrancy::NSVisualEffectMaterial::Popover,
            None,
            Some(12.0),
        ) {
            write_state(state).warnings.push(crate::contracts::SourceWarning {
                source: "popover".to_string(),
                message: format!("Failed to apply macOS popover vibrancy: {err}"),
            });
        }
    }

    let event_state = Arc::clone(state);
    let win_handle = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Focused(false) => {
            let _ = win_handle.hide();
            write_state(&event_state).tray_visible = false;
        }
        WindowEvent::CloseRequested { api, .. } => {
            // Keep the tray WebView alive for <16ms reopening.
            api.prevent_close();
            let _ = win_handle.hide();
            write_state(&event_state).tray_visible = false;
        }
        _ => {}
    });
}

pub fn toggle_popover(
    app: &AppHandle<Wry>,
    state: &Arc<RwLock<ShellState>>,
    tray_rect: Option<Rect>,
) {
    let Some(window) = app.get_webview_window(TRAY_WINDOW_LABEL) else {
        return;
    };

    let currently_visible = window
        .is_visible()
        .unwrap_or_else(|_| read_state(state).tray_visible);

    if currently_visible {
        dismiss_popover(app, state);
    } else {
        show_popover(&window, state, tray_rect);
    }
}

pub fn show_popover(
    window: &WebviewWindow<Wry>,
    state: &Arc<RwLock<ShellState>>,
    tray_rect: Option<Rect>,
) {
    if let Some(rect) = tray_rect {
        position_popover_near_tray(window, rect);
    } else {
        position_popover_default(window);
    }

    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.eval(
        "if(!window.__tokscaleEscBound){window.__tokscaleEscBound=true;\
         window.addEventListener('keydown',function(e){\
         if(e.key==='Escape'&&window.__TAURI_INTERNALS__){\
         window.__TAURI_INTERNALS__.invoke('dismiss_tray').catch(function(){});\
         }},true);}",
    );

    write_state(state).tray_visible = true;
}

pub fn dismiss_popover(app: &AppHandle<Wry>, state: &Arc<RwLock<ShellState>>) {
    if let Some(window) = app.get_webview_window(TRAY_WINDOW_LABEL) {
        let _ = window.hide();
    }
    write_state(state).tray_visible = false;
}

pub fn set_popover_height(app: &AppHandle<Wry>, logical_height: f64) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(TRAY_WINDOW_LABEL) {
        let clamped = clamp_popover_height(logical_height);
        window.set_size(Size::Logical(LogicalSize::new(
            POPOVER_WIDTH_LOGICAL,
            clamped,
        )))?;
    }
    Ok(())
}

fn position_popover_near_tray(window: &WebviewWindow<Wry>, rect: Rect) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let tray_pos = rect.position.to_physical::<i32>(scale);
    let tray_size = rect.size.to_physical::<u32>(scale);

    let monitor = window
        .monitor_from_point(tray_pos.x as f64, tray_pos.y as f64)
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        return;
    };

    let monitor_scale = monitor.scale_factor();
    let work = monitor.work_area();
    let work_bounds = WorkAreaBounds {
        x: work.position.x,
        y: work.position.y,
        width: work.size.width,
        height: work.size.height,
    };

    let outer = window.outer_size().ok();
    let popover_width = outer
        .map(|s| s.width)
        .unwrap_or_else(|| (POPOVER_WIDTH_LOGICAL * monitor_scale).round() as u32);
    let max_phys_height = (POPOVER_MAX_HEIGHT_LOGICAL * monitor_scale).round() as u32;
    let popover_height = outer
        .map(|s| s.height.min(max_phys_height))
        .unwrap_or_else(|| (POPOVER_DEFAULT_HEIGHT_LOGICAL * monitor_scale).round() as u32);

    let (x, y) = calculate_popover_position(
        PopoverPlatform::current(),
        TrayIconBounds {
            x: tray_pos.x,
            y: tray_pos.y,
            width: tray_size.width,
            height: tray_size.height,
        },
        PopoverSize {
            width: popover_width,
            height: popover_height,
        },
        work_bounds,
        monitor_scale,
    );

    let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
}

fn position_popover_default(window: &WebviewWindow<Wry>) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        return;
    };

    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let margin = (WORK_AREA_MARGIN_LOGICAL * scale).round() as i32;
    let outer = window.outer_size().ok();
    let popover_width = outer
        .map(|s| s.width as i32)
        .unwrap_or_else(|| (POPOVER_WIDTH_LOGICAL * scale).round() as i32);
    let popover_height = outer
        .map(|s| s.height as i32)
        .unwrap_or_else(|| (POPOVER_DEFAULT_HEIGHT_LOGICAL * scale).round() as i32);

    let x = (work.position.x + work.size.width as i32 - popover_width - margin)
        .max(work.position.x + margin);

    let y = match PopoverPlatform::current() {
        PopoverPlatform::Windows => {
            (work.position.y + work.size.height as i32 - popover_height - margin)
                .max(work.position.y + margin)
        }
        PopoverPlatform::MacOs | PopoverPlatform::Linux => work.position.y + margin,
    };

    let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_positions_below_tray_and_clamps_right_edge() {
        let work = WorkAreaBounds {
            x: 0,
            y: 25,
            width: 1440,
            height: 875,
        };
        // Tray icon very close to right edge.
        let tray = TrayIconBounds {
            x: 1400,
            y: 2,
            width: 28,
            height: 22,
        };
        let popover = PopoverSize {
            width: 360,
            height: 520,
        };

        let (x, y) = calculate_popover_position(PopoverPlatform::MacOs, tray, popover, work, 1.0);

        // Clamped within right margin: 1440 - 360 - 8 = 1072
        assert_eq!(x, 1072);
        // Positioned below tray (2 + 22 + 6 = 30), clamped inside work area (min_y = 25 + 8 = 33)
        assert_eq!(y, 33);
        assert!(x >= work.x);
        assert!(x + popover.width as i32 <= work.x + work.width as i32);
        assert!(y >= work.y);
        assert!(y + popover.height as i32 <= work.y + work.height as i32);
    }

    #[test]
    fn macos_clamps_to_secondary_monitor_with_negative_x() {
        let work = WorkAreaBounds {
            x: -1920,
            y: 25,
            width: 1920,
            height: 1055,
        };
        let tray = TrayIconBounds {
            x: -1910,
            y: 4,
            width: 24,
            height: 20,
        };
        let popover = PopoverSize {
            width: 360,
            height: 500,
        };

        let (x, y) = calculate_popover_position(PopoverPlatform::MacOs, tray, popover, work, 1.0);
        assert_eq!(x, -1920 + 8);
        assert_eq!(y, 25 + 8);
    }

    #[test]
    fn windows_positions_above_bottom_taskbar_and_below_top_taskbar() {
        let work = WorkAreaBounds {
            x: 0,
            y: 0,
            width: 1920,
            height: 1032,
        };
        let popover = PopoverSize {
            width: 360,
            height: 520,
        };

        // Bottom taskbar tray icon
        let bottom_tray = TrayIconBounds {
            x: 1700,
            y: 1040,
            width: 32,
            height: 32,
        };
        let (x, y) =
            calculate_popover_position(PopoverPlatform::Windows, bottom_tray, popover, work, 1.0);
        assert_eq!(x, 1536);
        assert_eq!(y, 1032 - 520 - 8);

        // Top taskbar tray icon
        let top_tray = TrayIconBounds {
            x: 1700,
            y: 8,
            width: 32,
            height: 32,
        };
        let (_x2, y2) =
            calculate_popover_position(PopoverPlatform::Windows, top_tray, popover, work, 1.0);
        assert_eq!(y2, 8 + 32 + 6);
    }

    #[test]
    fn clamp_popover_height_enforces_max_560() {
        assert_eq!(clamp_popover_height(50.0), 120.0);
        assert_eq!(clamp_popover_height(420.0), 420.0);
        assert_eq!(clamp_popover_height(900.0), 560.0);
        assert_eq!(clamp_popover_height(f64::NAN), 520.0);
    }
}
