use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Wry};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::contracts::{
    events, CompanionPreferences, UpdateCapability, UpdatePhase, UpdateStatusPayload,
};
use crate::shell::ShellController;

pub const RELEASE_LATEST_URL: &str = "https://github.com/munim/tokscale-ui/releases/latest";
const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedInstall {
    pub capability: UpdateCapability,
    pub install_method: String,
    pub fallback_hint: Option<String>,
}

pub fn current_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn detect_update_capability() -> DetectedInstall {
    let exe_path = std::env::current_exe().ok();
    let appimage_env = std::env::var_os("APPIMAGE").map(PathBuf::from);

    #[cfg(target_os = "macos")]
    let os = "macos";
    #[cfg(target_os = "windows")]
    let os = "windows";
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let os = "linux";

    detect_update_capability_for(os, exe_path.as_deref(), appimage_env.as_deref(), &is_path_writable)
}

pub fn detect_update_capability_for<F>(
    os: &str,
    exe_path: Option<&Path>,
    appimage_env: Option<&Path>,
    is_writable: &F,
) -> DetectedInstall
where
    F: Fn(&Path) -> bool,
{
    let exe_str = exe_path
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    match os {
        "macos" => {
            if exe_str.contains("/Volumes/") {
                return DetectedInstall {
                    capability: UpdateCapability::NotifyOnly,
                    install_method: "dmg-mounted".to_string(),
                    fallback_hint: Some(
                        "Move Tokscale UI.app to /Applications before updating, or run: brew upgrade --cask munim/tap/tokscale-ui"
                            .to_string(),
                    ),
                };
            }
            if exe_str.contains("/AppTranslocation/") {
                return DetectedInstall {
                    capability: UpdateCapability::NotifyOnly,
                    install_method: "translocated".to_string(),
                    fallback_hint: Some(
                        "Move Tokscale UI.app to /Applications and run: xattr -dr com.apple.quarantine \"/Applications/Tokscale UI.app\""
                            .to_string(),
                    ),
                };
            }
            if let Some(app_bundle) = exe_path.and_then(find_macos_app_bundle) {
                let contents_dir = app_bundle.join("Contents");
                let probe_target = if contents_dir.exists() {
                    contents_dir.as_path()
                } else {
                    app_bundle.as_path()
                };
                let parent_writable = app_bundle.parent().is_none_or(is_writable);
                if is_writable(probe_target) && parent_writable {
                    return DetectedInstall {
                        capability: UpdateCapability::SelfUpdate,
                        install_method: "macos-bundle".to_string(),
                        fallback_hint: None,
                    };
                }
                return DetectedInstall {
                    capability: UpdateCapability::NotifyOnly,
                    install_method: "macos-readonly".to_string(),
                    fallback_hint: Some(
                        "brew upgrade --cask munim/tap/tokscale-ui".to_string(),
                    ),
                };
            }
            DetectedInstall {
                capability: UpdateCapability::SelfUpdate,
                install_method: "dev-build".to_string(),
                fallback_hint: None,
            }
        }
        "windows" => {
            let lower = exe_str.to_ascii_lowercase();
            if lower.contains("\\scoop\\apps\\") || lower.contains("/scoop/apps/") {
                return DetectedInstall {
                    capability: UpdateCapability::NotifyOnly,
                    install_method: "scoop".to_string(),
                    fallback_hint: Some("scoop update tokscale-ui".to_string()),
                };
            }
            if let Some(parent) = exe_path.and_then(|p| p.parent()) {
                if !is_writable(parent) {
                    return DetectedInstall {
                        capability: UpdateCapability::NotifyOnly,
                        install_method: "windows-readonly".to_string(),
                        fallback_hint: Some("winget upgrade munim.TokscaleUI".to_string()),
                    };
                }
            }
            DetectedInstall {
                capability: UpdateCapability::SelfUpdate,
                install_method: if lower.contains("\\target\\") || lower.contains("/target/") {
                    "dev-build".to_string()
                } else {
                    "windows-nsis".to_string()
                },
                fallback_hint: None,
            }
        }
        _ => {
            if let Some(appimage_path) = appimage_env.filter(|p| !p.as_os_str().is_empty()) {
                let parent_writable = appimage_path.parent().is_none_or(is_writable);
                if is_writable(appimage_path) && parent_writable {
                    return DetectedInstall {
                        capability: UpdateCapability::SelfUpdate,
                        install_method: "appimage".to_string(),
                        fallback_hint: None,
                    };
                }
                return DetectedInstall {
                    capability: UpdateCapability::NotifyOnly,
                    install_method: "appimage-readonly".to_string(),
                    fallback_hint: Some(
                        "curl -fsSL https://raw.githubusercontent.com/munim/tokscale-ui/main/scripts/install.sh | sh"
                            .to_string(),
                    ),
                };
            }
            if exe_str.starts_with("/usr/")
                || exe_str.starts_with("/opt/")
                || exe_str.starts_with("/snap/")
            {
                return DetectedInstall {
                    capability: UpdateCapability::NotifyOnly,
                    install_method: "linux-package".to_string(),
                    fallback_hint: Some(
                        "curl -fsSL https://raw.githubusercontent.com/munim/tokscale-ui/main/scripts/install.sh | sh"
                            .to_string(),
                    ),
                };
            }
            if exe_str.contains("/target/") {
                return DetectedInstall {
                    capability: UpdateCapability::SelfUpdate,
                    install_method: "dev-build".to_string(),
                    fallback_hint: None,
                };
            }
            DetectedInstall {
                capability: UpdateCapability::NotifyOnly,
                install_method: "linux-binary".to_string(),
                fallback_hint: Some(
                    "curl -fsSL https://raw.githubusercontent.com/munim/tokscale-ui/main/scripts/install.sh | sh"
                        .to_string(),
                ),
            }
        }
    }
}

fn find_macos_app_bundle(exe_path: &Path) -> Option<PathBuf> {
    for ancestor in exe_path.ancestors() {
        if ancestor.extension().and_then(|s| s.to_str()) == Some("app") {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn is_path_writable(path: &Path) -> bool {
    let dir = if path.is_dir() {
        path
    } else if path.is_file() {
        let Ok(meta) = fs::metadata(path) else {
            return false;
        };
        if meta.permissions().readonly() {
            return false;
        }
        match path.parent() {
            Some(parent) => parent,
            None => return false,
        }
    } else {
        return false;
    };

    let probe_name = format!(
        ".tokscale-ui-write-probe-{}",
        std::process::id()
    );
    let probe_path = dir.join(probe_name);
    match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe_path)
    {
        Ok(file) => {
            drop(file);
            let _ = fs::remove_file(&probe_path);
            true
        }
        Err(_) => false,
    }
}

pub struct UpdateService {
    status: Arc<RwLock<UpdateStatusPayload>>,
    pending_update: Arc<tokio::sync::Mutex<Option<Update>>>,
    op_lock: Arc<tokio::sync::Mutex<()>>,
    notified_versions: Arc<Mutex<HashSet<String>>>,
}

impl Default for UpdateService {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateService {
    pub fn new() -> Self {
        let detected = detect_update_capability();
        Self {
            status: Arc::new(RwLock::new(UpdateStatusPayload {
                current_version: current_app_version().to_string(),
                phase: UpdatePhase::Idle,
                capability: detected.capability,
                install_method: detected.install_method,
                available_version: None,
                release_notes: None,
                pub_date: None,
                downloaded_bytes: None,
                total_bytes: None,
                error: None,
                last_checked_at: None,
                fallback_hint: detected.fallback_hint,
                release_url: Some(RELEASE_LATEST_URL.to_string()),
            })),
            pending_update: Arc::new(tokio::sync::Mutex::new(None)),
            op_lock: Arc::new(tokio::sync::Mutex::new(())),
            notified_versions: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn status(&self) -> UpdateStatusPayload {
        self.status
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn emit_status(&self, app: &AppHandle<Wry>, payload: &UpdateStatusPayload) {
        let _ = app.emit(events::UPDATE_STATUS, payload);
    }

    fn update_status<F>(&self, app: &AppHandle<Wry>, mutator: F) -> UpdateStatusPayload
    where
        F: FnOnce(&mut UpdateStatusPayload),
    {
        let next = {
            let mut guard = self.status.write().unwrap_or_else(|e| e.into_inner());
            mutator(&mut guard);
            guard.clone()
        };
        self.emit_status(app, &next);
        next
    }

    pub async fn check(
        &self,
        app: &AppHandle<Wry>,
        manual: bool,
        prefs: &CompanionPreferences,
        shell: Option<&ShellController>,
    ) -> Result<UpdateStatusPayload, String> {
        let _guard = self.op_lock.lock().await;

        {
            let current = self.status();
            if matches!(
                current.phase,
                UpdatePhase::Downloading | UpdatePhase::Ready
            ) {
                return Ok(current);
            }
        }

        self.update_status(app, |s| {
            s.phase = UpdatePhase::Checking;
            s.error = None;
        });

        let updater = match app.updater_builder().build() {
            Ok(u) => u,
            Err(err) => {
                let msg = err.to_string();
                let next = self.update_status(app, |s| {
                    s.phase = if manual {
                        UpdatePhase::Error
                    } else {
                        UpdatePhase::Idle
                    };
                    s.error = if manual { Some(msg.clone()) } else { None };
                });
                return if manual { Err(msg) } else { Ok(next) };
            }
        };

        let check_result = updater.check().await;
        let checked_at = current_unix_secs();

        match check_result {
            Ok(Some(update)) => {
                let version = update.version.clone();
                let release_notes = update.body.clone();
                let pub_date = update
                    .raw_json
                    .get("pub_date")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                *self.pending_update.lock().await = Some(update);

                let next = self.update_status(app, |s| {
                    s.phase = UpdatePhase::Available;
                    s.available_version = Some(version.clone());
                    s.release_notes = release_notes;
                    s.pub_date = pub_date;
                    s.downloaded_bytes = None;
                    s.total_bytes = None;
                    s.error = None;
                    s.last_checked_at = Some(checked_at);
                });

                if let Some(ctrl) = shell {
                    ctrl.set_update_menu_label(&format!("Update Available (v{version})…"));
                }

                let is_skipped = prefs.skipped_update_version.as_deref() == Some(version.as_str());
                if prefs.notifications_enabled && !is_skipped {
                    let should_notify = self
                        .notified_versions
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(version.clone());
                    if should_notify {
                        dispatch_update_notification(app, &next);
                    }
                }

                Ok(next)
            }
            Ok(None) => {
                *self.pending_update.lock().await = None;
                let next = self.update_status(app, |s| {
                    s.phase = if manual {
                        UpdatePhase::UpToDate
                    } else {
                        UpdatePhase::Idle
                    };
                    s.available_version = None;
                    s.release_notes = None;
                    s.pub_date = None;
                    s.downloaded_bytes = None;
                    s.total_bytes = None;
                    s.error = None;
                    s.last_checked_at = Some(checked_at);
                });

                if let Some(ctrl) = shell {
                    ctrl.set_update_menu_label("Check for Updates…");
                }

                Ok(next)
            }
            Err(err) => {
                let msg = err.to_string();
                let next = self.update_status(app, |s| {
                    s.phase = if manual {
                        UpdatePhase::Error
                    } else {
                        UpdatePhase::Idle
                    };
                    s.error = if manual { Some(msg.clone()) } else { None };
                    s.last_checked_at = Some(checked_at);
                });
                if manual {
                    Err(msg)
                } else {
                    Ok(next)
                }
            }
        }
    }

    pub async fn download_and_install<F>(
        &self,
        app: &AppHandle<Wry>,
        shell: Option<&ShellController>,
        on_before_exit: F,
    ) -> Result<UpdateStatusPayload, String>
    where
        F: FnOnce(),
    {
        let current = self.status();
        if current.capability == UpdateCapability::NotifyOnly {
            return Err(current.fallback_hint.unwrap_or_else(|| {
                "Automatic updates are not supported for this install method.".to_string()
            }));
        }

        let _guard = self.op_lock.lock().await;

        let update = {
            let mut slot = self.pending_update.lock().await;
            if let Some(existing) = slot.take() {
                existing
            } else {
                let updater = app
                    .updater_builder()
                    .build()
                    .map_err(|err| err.to_string())?;
                match updater.check().await.map_err(|err| err.to_string())? {
                    Some(found) => found,
                    None => {
                        let next = self.update_status(app, |s| {
                            s.phase = UpdatePhase::UpToDate;
                            s.available_version = None;
                            s.release_notes = None;
                            s.pub_date = None;
                            s.downloaded_bytes = None;
                            s.total_bytes = None;
                            s.error = None;
                            s.last_checked_at = Some(current_unix_secs());
                        });
                        return Ok(next);
                    }
                }
            }
        };

        let version = update.version.clone();
        let release_notes = update.body.clone();
        let pub_date = update
            .raw_json
            .get("pub_date")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        self.update_status(app, |s| {
            s.phase = UpdatePhase::Downloading;
            s.available_version = Some(version.clone());
            s.release_notes = release_notes;
            s.pub_date = pub_date;
            s.downloaded_bytes = Some(0);
            s.total_bytes = None;
            s.error = None;
        });

        let downloaded = Arc::new(AtomicU64::new(0));
        let total = Arc::new(AtomicU64::new(0));
        let last_emit = Arc::new(Mutex::new(
            Instant::now()
                .checked_sub(PROGRESS_EMIT_INTERVAL)
                .unwrap_or_else(Instant::now),
        ));

        let downloaded_ref = Arc::clone(&downloaded);
        let total_ref = Arc::clone(&total);
        let last_emit_ref = Arc::clone(&last_emit);
        let status_ref = Arc::clone(&self.status);
        let app_ref = app.clone();

        let bytes = match update
            .download(
                move |chunk_len, content_len| {
                    let d = downloaded_ref.fetch_add(chunk_len as u64, Ordering::Relaxed)
                        + chunk_len as u64;
                    if let Some(t) = content_len {
                        total_ref.store(t, Ordering::Relaxed);
                    }
                    let mut guard = last_emit_ref.lock().unwrap_or_else(|e| e.into_inner());
                    if guard.elapsed() >= PROGRESS_EMIT_INTERVAL {
                        *guard = Instant::now();
                        drop(guard);
                        let t_val = total_ref.load(Ordering::Relaxed);
                        let payload = {
                            let mut s = status_ref.write().unwrap_or_else(|e| e.into_inner());
                            s.downloaded_bytes = Some(d);
                            s.total_bytes = if t_val > 0 { Some(t_val) } else { None };
                            s.clone()
                        };
                        let _ = app_ref.emit(events::UPDATE_STATUS, &payload);
                    }
                },
                || {},
            )
            .await
        {
            Ok(bytes) => bytes,
            Err(err) => {
                let msg = err.to_string();
                *self.pending_update.lock().await = Some(update);
                self.update_status(app, |s| {
                    s.phase = UpdatePhase::Error;
                    s.error = Some(msg.clone());
                });
                return Err(msg);
            }
        };

        let final_bytes = downloaded.load(Ordering::Relaxed).max(bytes.len() as u64);
        let final_total = total.load(Ordering::Relaxed).max(final_bytes);
        self.update_status(app, |s| {
            s.downloaded_bytes = Some(final_bytes);
            s.total_bytes = Some(final_total);
        });

        // Stop timers and file watchers cleanly before replacing the bundle (or exiting on Windows).
        on_before_exit();

        if let Err(err) = update.install(&bytes) {
            let msg = err.to_string();
            *self.pending_update.lock().await = Some(update);
            self.update_status(app, |s| {
                s.phase = UpdatePhase::Error;
                s.error = Some(msg.clone());
            });
            return Err(msg);
        }

        // On Windows, `update.install` launches the passive NSIS installer and exits.
        // On macOS/Linux, mark the update ready so the user can restart cleanly.
        let next = self.update_status(app, |s| {
            s.phase = UpdatePhase::Ready;
            s.downloaded_bytes = Some(final_bytes);
            s.total_bytes = Some(final_total);
            s.error = None;
        });

        if let Some(ctrl) = shell {
            ctrl.set_update_menu_label(&format!("Restart to Update (v{version})…"));
        }

        Ok(next)
    }

    pub fn restart_to_apply<F>(&self, app: &AppHandle<Wry>, on_before_exit: F) -> !
    where
        F: FnOnce(),
    {
        on_before_exit();
        app.restart();
    }

    pub fn clear_for_skipped_version(
        &self,
        app: &AppHandle<Wry>,
        version: &str,
        shell: Option<&ShellController>,
    ) -> UpdateStatusPayload {
        let next = self.update_status(app, |s| {
            if s.available_version.as_deref() == Some(version) && s.phase == UpdatePhase::Available
            {
                s.phase = UpdatePhase::Idle;
                s.available_version = None;
                s.release_notes = None;
                s.pub_date = None;
            }
        });
        if let Some(ctrl) = shell {
            if next.phase == UpdatePhase::Idle {
                ctrl.set_update_menu_label("Check for Updates…");
            }
        }
        next
    }
}

fn dispatch_update_notification(app: &AppHandle<Wry>, status: &UpdateStatusPayload) {
    use tauri_plugin_notification::NotificationExt;

    let Some(version) = status.available_version.as_deref() else {
        return;
    };
    let body = match status.capability {
        UpdateCapability::SelfUpdate => {
            "Open Settings or click Update in the menubar popover to install.".to_string()
        }
        UpdateCapability::NotifyOnly => status
            .fallback_hint
            .clone()
            .unwrap_or_else(|| "A new release is available on GitHub.".to_string()),
    };

    let _ = app
        .notification()
        .builder()
        .title(format!("Tokscale UI v{version} Available"))
        .body(body)
        .show();
}

fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_macos_mounted_dmg_and_translocated_as_notify_only() {
        let dmg = detect_update_capability_for(
            "macos",
            Some(Path::new("/Volumes/Tokscale UI/Tokscale UI.app/Contents/MacOS/tokscale-ui")),
            None,
            &|_| true,
        );
        assert_eq!(dmg.capability, UpdateCapability::NotifyOnly);
        assert_eq!(dmg.install_method, "dmg-mounted");
        assert!(dmg.fallback_hint.unwrap().contains("/Applications"));

        let translocated = detect_update_capability_for(
            "macos",
            Some(Path::new(
                "/private/var/folders/xx/AppTranslocation/UUID/d/Tokscale UI.app/Contents/MacOS/tokscale-ui",
            )),
            None,
            &|_| true,
        );
        assert_eq!(translocated.capability, UpdateCapability::NotifyOnly);
        assert_eq!(translocated.install_method, "translocated");
    }

    #[test]
    fn detects_macos_writable_applications_bundle_as_self_update() {
        let installed = detect_update_capability_for(
            "macos",
            Some(Path::new("/Applications/Tokscale UI.app/Contents/MacOS/tokscale-ui")),
            None,
            &|_| true,
        );
        assert_eq!(installed.capability, UpdateCapability::SelfUpdate);
        assert_eq!(installed.install_method, "macos-bundle");
        assert!(installed.fallback_hint.is_none());

        let readonly = detect_update_capability_for(
            "macos",
            Some(Path::new("/Applications/Tokscale UI.app/Contents/MacOS/tokscale-ui")),
            None,
            &|_| false,
        );
        assert_eq!(readonly.capability, UpdateCapability::NotifyOnly);
        assert_eq!(readonly.install_method, "macos-readonly");
    }

    #[test]
    fn detects_windows_scoop_and_readonly_vs_nsis() {
        let scoop = detect_update_capability_for(
            "windows",
            Some(Path::new(r"C:\Users\dev\scoop\apps\tokscale-ui\current\tokscale-ui.exe")),
            None,
            &|_| true,
        );
        assert_eq!(scoop.capability, UpdateCapability::NotifyOnly);
        assert_eq!(scoop.install_method, "scoop");
        assert_eq!(scoop.fallback_hint.as_deref(), Some("scoop update tokscale-ui"));

        let readonly = detect_update_capability_for(
            "windows",
            Some(Path::new(r"C:\Program Files\Tokscale UI\tokscale-ui.exe")),
            None,
            &|_| false,
        );
        assert_eq!(readonly.capability, UpdateCapability::NotifyOnly);
        assert_eq!(readonly.install_method, "windows-readonly");

        let nsis = detect_update_capability_for(
            "windows",
            Some(Path::new(r"C:\Users\dev\AppData\Local\Tokscale UI\tokscale-ui.exe")),
            None,
            &|_| true,
        );
        assert_eq!(nsis.capability, UpdateCapability::SelfUpdate);
        assert_eq!(nsis.install_method, "windows-nsis");
    }

    #[test]
    fn detects_linux_appimage_vs_deb_package() {
        let appimage = detect_update_capability_for(
            "linux",
            Some(Path::new("/tmp/.mount_tokscaXXXX/usr/bin/tokscale-ui")),
            Some(Path::new("/home/dev/.local/bin/tokscale-ui.AppImage")),
            &|_| true,
        );
        assert_eq!(appimage.capability, UpdateCapability::SelfUpdate);
        assert_eq!(appimage.install_method, "appimage");

        let deb = detect_update_capability_for(
            "linux",
            Some(Path::new("/usr/bin/tokscale-ui")),
            None,
            &|_| false,
        );
        assert_eq!(deb.capability, UpdateCapability::NotifyOnly);
        assert_eq!(deb.install_method, "linux-package");
        assert!(deb.fallback_hint.unwrap().contains("install.sh"));
    }
}
