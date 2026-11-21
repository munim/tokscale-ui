use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::contracts::{DashboardTab, RangePreset, SyncMode};
use crate::shell::{
    dashboard, popover, read_state, write_state, DashboardHandoff, ShellHooks, ShellState,
    TraySnapshotLite,
};

pub const TRAY_ICON_ID: &str = "tokscale-tray";

pub const MENU_OPEN_DASHBOARD: &str = "tray:open-dashboard";
pub const MENU_SYNC_NOW: &str = "tray:sync-now";
pub const MENU_RANGE_TODAY: &str = "tray:range-today";
pub const MENU_RANGE_7D: &str = "tray:range-7d";
pub const MENU_RANGE_30D: &str = "tray:range-30d";
pub const MENU_RANGE_ALL: &str = "tray:range-all";
pub const MENU_QUIT: &str = "tray:quit";

const FALLBACK_ICON_PNG: &[u8] = include_bytes!("../../icons/32x32.png");

#[derive(Clone)]
pub struct TrayHandle {
    icon: TrayIcon<Wry>,
    range_today: CheckMenuItem<Wry>,
    range_7d: CheckMenuItem<Wry>,
    range_30d: CheckMenuItem<Wry>,
    range_all: CheckMenuItem<Wry>,
}

impl TrayHandle {
    pub fn apply_readout(&self, display_mode: &str, snapshot: &TraySnapshotLite) {
        let tooltip = format_windows_tooltip(snapshot.today_cost, snapshot.today_tokens);
        let _ = self.icon.set_tooltip(Some(tooltip));

        #[cfg(not(target_os = "windows"))]
        {
            let title = format_tray_title(display_mode, snapshot.today_cost, snapshot.today_tokens);
            if title.is_empty() {
                let _ = self.icon.set_title(None::<&str>);
            } else {
                let _ = self.icon.set_title(Some(title));
            }
        }
        #[cfg(target_os = "windows")]
        {
            let _ = display_mode;
        }
    }

    pub fn sync_range_checks(&self, range: RangePreset) {
        let active = normalize_menu_range(range);
        let _ = self.range_today.set_checked(active == RangePreset::Today);
        let _ = self.range_7d.set_checked(active == RangePreset::SevenDays);
        let _ = self.range_30d.set_checked(active == RangePreset::ThirtyDays);
        let _ = self.range_all.set_checked(active == RangePreset::All);
    }
}

pub fn normalize_menu_range(range: RangePreset) -> RangePreset {
    match range {
        RangePreset::Today
        | RangePreset::SevenDays
        | RangePreset::ThirtyDays
        | RangePreset::All => range,
        RangePreset::Custom => RangePreset::Today,
    }
}

pub fn format_usd(cost: f64) -> String {
    if !cost.is_finite() {
        return "$0.00".to_string();
    }

    let negative = cost < 0.0;
    let total_cents = (cost.abs() * 100.0).round() as u64;
    let whole = total_cents / 100;
    let frac = total_cents % 100;

    let whole_str = format_with_commas(whole);
    if negative && total_cents > 0 {
        format!("-${whole_str}.{frac:02}")
    } else {
        format!("${whole_str}.{frac:02}")
    }
}

fn format_with_commas(value: u64) -> String {
    let digits = value.to_string();
    let len = digits.len();
    if len <= 3 {
        return digits;
    }

    let mut out = String::with_capacity(len + (len - 1) / 3);
    for (idx, ch) in digits.chars().enumerate() {
        if idx > 0 && (len - idx).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

pub fn format_compact_tokens(tokens: u64) -> String {
    if tokens < 1_000 {
        return tokens.to_string();
    }

    if tokens < 1_000_000 {
        let scaled = (tokens as f64 / 1_000.0 * 10.0).round() / 10.0;
        if scaled >= 1000.0 {
            return "1.0M".to_string();
        }
        return format!("{scaled:.1}K");
    }

    if tokens < 1_000_000_000 {
        let scaled = (tokens as f64 / 1_000_000.0 * 10.0).round() / 10.0;
        if scaled >= 1000.0 {
            return "1.0B".to_string();
        }
        return format!("{scaled:.1}M");
    }

    let scaled = (tokens as f64 / 1_000_000_000.0 * 10.0).round() / 10.0;
    format!("{scaled:.1}B")
}

pub fn format_tray_title(display_mode: &str, cost: f64, tokens: u64) -> String {
    match display_mode {
        "icon" => String::new(),
        "tokens" => format_compact_tokens(tokens),
        "cost" => format_usd(cost),
        _ => format_usd(cost),
    }
}

pub fn format_windows_tooltip(cost: f64, tokens: u64) -> String {
    format!(
        "tokscale · {} · {} tokens",
        format_usd(cost),
        format_compact_tokens(tokens)
    )
}

struct LoadedTrayIcon {
    image: Image<'static>,
    is_template: bool,
}

fn load_tray_icon(app: &AppHandle<Wry>) -> Option<LoadedTrayIcon> {
    let mut candidate_roots: Vec<PathBuf> = Vec::new();
    if let Ok(res_dir) = app.path().resource_dir() {
        candidate_roots.push(res_dir);
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidate_roots.push(cwd.clone());
        candidate_roots.push(cwd.join("src-tauri"));
    }

    #[cfg(target_os = "macos")]
    let rel_candidates = [("icons/tray/iconTemplate.png", true)];
    #[cfg(target_os = "windows")]
    let rel_candidates = [
        ("icons/tray/win-dark-16.png", false),
        ("icons/tray/win-light-16.png", false),
    ];
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let rel_candidates = [
        ("icons/tray/linux-symbolic.png", false),
        ("icons/tray/iconTemplate.png", false),
    ];

    for root in &candidate_roots {
        for (rel, is_template) in rel_candidates {
            let path = root.join(rel);
            if path.is_file() {
                if let Ok(image) = Image::from_path(&path) {
                    return Some(LoadedTrayIcon {
                        image: image.to_owned(),
                        is_template,
                    });
                }
            }
        }
    }

    if let Some(default_icon) = app.default_window_icon() {
        return Some(LoadedTrayIcon {
            image: default_icon.clone().to_owned(),
            is_template: false,
        });
    }

    Image::from_bytes(FALLBACK_ICON_PNG)
        .ok()
        .map(|image| LoadedTrayIcon {
            image: image.to_owned(),
            is_template: false,
        })
}

pub fn setup_tray(
    app: &AppHandle<Wry>,
    state: &Arc<RwLock<ShellState>>,
    hooks: &Arc<RwLock<ShellHooks>>,
    on_quit: Arc<dyn Fn() + Send + Sync>,
) -> tauri::Result<TrayHandle> {
    let initial_range = normalize_menu_range(read_state(state).active_range);

    let open_dashboard_item =
        MenuItemBuilder::with_id(MENU_OPEN_DASHBOARD, "Open dashboard").build(app)?;
    let sync_now_item = MenuItemBuilder::with_id(MENU_SYNC_NOW, "Sync now").build(app)?;

    let range_today = CheckMenuItemBuilder::with_id(MENU_RANGE_TODAY, "Today")
        .checked(initial_range == RangePreset::Today)
        .build(app)?;
    let range_7d = CheckMenuItemBuilder::with_id(MENU_RANGE_7D, "Last 7 days")
        .checked(initial_range == RangePreset::SevenDays)
        .build(app)?;
    let range_30d = CheckMenuItemBuilder::with_id(MENU_RANGE_30D, "Last 30 days")
        .checked(initial_range == RangePreset::ThirtyDays)
        .build(app)?;
    let range_all = CheckMenuItemBuilder::with_id(MENU_RANGE_ALL, "All time")
        .checked(initial_range == RangePreset::All)
        .build(app)?;

    let range_submenu = SubmenuBuilder::new(app, "Range")
        .item(&range_today)
        .item(&range_7d)
        .item(&range_30d)
        .item(&range_all)
        .build()?;

    let quit_item = MenuItemBuilder::with_id(MENU_QUIT, "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&open_dashboard_item)
        .item(&sync_now_item)
        .item(&range_submenu)
        .separator()
        .item(&quit_item)
        .build()?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ICON_ID)
        .menu(&menu)
        .show_menu_on_left_click(false);

    if let Some(loaded) = load_tray_icon(app) {
        builder = builder.icon(loaded.image);
        #[cfg(target_os = "macos")]
        if loaded.is_template {
            builder = builder.icon_as_template(true);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = loaded.is_template;
    }

    let menu_state = Arc::clone(state);
    let menu_hooks = Arc::clone(hooks);
    let r_today = range_today.clone();
    let r_7d = range_7d.clone();
    let r_30d = range_30d.clone();
    let r_all = range_all.clone();

    builder = builder.on_menu_event(move |app, event| {
        let id = event.id().as_ref();
        match id {
            MENU_OPEN_DASHBOARD => {
                popover::dismiss_popover(app, &menu_state);
                let active_range = read_state(&menu_state).active_range;
                let handoff = DashboardHandoff {
                    tab: DashboardTab::Overview,
                    range: Some(active_range),
                    query: None,
                };
                let _ = dashboard::open_dashboard(app, &menu_state, handoff);
            }
            MENU_SYNC_NOW => {
                let cb = menu_hooks
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .on_sync_requested
                    .clone();
                if let Some(cb) = cb {
                    cb(SyncMode::FullManual);
                }
            }
            MENU_RANGE_TODAY | MENU_RANGE_7D | MENU_RANGE_30D | MENU_RANGE_ALL => {
                let next_range = match id {
                    MENU_RANGE_TODAY => RangePreset::Today,
                    MENU_RANGE_7D => RangePreset::SevenDays,
                    MENU_RANGE_30D => RangePreset::ThirtyDays,
                    MENU_RANGE_ALL => RangePreset::All,
                    _ => RangePreset::Today,
                };

                {
                    let mut guard = write_state(&menu_state);
                    guard.active_range = next_range;
                }

                let _ = r_today.set_checked(next_range == RangePreset::Today);
                let _ = r_7d.set_checked(next_range == RangePreset::SevenDays);
                let _ = r_30d.set_checked(next_range == RangePreset::ThirtyDays);
                let _ = r_all.set_checked(next_range == RangePreset::All);

                let cb = menu_hooks
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .on_range_changed
                    .clone();
                if let Some(cb) = cb {
                    cb(next_range);
                }
            }
            MENU_QUIT => {
                on_quit();
                let cb = menu_hooks
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .on_quit_requested
                    .clone();
                if let Some(cb) = cb {
                    cb();
                }
                app.exit(0);
            }
            _ => {}
        }
    });

    let click_state = Arc::clone(state);
    builder = builder.on_tray_icon_event(move |tray, event| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            rect,
            ..
        } = event
        {
            popover::toggle_popover(tray.app_handle(), &click_state, Some(rect));
        }
    });

    let icon = builder.build(app)?;
    let handle = TrayHandle {
        icon,
        range_today,
        range_7d,
        range_30d,
        range_all,
    };

    let (display_mode, snapshot) = {
        let mut guard = write_state(state);
        guard.tray_alive = true;
        (
            guard.preferences.menubar_display.clone(),
            guard.latest_readout.clone(),
        )
    };
    handle.apply_readout(&display_mode, &snapshot);

    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readout_formatting_cost_tokens_and_icon_mode() {
        assert_eq!(format_tray_title("cost", 48.12, 61_400_000), "$48.12");
        assert_eq!(format_tray_title("cost", 1234.5, 0), "$1,234.50");
        assert_eq!(format_tray_title("cost", 999.999, 0), "$1,000.00");
        assert_eq!(format_tray_title("cost", 0.0, 0), "$0.00");
        assert_eq!(format_tray_title("cost", f64::NAN, 0), "$0.00");

        assert_eq!(format_tray_title("tokens", 48.12, 61_400_000), "61.4M");
        assert_eq!(format_tray_title("tokens", 0.0, 850), "850");
        assert_eq!(format_tray_title("tokens", 0.0, 1_500), "1.5K");
        assert_eq!(format_tray_title("tokens", 0.0, 999_999), "1.0M");
        assert_eq!(format_tray_title("tokens", 0.0, 1_250_000_000), "1.3B");

        assert_eq!(format_tray_title("icon", 48.12, 61_400_000), "");
    }

    #[test]
    fn windows_tooltip_formatting_matches_contract() {
        assert_eq!(
            format_windows_tooltip(48.12, 61_400_000),
            "tokscale · $48.12 · 61.4M tokens"
        );
    }

    #[test]
    fn menu_range_normalization_maps_custom_to_today() {
        assert_eq!(normalize_menu_range(RangePreset::Today), RangePreset::Today);
        assert_eq!(
            normalize_menu_range(RangePreset::SevenDays),
            RangePreset::SevenDays
        );
        assert_eq!(
            normalize_menu_range(RangePreset::ThirtyDays),
            RangePreset::ThirtyDays
        );
        assert_eq!(normalize_menu_range(RangePreset::All), RangePreset::All);
        assert_eq!(normalize_menu_range(RangePreset::Custom), RangePreset::Today);
    }
}
