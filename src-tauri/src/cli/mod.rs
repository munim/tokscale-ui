//! Tokscale CLI locator, serialized subprocess queue, and global installer.

pub mod commands;
pub mod error;
pub mod install;
pub mod locate;
pub mod providers;
pub mod queue;

use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::contracts::{CliInstallResult, CliResolutionInfo, CompanionPreferences, SourceWarning};

pub use commands::*;
pub use error::CliError;
pub use locate::{CliTarget, ProbeEnv};
pub use providers::PROVIDER_SYNC;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedResolution {
    resolved_via: String,
    binary_path: Option<String>,
    version: Option<String>,
}

/// Facade for locating, invoking, syncing, and installing the `tokscale` CLI.
#[derive(Clone)]
pub struct CliService {
    prefs: Arc<RwLock<CompanionPreferences>>,
    env: ProbeEnv,
    queue: queue::CliQueue,
    cached_resolution: Arc<RwLock<Option<CachedResolution>>>,
}

impl CliService {
    /// Construct a `CliService` using the process environment.
    pub fn new(prefs: Arc<RwLock<CompanionPreferences>>) -> Self {
        Self::with_env(prefs, ProbeEnv::from_process())
    }

    /// Construct a `CliService` with an explicit `ProbeEnv` (useful for hermetic tests).
    pub fn with_env(prefs: Arc<RwLock<CompanionPreferences>>, env: ProbeEnv) -> Self {
        Self {
            prefs,
            env,
            queue: queue::CliQueue::spawn(),
            cached_resolution: Arc::new(RwLock::new(None)),
        }
    }

    /// Clear cached resolution/version metadata so the next `resolve()` re-queries `--version`.
    pub fn invalidate_resolution(&self) {
        let mut cache = self
            .cached_resolution
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *cache = None;
    }

    /// Probe the CLI binary (custom path -> PATH -> known probes -> bunx -> npx -> none)
    /// and cache `--version` until the resolved binary path changes.
    pub async fn resolve(&self) -> CliResolutionInfo {
        let target = self.locate_target();
        let resolved_via = target.resolved_via().to_string();
        let binary_path = target.binary_path();
        let fallback = target.fallback();

        if matches!(target, CliTarget::None) {
            self.invalidate_resolution();
            return CliResolutionInfo {
                resolved_via,
                binary_path,
                version: None,
                fallback,
            };
        }

        if let Some(cached) = self
            .cached_resolution
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .filter(|c| c.resolved_via == resolved_via && c.binary_path == binary_path)
        {
            return CliResolutionInfo {
                resolved_via,
                binary_path,
                version: cached.version.clone(),
                fallback,
            };
        }

        let version = match self
            .queue
            .run_tokscale(
                target,
                commands::version(),
                commands::TIMEOUT_SHORT,
                self.env.clone(),
            )
            .await
        {
            Ok(out) => parse_version_output(&out),
            Err(_) => None,
        };

        {
            let mut cache = self
                .cached_resolution
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *cache = Some(CachedResolution {
                resolved_via: resolved_via.clone(),
                binary_path: binary_path.clone(),
                version: version.clone(),
            });
        }

        CliResolutionInfo {
            resolved_via,
            binary_path,
            version,
            fallback,
        }
    }

    /// Run an arbitrary `tokscale` command through the serialized queue and return raw stdout.
    pub async fn run_raw(&self, argv: Vec<String>, timeout: Duration) -> Result<String, CliError> {
        let target = self.locate_target();
        self.queue
            .run_tokscale(target, argv, timeout, self.env.clone())
            .await
    }

    /// Run a `tokscale` command through the serialized queue and deserialize JSON stdout.
    pub async fn run_json<T: DeserializeOwned>(
        &self,
        argv: Vec<String>,
        timeout: Duration,
    ) -> Result<T, CliError> {
        let raw = self.run_raw(argv, timeout).await?;
        serde_json::from_str::<T>(&raw).map_err(|err| CliError::InvalidJson(err.to_string()))
    }

    /// Run `tokscale <provider> sync` for all credentialed API-backed providers.
    pub async fn sync_providers(&self) -> Vec<SourceWarning> {
        providers::sync_all_credentialed(self).await
    }

    /// Run `tokscale <provider> sync` for a single API-backed provider.
    pub async fn sync_single_provider(&self, provider: &str) -> Result<(), SourceWarning> {
        providers::sync_single_provider(self, provider).await
    }

    /// Filesystem-only check for whether a provider's credential file exists.
    pub fn is_provider_credentialed(&self, provider: &str) -> bool {
        providers::is_provider_credentialed(provider, &self.env)
    }

    /// Returns `true` if `provider` is an API-backed provider in `PROVIDER_SYNC`.
    pub fn is_api_backed_provider(&self, provider: &str) -> bool {
        providers::is_api_backed_provider(provider)
    }

    /// Install or upgrade `tokscale` globally via `bun` or `npm`, then refresh cached version.
    pub async fn install_or_upgrade(&self) -> CliInstallResult {
        install::install_or_upgrade(self).await
    }

    pub(crate) fn env(&self) -> &ProbeEnv {
        &self.env
    }

    pub(crate) fn queue(&self) -> &queue::CliQueue {
        &self.queue
    }

    fn locate_target(&self) -> CliTarget {
        let custom_path = {
            let guard = self
                .prefs
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.custom_tokscale_path.clone()
        };
        locate::locate_tokscale(custom_path.as_deref(), &self.env)
    }
}

fn parse_version_output(raw: &str) -> Option<String> {
    let line = raw.lines().map(str::trim).find(|l| !l.is_empty())?;
    let stripped = line
        .strip_prefix("tokscale ")
        .or_else(|| line.strip_prefix("tokscale@"))
        .unwrap_or(line)
        .trim();
    let stripped = stripped.strip_prefix('v').unwrap_or(stripped).trim();
    if stripped.is_empty() {
        None
    } else {
        Some(stripped.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    use super::*;
    use crate::cli::locate::write_test_script;

    #[cfg(unix)]
    #[tokio::test]
    async fn resolve_caches_version_until_binary_path_changes() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let bin_dir = tmp.path().join("bin");
        let count_file = tmp.path().join("version-calls.log");
        let bin1 = bin_dir.join("tokscale");
        let bin2 = tmp.path().join("custom-tokscale");

        write_test_script(
            &bin1,
            &format!(
                "#!/bin/sh\necho call >> \"{}\"\necho 'tokscale 4.23.0'\n",
                count_file.display()
            ),
        );
        write_test_script(
            &bin2,
            &format!(
                "#!/bin/sh\necho call >> \"{}\"\necho 'tokscale 4.24.0'\n",
                count_file.display()
            ),
        );

        let env = ProbeEnv::isolated(home).with_path([bin_dir]);
        let prefs = Arc::new(RwLock::new(CompanionPreferences::default()));
        let service = CliService::with_env(Arc::clone(&prefs), env);

        let first = service.resolve().await;
        let second = service.resolve().await;
        assert_eq!(first.version.as_deref(), Some("4.23.0"));
        assert_eq!(second.version.as_deref(), Some("4.23.0"));
        assert_eq!(fs::read_to_string(&count_file).unwrap().lines().count(), 1);

        {
            let mut guard = prefs.write().unwrap();
            guard.custom_tokscale_path = Some(bin2.to_string_lossy().into_owned());
        }

        let third = service.resolve().await;
        assert_eq!(third.resolved_via, "custom");
        assert_eq!(third.version.as_deref(), Some("4.24.0"));
        assert_eq!(fs::read_to_string(&count_file).unwrap().lines().count(), 2);
    }
}
