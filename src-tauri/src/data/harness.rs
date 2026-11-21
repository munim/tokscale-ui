use std::collections::HashMap;

use crate::contracts::{ChartSegment, HarnessSeriesItem};

pub const MAX_STACKED_HARNESSES: usize = 5;
pub const OTHER_HARNESS_ID: &str = "other";
pub const OTHER_HARNESS_LABEL: &str = "Other";
pub const OTHER_HARNESS_COLOR: &str = "var(--c-other)";
pub const OTHER_HARNESS_MONOGRAM: &str = "OT";

const FNV1A_64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV1A_64_PRIME: u64 = 0x100000001b3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessMeta {
    pub id: String,
    pub label: String,
    pub color: String,
    pub monogram: String,
    pub curated: bool,
}

pub fn fnv1a_64(input: &str) -> u64 {
    let mut hash = FNV1A_64_OFFSET_BASIS;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV1A_64_PRIME);
    }
    hash
}

fn monogram_from_label(label: &str) -> String {
    let chars: String = label
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(2)
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if chars.is_empty() {
        "??".to_string()
    } else {
        chars
    }
}

fn monogram_from_id(id: &str) -> String {
    let chars: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(2)
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if chars.is_empty() {
        "??".to_string()
    } else {
        chars
    }
}

fn fallback_label_from_id(id: &str) -> String {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return "Unknown".to_string();
    }
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(first) => {
            let mut out = String::new();
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
            out
        }
        None => "Unknown".to_string(),
    }
}

pub fn resolve(id: &str) -> HarnessMeta {
    let normalized = id.trim().to_ascii_lowercase();

    match normalized.as_str() {
        "claude" => HarnessMeta {
            id: "claude".to_string(),
            label: "Claude Code".to_string(),
            color: "oklch(68% 0.15 50)".to_string(),
            monogram: "CC".to_string(),
            curated: true,
        },
        "codex" => HarnessMeta {
            id: "codex".to_string(),
            label: "Codex".to_string(),
            color: "oklch(60% 0.13 250)".to_string(),
            monogram: "CO".to_string(),
            curated: true,
        },
        "opencode" => HarnessMeta {
            id: "opencode".to_string(),
            label: "OpenCode".to_string(),
            color: "oklch(60% 0.12 305)".to_string(),
            monogram: "OP".to_string(),
            curated: true,
        },
        "gemini" => HarnessMeta {
            id: "gemini".to_string(),
            label: "Gemini CLI".to_string(),
            color: "oklch(78% 0.13 85)".to_string(),
            monogram: "GE".to_string(),
            curated: true,
        },
        "other" => HarnessMeta {
            id: OTHER_HARNESS_ID.to_string(),
            label: OTHER_HARNESS_LABEL.to_string(),
            color: OTHER_HARNESS_COLOR.to_string(),
            monogram: OTHER_HARNESS_MONOGRAM.to_string(),
            curated: false,
        },
        _ => {
            let hue = (fnv1a_64(&normalized) % 360) as u16;
            let color = format!("oklch(62% 0.12 {hue})");

            let known_label = match normalized.as_str() {
                "cursor" => Some("Cursor"),
                "trae" => Some("Trae"),
                "warp" => Some("Warp"),
                "capy" => Some("Capy"),
                "antigravity" => Some("Antigravity"),
                "copilot" => Some("GitHub Copilot"),
                "amp" => Some("Amp"),
                "droid" => Some("Droid"),
                _ => None,
            };

            let (label, monogram) = match known_label {
                Some(label) => (label.to_string(), monogram_from_label(label)),
                None => (
                    fallback_label_from_id(id),
                    monogram_from_id(&normalized),
                ),
            };

            HarnessMeta {
                id: normalized,
                label,
                color,
                monogram,
                curated: false,
            }
        }
    }
}

pub fn known_harnesses() -> Vec<HarnessMeta> {
    [
        "claude",
        "codex",
        "opencode",
        "gemini",
        "cursor",
        "trae",
        "warp",
        "capy",
        "antigravity",
        "copilot",
        "amp",
        "droid",
    ]
    .into_iter()
    .map(resolve)
    .collect()
}

/// Bucket `HarnessSeriesItem`s so that at most `max_top` (default 5) remain individual and
/// remaining harnesses roll into `"other"` (`var(--c-other)`).
pub fn bucket_harnesses(mut items: Vec<HarnessSeriesItem>, max_top: usize) -> Vec<HarnessSeriesItem> {
    items.sort_by(|a, b| {
        b.cost
            .total_cmp(&a.cost)
            .then_with(|| b.tokens.cmp(&a.tokens))
            .then_with(|| a.id.cmp(&b.id))
    });

    if items.len() <= max_top {
        return items;
    }

    let mut top = Vec::with_capacity(max_top + 1);
    let mut other_cost = 0.0_f64;
    let mut other_tokens = 0_u64;
    let mut other_share = 0.0_f64;
    let mut other_sessions: Option<u64> = None;

    for (idx, item) in items.into_iter().enumerate() {
        if idx < max_top {
            top.push(item);
        } else {
            other_cost += item.cost;
            other_tokens = other_tokens.saturating_add(item.tokens);
            other_share += item.share_percent;
            if let Some(s) = item.sessions {
                other_sessions = Some(other_sessions.unwrap_or(0).saturating_add(s));
            }
        }
    }

    top.push(HarnessSeriesItem {
        id: OTHER_HARNESS_ID.to_string(),
        label: OTHER_HARNESS_LABEL.to_string(),
        color: OTHER_HARNESS_COLOR.to_string(),
        monogram: OTHER_HARNESS_MONOGRAM.to_string(),
        cost: other_cost,
        tokens: other_tokens,
        share_percent: other_share,
        sessions: other_sessions,
    });

    top
}

/// Bucket chart segments for a single bar given the globally top `top_ids` harnesses.
/// Any harness not in `top_ids` rolls into a single `"other"` segment.
pub fn bucket_segments_by_top_ids(
    segments: Vec<ChartSegment>,
    top_ids: &[String],
) -> Vec<ChartSegment> {
    let mut kept = Vec::new();
    let mut other_cost = 0.0_f64;
    let mut other_tokens = 0_u64;
    let mut has_other = false;

    let mut by_id: HashMap<String, ChartSegment> = HashMap::new();
    for seg in segments {
        if top_ids.iter().any(|id| id == &seg.harness_id) {
            let entry = by_id.entry(seg.harness_id.clone()).or_insert_with(|| ChartSegment {
                harness_id: seg.harness_id.clone(),
                label: seg.label.clone(),
                color: seg.color.clone(),
                cost: 0.0,
                tokens: 0,
            });
            entry.cost += seg.cost;
            entry.tokens = entry.tokens.saturating_add(seg.tokens);
        } else {
            has_other = true;
            other_cost += seg.cost;
            other_tokens = other_tokens.saturating_add(seg.tokens);
        }
    }

    for id in top_ids {
        if let Some(seg) = by_id.remove(id) {
            kept.push(seg);
        }
    }

    if has_other {
        kept.push(ChartSegment {
            harness_id: OTHER_HARNESS_ID.to_string(),
            label: OTHER_HARNESS_LABEL.to_string(),
            color: OTHER_HARNESS_COLOR.to_string(),
            cost: other_cost,
            tokens: other_tokens,
        });
    }

    kept
}
