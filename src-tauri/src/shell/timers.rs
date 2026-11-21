use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, watch};

use crate::contracts::SyncMode;
use crate::shell::ShellHooks;

pub const WATCHER_DEBOUNCE: Duration = Duration::from_secs(5);
pub const OFFLINE_REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);
pub const QUOTA_REFRESH_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone)]
pub struct DebounceGate {
    debounce: Duration,
    last_event: Option<Instant>,
}

impl DebounceGate {
    pub fn new(debounce: Duration) -> Self {
        Self {
            debounce,
            last_event: None,
        }
    }

    pub fn record_event(&mut self, at: Instant) {
        self.last_event = Some(at);
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.last_event.map(|ts| ts + self.debounce)
    }

    pub fn poll_fire(&mut self, now: Instant) -> bool {
        let Some(last) = self.last_event else {
            return false;
        };
        if now.saturating_duration_since(last) >= self.debounce {
            self.last_event = None;
            true
        } else {
            false
        }
    }
}

pub fn should_trigger_watcher(kind: &EventKind) -> bool {
    !matches!(kind, EventKind::Access(_))
}

pub fn filter_existing_watch_dirs(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    for path in paths {
        if path.exists() && !unique.contains(&path) {
            unique.push(path);
        }
    }
    unique
}

pub struct ShellTimers {
    shutdown_tx: watch::Sender<bool>,
    watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
    watched_paths: Arc<Mutex<Vec<PathBuf>>>,
}

impl ShellTimers {
    pub fn start(watch_paths: Vec<PathBuf>, hooks: Arc<RwLock<ShellHooks>>) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let watcher = build_watcher(event_tx);
        let watcher_arc = Arc::new(Mutex::new(watcher));
        let watched_paths_arc = Arc::new(Mutex::new(Vec::new()));

        let timers = Self {
            shutdown_tx,
            watcher: watcher_arc,
            watched_paths: watched_paths_arc,
        };

        timers.set_watch_paths(watch_paths);

        spawn_watcher_debounce_task(event_rx, shutdown_rx.clone(), Arc::clone(&hooks));
        spawn_periodic_task(OFFLINE_REFRESH_INTERVAL, shutdown_rx.clone(), {
            let hooks = Arc::clone(&hooks);
            move || {
                let cb = hooks
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .on_sync_requested
                    .clone();
                if let Some(cb) = cb {
                    cb(SyncMode::LocalOffline);
                }
            }
        });
        spawn_periodic_task(QUOTA_REFRESH_INTERVAL, shutdown_rx, move || {
            let cb = hooks
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .on_quota_refresh
                .clone();
            if let Some(cb) = cb {
                cb();
            }
        });

        timers
    }

    pub fn set_watch_paths(&self, paths: Vec<PathBuf>) {
        let next_paths = filter_existing_watch_dirs(paths);
        let mut watcher_guard = self.watcher.lock().unwrap_or_else(|e| e.into_inner());
        let mut paths_guard = self.watched_paths.lock().unwrap_or_else(|e| e.into_inner());

        if let Some(watcher) = watcher_guard.as_mut() {
            for old_path in paths_guard.iter() {
                let _ = watcher.unwatch(old_path);
            }
            let mut active = Vec::new();
            for path in next_paths {
                if watcher.watch(&path, RecursiveMode::Recursive).is_ok() {
                    active.push(path);
                }
            }
            *paths_guard = active;
        } else {
            *paths_guard = next_paths;
        }
    }

    pub fn is_stopped(&self) -> bool {
        *self.shutdown_tx.borrow()
    }

    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
        let mut watcher_guard = self.watcher.lock().unwrap_or_else(|e| e.into_inner());
        *watcher_guard = None;
    }
}

impl Drop for ShellTimers {
    fn drop(&mut self) {
        self.stop();
    }
}

fn build_watcher(event_tx: mpsc::UnboundedSender<()>) -> Option<RecommendedWatcher> {
    notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(event) = res {
            if should_trigger_watcher(&event.kind) {
                let _ = event_tx.send(());
            }
        }
    })
    .ok()
}

fn spawn_watcher_debounce_task(
    mut event_rx: mpsc::UnboundedReceiver<()>,
    mut shutdown_rx: watch::Receiver<bool>,
    hooks: Arc<RwLock<ShellHooks>>,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        break;
                    }
                }
                ev = event_rx.recv() => {
                    if ev.is_none() {
                        break;
                    }
                }
            }

            let mut deadline = tokio::time::Instant::now() + WATCHER_DEBOUNCE;
            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            return;
                        }
                    }
                    ev = event_rx.recv() => {
                        match ev {
                            Some(()) => {
                                deadline = tokio::time::Instant::now() + WATCHER_DEBOUNCE;
                            }
                            None => return,
                        }
                    }
                    _ = tokio::time::sleep_until(deadline) => {
                        let cb = hooks
                            .read()
                            .unwrap_or_else(|e| e.into_inner())
                            .on_sync_requested
                            .clone();
                        if let Some(cb) = cb {
                            cb(SyncMode::LocalOffline);
                        }
                        break;
                    }
                }
            }
        }
    });
}

fn spawn_periodic_task<F>(interval: Duration, mut shutdown_rx: watch::Receiver<bool>, on_tick: F)
where
    F: Fn() + Send + Sync + 'static,
{
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        break;
                    }
                }
                _ = tokio::time::sleep(interval) => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                    on_tick();
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debounce_gate_coalesces_burst_into_single_refresh() {
        let mut gate = DebounceGate::new(WATCHER_DEBOUNCE);
        let t0 = Instant::now();

        gate.record_event(t0);
        gate.record_event(t0 + Duration::from_secs(2));
        gate.record_event(t0 + Duration::from_secs(4));

        // 6 seconds after t0 is only 2 seconds after the last burst event at t0+4s.
        assert!(!gate.poll_fire(t0 + Duration::from_secs(6)));
        assert_eq!(gate.next_deadline(), Some(t0 + Duration::from_secs(9)));

        // At t0 + 9s (5s after last event), it fires once and clears.
        assert!(gate.poll_fire(t0 + Duration::from_secs(9)));
        assert!(!gate.poll_fire(t0 + Duration::from_secs(15)));
        assert_eq!(gate.next_deadline(), None);
    }

    #[test]
    fn watcher_ignores_access_events_and_filters_missing_dirs() {
        assert!(!should_trigger_watcher(&EventKind::Access(
            notify::event::AccessKind::Read
        )));
        assert!(should_trigger_watcher(&EventKind::Modify(
            notify::event::ModifyKind::Data(notify::event::DataChange::Content)
        )));

        let tmp = tempfile::TempDir::new().unwrap();
        let existing = tmp.path().to_path_buf();
        let missing = tmp.path().join("does-not-exist");
        let filtered = filter_existing_watch_dirs(vec![existing.clone(), missing, existing.clone()]);
        assert_eq!(filtered, vec![existing]);
    }

    #[tokio::test]
    async fn timers_start_and_cancel_cleanly_on_stop() {
        let tmp = tempfile::TempDir::new().unwrap();
        let hooks = Arc::new(RwLock::new(ShellHooks::default()));
        let timers = ShellTimers::start(vec![tmp.path().to_path_buf()], hooks);

        assert!(!timers.is_stopped());
        timers.stop();
        assert!(timers.is_stopped());
    }
}
