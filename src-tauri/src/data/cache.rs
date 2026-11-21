use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::contracts::SourceWarning;
use crate::data::schema::{TuiCache, UsageCache, EXPECTED_SCHEMA_VERSION};

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum CacheOutcome {
    Fresh(TuiCache),
    Missing,
    Drift { found: u32, expected: u32 },
    Corrupt(String),
}

impl CacheOutcome {
    pub fn fallback_required(&self) -> bool {
        !matches!(self, Self::Fresh(_))
    }

    pub fn warning(&self) -> Option<SourceWarning> {
        match self {
            Self::Fresh(_) => None,
            Self::Missing => Some(SourceWarning {
                source: "cache".to_string(),
                message: "tui-data-cache.json is missing; CLI fallback required".to_string(),
            }),
            Self::Drift { found, expected } => Some(SourceWarning {
                source: "cache".to_string(),
                message: format!(
                    "tui-data-cache.json schema version drift (found {found}, expected {expected}); CLI fallback required"
                ),
            }),
            Self::Corrupt(reason) => Some(SourceWarning {
                source: "cache".to_string(),
                message: format!("tui-data-cache.json unreadable/corrupt: {reason}"),
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum UsageCacheOutcome {
    Fresh(UsageCache),
    Missing,
    Corrupt(String),
}

impl UsageCacheOutcome {
    pub fn warning(&self) -> Option<SourceWarning> {
        match self {
            Self::Fresh(_) | Self::Missing => None,
            Self::Corrupt(reason) => Some(SourceWarning {
                source: "usage_cache".to_string(),
                message: format!("subscription-usage-cache.json unreadable/corrupt: {reason}"),
            }),
        }
    }
}

pub fn tokscale_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TOKSCALE_CONFIG_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("tokscale")
}

pub fn tui_cache_path() -> PathBuf {
    tokscale_config_dir()
        .join("cache")
        .join("tui-data-cache.json")
}

pub fn usage_cache_path() -> PathBuf {
    tokscale_config_dir()
        .join("cache")
        .join("subscription-usage-cache.json")
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemaVersionProbe {
    schema_version: Option<u32>,
}

pub fn parse_tui_cache_bytes(bytes: &[u8]) -> CacheOutcome {
    match serde_json::from_slice::<TuiCache>(bytes) {
        Ok(cache) => {
            if cache.schema_version != EXPECTED_SCHEMA_VERSION {
                CacheOutcome::Drift {
                    found: cache.schema_version,
                    expected: EXPECTED_SCHEMA_VERSION,
                }
            } else {
                CacheOutcome::Fresh(cache)
            }
        }
        Err(err) => {
            if let Ok(probe) = serde_json::from_slice::<SchemaVersionProbe>(bytes) {
                if let Some(found) = probe.schema_version {
                    if found != EXPECTED_SCHEMA_VERSION {
                        return CacheOutcome::Drift {
                            found,
                            expected: EXPECTED_SCHEMA_VERSION,
                        };
                    }
                }
            }
            CacheOutcome::Corrupt(err.to_string())
        }
    }
}

pub fn read_tui_cache(path: &Path) -> CacheOutcome {
    match std::fs::read(path) {
        Ok(bytes) => parse_tui_cache_bytes(&bytes),
        Err(err) if err.kind() == ErrorKind::NotFound => CacheOutcome::Missing,
        Err(err) => CacheOutcome::Corrupt(err.to_string()),
    }
}

pub fn parse_usage_cache_bytes(bytes: &[u8]) -> UsageCacheOutcome {
    match serde_json::from_slice::<UsageCache>(bytes) {
        Ok(cache) => UsageCacheOutcome::Fresh(cache),
        Err(err) => UsageCacheOutcome::Corrupt(err.to_string()),
    }
}

pub fn read_usage_cache(path: &Path) -> UsageCacheOutcome {
    match std::fs::read(path) {
        Ok(bytes) => parse_usage_cache_bytes(&bytes),
        Err(err) if err.kind() == ErrorKind::NotFound => UsageCacheOutcome::Missing,
        Err(err) => UsageCacheOutcome::Corrupt(err.to_string()),
    }
}
