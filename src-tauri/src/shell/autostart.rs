use tauri::{AppHandle, Manager, Runtime, Wry};
use tauri_plugin_autostart::AutoLaunchManager;

use crate::shell::ShellError;

pub fn build_plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    let builder = tauri_plugin_autostart::Builder::new();
    #[cfg(target_os = "macos")]
    let builder = builder.macos_launcher(tauri_plugin_autostart::MacosLauncher::LaunchAgent);
    builder.build()
}

pub fn sync_autostart(app: &AppHandle<Wry>, launch_at_login: bool) -> Result<(), ShellError> {
    let Some(manager) = app.try_state::<AutoLaunchManager>() else {
        return Ok(());
    };

    let currently_enabled = manager
        .is_enabled()
        .map_err(|err| ShellError::Autostart(err.to_string()))?;

    if launch_at_login && !currently_enabled {
        manager
            .enable()
            .map_err(|err| ShellError::Autostart(err.to_string()))?;
    } else if !launch_at_login && currently_enabled {
        manager
            .disable()
            .map_err(|err| ShellError::Autostart(err.to_string()))?;
    }

    Ok(())
}
