use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::marker::PhantomData;

/// Verified schema version emitted by `tokscale` v4.23.0 `tui-data-cache.json`.
pub const EXPECTED_SCHEMA_VERSION: u32 = 12;

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TuiCache {
    pub schema_version: u32,
    pub parser_generation: u64,
    pub timestamp: u64,
    pub enabled_clients: Vec<String>,
    pub group_by: String,
    pub report_scope: ReportScope,
    pub data: TuiData,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ReportScope {
    pub since: Option<String>,
    pub until: Option<String>,
    pub year: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct TokenCounts {
    pub input: u64,
    pub output: u64,
    #[serde(alias = "cache_read")]
    pub cache_read: u64,
    #[serde(alias = "cache_write")]
    pub cache_write: u64,
    pub reasoning: u64,
}

impl TokenCounts {
    pub fn total(&self) -> u64 {
        self.input
            .saturating_add(self.output)
            .saturating_add(self.cache_read)
            .saturating_add(self.cache_write)
            .saturating_add(self.reasoning)
    }

    pub fn add_assign(&mut self, other: &Self) {
        self.input = self.input.saturating_add(other.input);
        self.output = self.output.saturating_add(other.output);
        self.cache_read = self.cache_read.saturating_add(other.cache_read);
        self.cache_write = self.cache_write.saturating_add(other.cache_write);
        self.reasoning = self.reasoning.saturating_add(other.reasoning);
    }
}

impl From<TokenCounts> for crate::contracts::TokenCounts {
    fn from(value: TokenCounts) -> Self {
        Self {
            input: value.input,
            output: value.output,
            cache_read: value.cache_read,
            cache_write: value.cache_write,
            reasoning: value.reasoning,
        }
    }
}

impl From<crate::contracts::TokenCounts> for TokenCounts {
    fn from(value: crate::contracts::TokenCounts) -> Self {
        Self {
            input: value.input,
            output: value.output,
            cache_read: value.cache_read,
            cache_write: value.cache_write,
            reasoning: value.reasoning,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ModelRow {
    pub model: String,
    pub color_key: String,
    pub provider: String,
    pub client: String,
    pub workspace_key: Option<String>,
    pub workspace_label: Option<String>,
    pub machine: Option<String>,
    pub tokens: TokenCounts,
    pub cost: f64,
    pub performance: Option<serde_json::Value>,
    pub session_count: u64,
    pub group_key: String,
}

impl ModelRow {
    pub fn ms_per_1k_tokens(&self) -> Option<f64> {
        let perf = self.performance.as_ref()?;
        perf.get("msPer1KTokens")
            .or_else(|| perf.get("ms_per_1k_tokens"))
            .and_then(|v| v.as_f64())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AgentRow {
    pub agent: String,
    pub clients: String,
    pub tokens: TokenCounts,
    pub cost: f64,
    pub message_count: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct DailyRow {
    pub date: String,
    pub tokens: TokenCounts,
    pub cost: f64,
    pub models: serde_json::Value,
    #[serde(deserialize_with = "deserialize_pairs_or_map")]
    pub source_breakdown: Vec<(String, SourceBreakdown)>,
    pub message_count: u64,
    pub turn_count: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SourceBreakdownModel {
    pub client: String,
    pub provider: String,
    pub display_name: String,
    pub color_key: String,
    pub tokens: TokenCounts,
    pub cost: f64,
    #[serde(alias = "messageCount", alias = "message_count")]
    pub messages: u64,
    pub performance: Option<serde_json::Value>,
}

impl SourceBreakdownModel {
    pub fn ms_per_1k_tokens(&self) -> Option<f64> {
        let perf = self.performance.as_ref()?;
        perf.get("msPer1KTokens")
            .or_else(|| perf.get("ms_per_1k_tokens"))
            .and_then(|v| v.as_f64())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SourceBreakdown {
    pub tokens: TokenCounts,
    pub cost: f64,
    #[serde(deserialize_with = "deserialize_pairs_or_map")]
    pub models: Vec<(String, SourceBreakdownModel)>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct HourlyRow {
    pub datetime: String,
    pub tokens: TokenCounts,
    pub cost: f64,
    #[serde(deserialize_with = "deserialize_string_or_vec")]
    pub clients: Vec<String>,
    #[serde(deserialize_with = "deserialize_pairs_or_map")]
    pub models: Vec<(String, HourlyModel)>,
    pub message_count: u64,
    pub turn_count: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct HourlyModel {
    pub provider: String,
    pub display_name: String,
    pub color_key: String,
    pub tokens: TokenCounts,
    pub cost: f64,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GraphCell {
    pub date: String,
    #[serde(deserialize_with = "deserialize_graph_tokens")]
    pub tokens: TokenCounts,
    pub cost: f64,
    pub intensity: f64,
}

impl GraphCell {
    /// Convert normalized cache intensity (`0.0..=1.0`) or integer level (`0..=4`) into `0..=4`.
    pub fn intensity_level(&self) -> u8 {
        if !self.intensity.is_finite() || self.intensity <= 0.0 {
            0
        } else if self.intensity <= 1.0 {
            ((self.intensity * 4.0).ceil() as u8).clamp(1, 4)
        } else {
            (self.intensity.round() as u8).clamp(0, 4)
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Graph {
    pub weeks: Vec<Vec<Option<GraphCell>>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TuiData {
    pub models: Vec<ModelRow>,
    pub agents: Vec<AgentRow>,
    pub daily: Vec<DailyRow>,
    pub hourly: Vec<HourlyRow>,
    pub graph: Graph,
    pub total_tokens: u64,
    pub undated_tokens: u64,
    pub total_cost: f64,
    pub current_streak: u32,
    pub longest_streak: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct UsageCache {
    pub timestamp: u64,
    pub data: Vec<ProviderUsage>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ProviderUsage {
    pub provider: String,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub metrics: Vec<UsageMetric>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct UsageMetric {
    pub label: String,
    #[serde(alias = "remaining_label", alias = "remainingLabel")]
    pub remaining_label: Option<String>,
    #[serde(alias = "remaining_percent", alias = "remainingPercent")]
    pub remaining_percent: Option<f64>,
    #[serde(alias = "used_percent", alias = "usedPercent")]
    pub used_percent: Option<f64>,
    #[serde(alias = "resets_at", alias = "resetsAt")]
    pub resets_at: Option<String>,
}

/// CLI `clients --json` row shape (tolerant to both snake_case and camelCase).
#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientHealthCliRow {
    pub client: String,
    pub label: Option<String>,
    #[serde(alias = "sessions_path")]
    pub sessions_path: String,
    #[serde(alias = "sessions_path_exists", alias = "exists")]
    pub sessions_path_exists: bool,
    #[serde(alias = "message_count", alias = "messages")]
    pub message_count: u64,
    #[serde(alias = "headless_supported")]
    pub headless_supported: bool,
    #[serde(alias = "headless_message_count")]
    pub headless_message_count: u64,
    pub status: String,
    pub credentialed: bool,
    #[serde(alias = "api_backed")]
    pub api_backed: bool,
    #[serde(alias = "last_synced_at")]
    pub last_synced_at: Option<u64>,
    pub warning: Option<String>,
}

/// CLI `clients --json` envelope or array.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientsCliPayload {
    #[serde(alias = "headless_roots")]
    pub headless_roots: Vec<String>,
    pub note: String,
    pub clients: Vec<ClientHealthCliRow>,
}

/// CLI `monthly --json` row shape.
#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct MonthlyCliRow {
    pub month: String,
    pub models: Vec<String>,
    pub tokens: TokenCounts,
    #[serde(alias = "message_count")]
    pub message_count: u64,
    #[serde(alias = "session_count")]
    pub session_count: u64,
    #[serde(alias = "active_days")]
    pub active_days: u32,
    pub cost: f64,
}

fn deserialize_pairs_or_map<'de, D, V>(deserializer: D) -> Result<Vec<(String, V)>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de>,
{
    struct PairsOrMapVisitor<V>(PhantomData<V>);

    impl<'de, V> Visitor<'de> for PairsOrMapVisitor<V>
    where
        V: Deserialize<'de>,
    {
        type Value = Vec<(String, V)>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence of [key, value] pairs, a map, or null")
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Vec::new())
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Vec::new())
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(pair) = seq.next_element::<(String, V)>()? {
                out.push(pair);
            }
            Ok(out)
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut out = Vec::with_capacity(map.size_hint().unwrap_or(0));
            while let Some((k, v)) = map.next_entry::<String, V>()? {
                out.push((k, v));
            }
            Ok(out)
        }
    }

    deserializer.deserialize_any(PairsOrMapVisitor(PhantomData))
}

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrVecVisitor;

    impl<'de> Visitor<'de> for StringOrVecVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string, sequence of strings, or null")
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Vec::new())
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Vec::new())
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect())
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(item) = seq.next_element::<String>()? {
                let trimmed = item.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
            Ok(out)
        }
    }

    deserializer.deserialize_any(StringOrVecVisitor)
}

fn deserialize_graph_tokens<'de, D>(deserializer: D) -> Result<TokenCounts, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RawGraphTokens {
        Count(u64),
        Counts(TokenCounts),
    }

    let opt = Option::<RawGraphTokens>::deserialize(deserializer)?;
    Ok(match opt {
        None => TokenCounts::default(),
        Some(RawGraphTokens::Count(total)) => TokenCounts {
            input: total,
            ..TokenCounts::default()
        },
        Some(RawGraphTokens::Counts(counts)) => counts,
    })
}
