use std::sync::{Arc, RwLock};

use tauri::{AppHandle, Manager, Runtime, Wry};
use tauri_plugin_global_shortcut::{GlobalShortcut, ShortcutState};

use crate::contracts::SourceWarning;
use crate::shell::{popover, write_state, ShellState};

pub const SHORTCUT_WARNING_SOURCE: &str = "shortcut";

pub fn build_plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new().build()
}

pub fn normalize_shortcut(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn apply_global_shortcut(
    app: &AppHandle<Wry>,
    state: &Arc<RwLock<ShellState>>,
    raw_shortcut: Option<&str>,
) -> Option<SourceWarning> {
    {
        let mut guard = write_state(state);
        guard
            .warnings
            .retain(|warning| warning.source != SHORTCUT_WARNING_SOURCE);
    }

    let normalized = normalize_shortcut(raw_shortcut);
    let gs = app.try_state::<GlobalShortcut<Wry>>()?;

    let _ = gs.unregister_all();

    let accel = normalized?;

    let toggle_state = Arc::clone(state);
    if let Err(err) = gs.on_shortcut(accel.as_str(), move |app_handle, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            popover::toggle_popover(app_handle, &toggle_state, None);
        }
    }) {
        let warning = SourceWarning {
            source: SHORTCUT_WARNING_SOURCE.to_string(),
            message: format!("Failed to register global shortcut '{accel}': {err}"),
        };
        write_state(state).warnings.push(warning.clone());
        return Some(warning);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_shortcut_trims_and_ignores_blank_values() {
        assert_eq!(normalize_shortcut(None), None);
        assert_eq!(normalize_shortcut(Some("")), None);
        assert_eq!(normalize_shortcut(Some("   \t  ")), None);
        assert_eq!(
            normalize_shortcut(Some("  CmdOrCtrl+Shift+T  ")),
            Some("CmdOrCtrl+Shift+T".to_string())
        );
    }
}
