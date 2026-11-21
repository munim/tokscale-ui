use std::collections::{BTreeMap, HashMap};

use chrono::{Datelike, Duration, NaiveDate};

use crate::contracts::{
    AgentPersonaRow, BestDay, ChartBar, ChartSegment, CliResolutionInfo, DashboardKpis,
    DashboardNavigateEvent, DashboardQuery, DashboardSnapshot, DashboardSources, DailyViewRow,
    GraphCell as ContractGraphCell, GraphData, HarnessSeriesItem, ProviderUsageView, RangePreset,
    SessionDetail, SessionRow, SourceHealthRow, SourceWarning, StatsSummary, SyncStatus,
    TopModelSummary, TraySnapshot, TrayTotal, UsageMetricView,
};
use crate::data::harness::{
    self, bucket_harnesses, bucket_segments_by_top_ids, MAX_STACKED_HARNESSES,
};
use crate::data::schema::{
    ClientsCliPayload, DailyRow, MonthlyCliRow, TokenCounts, TuiCache, UsageCache,
};
use crate::data::sessions::{filter_sessions, GroupedModelCliRow, ReportSession};
use crate::data::slice::{
    client_matches, contains_ci, contract_tokens_total, daily_for_range, hourly_for_range,
    models_for_range, monthly_for_range, normalize_search, parse_date_prefix, resolve_bounds,
    DateBounds,
};
use crate::data::source::DataOrigin;

#[derive(Debug, Clone, Default)]
pub struct DataStore {
    pub cache: TuiCache,
    pub usage: UsageCache,
    pub clients_payload: Option<ClientsCliPayload>,
    pub report_sessions: Vec<ReportSession>,
    pub grouped_models: Vec<GroupedModelCliRow>,
    pub joined_sessions: Vec<SessionRow>,
    pub session_details: HashMap<(String, String), SessionDetail>,
    pub monthly_override: Option<Vec<MonthlyCliRow>>,
    pub cli_info: CliResolutionInfoDefault,
    pub origin: DataOrigin,
    pub fallback_required: bool,
    pub syncing: bool,
    pub last_synced_at: Option<u64>,
    pub warnings: Vec<SourceWarning>,
    pub pending_handoff: Option<DashboardNavigateEvent>,
}

#[derive(Debug, Clone)]
pub struct CliResolutionInfoDefault(pub CliResolutionInfo);

impl Default for CliResolutionInfoDefault {
    fn default() -> Self {
        Self(CliResolutionInfo {
            resolved_via: "unavailable".to_string(),
            binary_path: None,
            version: None,
            fallback: None,
        })
    }
}

pub fn build_tray(range: RangePreset, store: &DataStore, today: NaiveDate) -> TraySnapshot {
    let query = DashboardQuery {
        range,
        custom_since: None,
        custom_until: None,
        group_by: "client,model".to_string(),
        active_clients: Vec::new(),
        search: None,
        sort: Some("cost".to_string()),
    };

    let (daily_views, mut slice_warnings) = daily_for_range(&store.cache.data, &query, today);
    let (models, model_warnings) = models_for_range(&store.cache.data, &query, today);
    slice_warnings.extend(model_warnings);

    let (total_cost, total_tokens) = compute_totals(
        &daily_views,
        &models,
        &store.cache,
        range == RangePreset::All,
        false,
    );

    let sessions_in_range = filter_sessions(
        &store.joined_sessions,
        &store.session_details,
        &query,
        today,
    );

    let sessions_count = compute_session_count(&sessions_in_range, &models, range);

    let raw_harnesses = aggregate_harnesses(
        &daily_views,
        &models,
        &sessions_in_range,
        total_cost,
        !store.joined_sessions.is_empty(),
    );
    let harnesses = bucket_harnesses(raw_harnesses, MAX_STACKED_HARNESSES);
    let top_ids: Vec<String> = harnesses
        .iter()
        .filter(|h| h.id != harness::OTHER_HARNESS_ID)
        .map(|h| h.id.clone())
        .collect();

    let (hourly_views, _) = if range == RangePreset::Today {
        hourly_for_range(&store.cache.data, &query, today)
    } else {
        (Vec::new(), Vec::new())
    };

    let bounds = resolve_bounds(range, None, None, today).unwrap_or(DateBounds {
        since: None,
        until: None,
    });
    let bars = build_chart_bars(range, bounds, &daily_views, &hourly_views, &top_ids, today);

    let top_models = models
        .iter()
        .take(3)
        .map(|m| {
            let tokens = contract_tokens_total(&m.tokens);
            let share_percent = if total_cost > 0.0 {
                (m.cost / total_cost) * 100.0
            } else {
                0.0
            };
            TopModelSummary {
                model: m.model.clone(),
                client: m.client.clone(),
                provider: m.provider.clone(),
                cost: m.cost,
                tokens,
                share_percent,
            }
        })
        .collect();

    let sources = build_source_health_rows(store);
    let mut warnings = store.warnings.clone();
    warnings.extend(slice_warnings);

    TraySnapshot {
        range,
        total: TrayTotal {
            cost: total_cost,
            tokens: total_tokens,
            sessions: sessions_count,
        },
        bars,
        harnesses,
        top_models,
        sources,
        sync_status: SyncStatus {
            syncing: store.syncing,
            last_synced_at: store.last_synced_at.or(Some(store.cache.timestamp).filter(|&t| t > 0)),
            warnings,
        },
        cli_info: store.cli_info.0.clone(),
    }
}

pub fn build_dashboard(
    query: &DashboardQuery,
    store: &DataStore,
    today: NaiveDate,
) -> DashboardSnapshot {
    let (daily_views, mut slice_warnings) = daily_for_range(&store.cache.data, query, today);
    let (models, model_warnings) = models_for_range(&store.cache.data, query, today);
    let (hourly_views, hourly_warnings) = hourly_for_range(&store.cache.data, query, today);

    slice_warnings.extend(model_warnings);
    slice_warnings.extend(hourly_warnings);

    // Deduplicate warnings while preserving order.
    let mut seen_warnings = Vec::new();
    for w in store.warnings.iter().cloned().chain(slice_warnings) {
        if !seen_warnings.contains(&w) {
            seen_warnings.push(w);
        }
    }

    let sessions_in_range = filter_sessions(
        &store.joined_sessions,
        &store.session_details,
        query,
        today,
    );

    let has_filters =
        !query.active_clients.is_empty() || normalize_search(query.search.as_deref()).is_some();

    let (total_cost, total_tokens) = compute_totals(
        &daily_views,
        &models,
        &store.cache,
        query.range == RangePreset::All,
        has_filters,
    );

    let sessions_count = compute_session_count(&sessions_in_range, &models, query.range);

    let active_days = daily_views
        .iter()
        .filter(|d| d.cost > 0.0 || contract_tokens_total(&d.tokens) > 0)
        .count() as u32;

    let cost_per_million = if total_tokens > 0 {
        total_cost / (total_tokens as f64 / 1_000_000.0)
    } else {
        0.0
    };

    let raw_harnesses = aggregate_harnesses(
        &daily_views,
        &models,
        &sessions_in_range,
        total_cost,
        !store.joined_sessions.is_empty(),
    );
    let harnesses = bucket_harnesses(raw_harnesses, MAX_STACKED_HARNESSES);
    let top_ids: Vec<String> = harnesses
        .iter()
        .filter(|h| h.id != harness::OTHER_HARNESS_ID)
        .map(|h| h.id.clone())
        .collect();

    let bounds = resolve_bounds(
        query.range,
        query.custom_since.as_deref(),
        query.custom_until.as_deref(),
        today,
    )
    .unwrap_or(DateBounds {
        since: None,
        until: None,
    });

    let chart = build_chart_bars(
        query.range,
        bounds,
        &daily_views,
        &hourly_views,
        &top_ids,
        today,
    );

    let monthly = if let Some(ref override_rows) = store.monthly_override {
        if query.range == RangePreset::All && !has_filters {
            override_rows
                .iter()
                .map(|m| crate::contracts::MonthlyViewRow {
                    month: m.month.clone(),
                    models: m.models.clone(),
                    tokens: m.tokens.into(),
                    message_count: m.message_count,
                    session_count: m.session_count,
                    active_days: m.active_days,
                    cost: m.cost,
                })
                .collect()
        } else {
            monthly_for_range(
                &daily_views,
                &store.cache.data.daily,
                &query.active_clients,
                &sessions_in_range,
            )
        }
    } else {
        monthly_for_range(
            &daily_views,
            &store.cache.data.daily,
            &query.active_clients,
            &sessions_in_range,
        )
    };

    let graph = build_graph_data(&store.cache.data.daily, &daily_views, &store.cache, bounds);
    let stats = build_stats_summary(&daily_views, &store.cache, query.range == RangePreset::All && !has_filters, today);
    let usage = build_usage_views(&store.usage);
    let agents = build_agent_rows(&store.cache, query);
    let sources = build_dashboard_sources(store);

    DashboardSnapshot {
        query: query.clone(),
        kpis: DashboardKpis {
            total_cost,
            total_tokens,
            sessions: sessions_count,
            cost_per_million,
            active_days,
        },
        chart,
        harnesses,
        models,
        daily: daily_views,
        hourly: hourly_views,
        monthly,
        graph,
        stats,
        usage,
        agents,
        sources,
        sync_status: SyncStatus {
            syncing: store.syncing,
            last_synced_at: store.last_synced_at.or(Some(store.cache.timestamp).filter(|&t| t > 0)),
            warnings: seen_warnings,
        },
        pending_handoff: store.pending_handoff.clone(),
    }
}

fn compute_totals(
    daily_views: &[DailyViewRow],
    models: &[crate::contracts::ModelViewRow],
    cache: &TuiCache,
    is_all: bool,
    has_filters: bool,
) -> (f64, u64) {
    if !daily_views.is_empty() {
        let cost: f64 = daily_views.iter().map(|d| d.cost).sum();
        let tokens: u64 = daily_views
            .iter()
            .map(|d| contract_tokens_total(&d.tokens))
            .sum();
        if is_all && !has_filters && cache.data.undated_tokens > 0 {
            return (
                cache.data.total_cost.max(cost),
                cache.data.total_tokens.max(tokens),
            );
        }
        return (cost, tokens);
    }

    if !models.is_empty() {
        let cost: f64 = models.iter().map(|m| m.cost).sum();
        let tokens: u64 = models
            .iter()
            .map(|m| contract_tokens_total(&m.tokens))
            .sum();
        return (cost, tokens);
    }

    if is_all && !has_filters {
        (cache.data.total_cost, cache.data.total_tokens)
    } else {
        (0.0, 0)
    }
}

fn compute_session_count(
    sessions_in_range: &[SessionRow],
    models: &[crate::contracts::ModelViewRow],
    range: RangePreset,
) -> u64 {
    if !sessions_in_range.is_empty() {
        return sessions_in_range.len() as u64;
    }
    if range == RangePreset::All {
        return models.iter().map(|m| m.session_count).sum();
    }
    0
}

fn aggregate_harnesses(
    daily_views: &[DailyViewRow],
    models: &[crate::contracts::ModelViewRow],
    sessions_in_range: &[SessionRow],
    total_cost: f64,
    has_session_data: bool,
) -> Vec<HarnessSeriesItem> {
    let mut by_client: HashMap<String, (f64, u64)> = HashMap::new();

    for day in daily_views {
        for sb in &day.source_breakdown {
            let key = sb.client.trim().to_ascii_lowercase();
            if key.is_empty() {
                continue;
            }
            let entry = by_client.entry(key).or_insert((0.0, 0));
            entry.0 += sb.cost;
            entry.1 = entry.1.saturating_add(contract_tokens_total(&sb.tokens));
        }
    }

    if by_client.is_empty() {
        for m in models {
            let primary = m
                .client
                .split(',')
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("unknown")
                .to_ascii_lowercase();
            let entry = by_client.entry(primary).or_insert((0.0, 0));
            entry.0 += m.cost;
            entry.1 = entry.1.saturating_add(contract_tokens_total(&m.tokens));
        }
    }

    let mut session_counts: HashMap<String, u64> = HashMap::new();
    for s in sessions_in_range {
        let key = s.client.trim().to_ascii_lowercase();
        if !key.is_empty() {
            *session_counts.entry(key).or_insert(0) += 1;
        }
    }

    by_client
        .into_iter()
        .map(|(id, (cost, tokens))| {
            let meta = harness::resolve(&id);
            let share_percent = if total_cost > 0.0 {
                (cost / total_cost) * 100.0
            } else {
                0.0
            };
            let sessions = if has_session_data {
                Some(session_counts.get(&id).copied().unwrap_or(0))
            } else {
                None
            };
            HarnessSeriesItem {
                id: meta.id,
                label: meta.label,
                color: meta.color,
                monogram: meta.monogram,
                cost,
                tokens,
                share_percent,
                sessions,
            }
        })
        .collect()
}

fn build_chart_bars(
    range: RangePreset,
    bounds: DateBounds,
    daily_views: &[DailyViewRow],
    hourly_views: &[crate::contracts::HourlyViewRow],
    top_ids: &[String],
    today: NaiveDate,
) -> Vec<ChartBar> {
    if range == RangePreset::Today && !hourly_views.is_empty() {
        let mut sorted_hourly = hourly_views.to_vec();
        sorted_hourly.sort_by(|a, b| a.datetime.cmp(&b.datetime));

        return sorted_hourly
            .into_iter()
            .map(|h| {
                let slot = h
                    .datetime
                    .split_whitespace()
                    .nth(1)
                    .and_then(|t| t.get(..5))
                    .unwrap_or(h.datetime.as_str())
                    .to_string();
                let total_tokens = contract_tokens_total(&h.tokens);
                let raw_segments = if h.clients.is_empty() {
                    Vec::new()
                } else {
                    let count = h.clients.len() as f64;
                    let per_cost = h.cost / count;
                    let per_tokens = total_tokens / h.clients.len() as u64;
                    h.clients
                        .iter()
                        .map(|c| {
                            let meta = harness::resolve(c);
                            ChartSegment {
                                harness_id: meta.id,
                                label: meta.label,
                                color: meta.color,
                                cost: per_cost,
                                tokens: per_tokens,
                            }
                        })
                        .collect()
                };
                ChartBar {
                    slot: slot.clone(),
                    label: slot,
                    total_cost: h.cost,
                    total_tokens,
                    segments: bucket_segments_by_top_ids(raw_segments, top_ids),
                }
            })
            .collect();
    }

    let daily_by_date: HashMap<&str, &DailyViewRow> =
        daily_views.iter().map(|d| (d.date.as_str(), d)).collect();

    match range {
        RangePreset::Today | RangePreset::SevenDays | RangePreset::ThirtyDays => {
            let start = bounds.since.unwrap_or(today);
            let end = bounds.until.unwrap_or(today);
            let mut bars = Vec::new();
            let mut cur = start;
            while cur <= end {
                let key = cur.format("%Y-%m-%d").to_string();
                let label = cur.format("%m-%d").to_string();
                bars.push(bar_from_daily(
                    key.clone(),
                    label,
                    daily_by_date.get(key.as_str()).copied(),
                    top_ids,
                ));
                cur += Duration::days(1);
            }
            bars
        }
        RangePreset::Custom => {
            if let (Some(start), Some(end)) = (bounds.since, bounds.until) {
                let span = (end - start).num_days();
                if (0..=62).contains(&span) {
                    let mut bars = Vec::new();
                    let mut cur = start;
                    while cur <= end {
                        let key = cur.format("%Y-%m-%d").to_string();
                        let label = cur.format("%m-%d").to_string();
                        bars.push(bar_from_daily(
                            key.clone(),
                            label,
                            daily_by_date.get(key.as_str()).copied(),
                            top_ids,
                        ));
                        cur += Duration::days(1);
                    }
                    return bars;
                }
            }
            build_adaptive_bars(daily_views, top_ids)
        }
        RangePreset::All => build_adaptive_bars(daily_views, top_ids),
    }
}

fn build_adaptive_bars(daily_views: &[DailyViewRow], top_ids: &[String]) -> Vec<ChartBar> {
    if daily_views.len() <= 45 {
        let mut sorted = daily_views.to_vec();
        sorted.sort_by(|a, b| a.date.cmp(&b.date));
        return sorted
            .iter()
            .map(|d| {
                let label = d.date.get(5..10).unwrap_or(d.date.as_str()).to_string();
                bar_from_daily(d.date.clone(), label, Some(d), top_ids)
            })
            .collect();
    }

    // Group by YYYY-MM chronologically for multi-month history.
    let mut by_month: BTreeMap<String, (f64, u64, Vec<ChartSegment>)> = BTreeMap::new();
    for d in daily_views {
        let Some(month) = d.date.get(..7) else {
            continue;
        };
        let entry = by_month
            .entry(month.to_string())
            .or_insert_with(|| (0.0, 0, Vec::new()));
        entry.0 += d.cost;
        entry.1 = entry.1.saturating_add(contract_tokens_total(&d.tokens));
        for sb in &d.source_breakdown {
            let meta = harness::resolve(&sb.client);
            entry.2.push(ChartSegment {
                harness_id: meta.id,
                label: meta.label,
                color: meta.color,
                cost: sb.cost,
                tokens: contract_tokens_total(&sb.tokens),
            });
        }
    }

    by_month
        .into_iter()
        .map(|(month, (total_cost, total_tokens, segs))| ChartBar {
            slot: month.clone(),
            label: month,
            total_cost,
            total_tokens,
            segments: bucket_segments_by_top_ids(segs, top_ids),
        })
        .collect()
}

fn bar_from_daily(
    slot: String,
    label: String,
    day: Option<&DailyViewRow>,
    top_ids: &[String],
) -> ChartBar {
    let Some(day) = day else {
        return ChartBar {
            slot,
            label,
            total_cost: 0.0,
            total_tokens: 0,
            segments: Vec::new(),
        };
    };

    let raw_segments: Vec<ChartSegment> = day
        .source_breakdown
        .iter()
        .map(|sb| {
            let meta = harness::resolve(&sb.client);
            ChartSegment {
                harness_id: meta.id,
                label: meta.label,
                color: meta.color,
                cost: sb.cost,
                tokens: contract_tokens_total(&sb.tokens),
            }
        })
        .collect();

    ChartBar {
        slot,
        label,
        total_cost: day.cost,
        total_tokens: contract_tokens_total(&day.tokens),
        segments: bucket_segments_by_top_ids(raw_segments, top_ids),
    }
}

fn build_graph_data(
    raw_daily: &[DailyRow],
    filtered_daily: &[DailyViewRow],
    cache: &TuiCache,
    bounds: DateBounds,
) -> GraphData {
    let raw_by_date: HashMap<&str, &DailyRow> =
        raw_daily.iter().map(|d| (d.date.as_str(), d)).collect();
    let filtered_by_date: HashMap<&str, &DailyViewRow> =
        filtered_daily.iter().map(|d| (d.date.as_str(), d)).collect();

    if !cache.data.graph.weeks.is_empty() {
        let weeks = cache
            .data
            .graph
            .weeks
            .iter()
            .map(|week| {
                week.iter()
                    .map(|cell_opt| {
                        cell_opt.as_ref().map(|cell| {
                            if !bounds.contains_str(&cell.date) {
                                return ContractGraphCell {
                                    date: cell.date.clone(),
                                    tokens: TokenCounts::default().into(),
                                    cost: 0.0,
                                    intensity: 0,
                                };
                            }
                            if let Some(day) = filtered_by_date.get(cell.date.as_str()) {
                                let tokens_total = contract_tokens_total(&day.tokens);
                                let intensity = if tokens_total == 0 && day.cost == 0.0 {
                                    0
                                } else {
                                    cell.intensity_level().max(1)
                                };
                                return ContractGraphCell {
                                    date: cell.date.clone(),
                                    tokens: day.tokens,
                                    cost: day.cost,
                                    intensity,
                                };
                            }
                            let tokens = raw_by_date
                                .get(cell.date.as_str())
                                .map(|d| d.tokens.into())
                                .unwrap_or_else(|| cell.tokens.into());
                            ContractGraphCell {
                                date: cell.date.clone(),
                                tokens,
                                cost: cell.cost,
                                intensity: cell.intensity_level(),
                            }
                        })
                    })
                    .collect()
            })
            .collect();

        return GraphData { weeks };
    }

    // Fallback synthesis when graph is absent (e.g. CLI fallback mode).
    if filtered_daily.is_empty() {
        return GraphData { weeks: Vec::new() };
    }

    let max_cost = filtered_daily
        .iter()
        .map(|d| d.cost)
        .fold(0.0_f64, f64::max);

    let mut by_week: BTreeMap<(i32, u32), [Option<ContractGraphCell>; 7]> = BTreeMap::new();
    for day in filtered_daily {
        let Some(date) = parse_date_prefix(&day.date) else {
            continue;
        };
        let iso = date.iso_week();
        let weekday_idx = date.weekday().num_days_from_sunday() as usize;
        let intensity = if day.cost <= 0.0 && contract_tokens_total(&day.tokens) == 0 {
            0
        } else if max_cost > 0.0 {
            (((day.cost / max_cost) * 4.0).ceil() as u8).clamp(1, 4)
        } else {
            1
        };
        let week_arr = by_week
            .entry((iso.year(), iso.week()))
            .or_insert([None, None, None, None, None, None, None]);
        week_arr[weekday_idx] = Some(ContractGraphCell {
            date: day.date.clone(),
            tokens: day.tokens,
            cost: day.cost,
            intensity,
        });
    }

    GraphData {
        weeks: by_week.into_values().map(|w| w.to_vec()).collect(),
    }
}

fn build_stats_summary(
    daily_views: &[DailyViewRow],
    cache: &TuiCache,
    use_cached_streaks: bool,
    today: NaiveDate,
) -> StatsSummary {
    let active_rows: Vec<&DailyViewRow> = daily_views
        .iter()
        .filter(|d| d.cost > 0.0 || contract_tokens_total(&d.tokens) > 0)
        .collect();

    let active_days = active_rows.len() as u32;
    let total_cost: f64 = active_rows.iter().map(|d| d.cost).sum();
    let avg_cost_per_active_day = if active_days > 0 {
        total_cost / f64::from(active_days)
    } else {
        0.0
    };

    let best_day = active_rows
        .iter()
        .max_by(|a, b| a.cost.total_cmp(&b.cost).then_with(|| b.date.cmp(&a.date)))
        .map(|d| BestDay {
            date: d.date.clone(),
            cost: d.cost,
        });

    let (current_streak, longest_streak) =
        if use_cached_streaks && (cache.data.current_streak > 0 || cache.data.longest_streak > 0) {
            (cache.data.current_streak, cache.data.longest_streak)
        } else {
            compute_streaks(&active_rows, today)
        };

    StatsSummary {
        active_days,
        current_streak,
        longest_streak,
        best_day,
        avg_cost_per_active_day,
    }
}

fn compute_streaks(active_rows: &[&DailyViewRow], today: NaiveDate) -> (u32, u32) {
    let mut dates: Vec<NaiveDate> = active_rows
        .iter()
        .filter_map(|d| parse_date_prefix(&d.date))
        .collect();
    dates.sort_unstable();
    dates.dedup();

    if dates.is_empty() {
        return (0, 0);
    }

    let mut longest = 1_u32;
    let mut run = 1_u32;
    for pair in dates.windows(2) {
        if pair[1] - pair[0] == Duration::days(1) {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 1;
        }
    }

    let mut current = 0_u32;
    if let Some(&last) = dates.last() {
        if last == today || last == today - Duration::days(1) {
            current = 1;
            for pair in dates.windows(2).rev() {
                if pair[1] - pair[0] == Duration::days(1) {
                    current += 1;
                } else {
                    break;
                }
            }
        }
    }

    (current, longest)
}

fn build_usage_views(usage: &UsageCache) -> Vec<ProviderUsageView> {
    usage
        .data
        .iter()
        .map(|p| ProviderUsageView {
            provider: p.provider.clone(),
            email: p.email.clone(),
            plan: p.plan.clone(),
            metrics: p
                .metrics
                .iter()
                .map(|m| UsageMetricView {
                    label: m.label.clone(),
                    remaining_label: m.remaining_label.clone(),
                    remaining_percent: m.remaining_percent,
                    used_percent: m.used_percent,
                    resets_at: m.resets_at.clone(),
                })
                .collect(),
        })
        .collect()
}

fn build_agent_rows(cache: &TuiCache, query: &DashboardQuery) -> Vec<AgentPersonaRow> {
    let needle = normalize_search(query.search.as_deref());
    let mut rows: Vec<AgentPersonaRow> = cache
        .data
        .agents
        .iter()
        .filter(|a| {
            if !query.active_clients.is_empty()
                && !a
                    .clients
                    .split(',')
                    .map(str::trim)
                    .filter(|c| !c.is_empty())
                    .any(|c| client_matches(c, &query.active_clients))
            {
                return false;
            }
            if let Some(ref q) = needle {
                if !contains_ci(&a.agent, q) && !contains_ci(&a.clients, q) {
                    return false;
                }
            }
            true
        })
        .map(|a| AgentPersonaRow {
            agent: a.agent.clone(),
            clients: a.clients.clone(),
            tokens: a.tokens.into(),
            cost: a.cost,
            message_count: a.message_count,
        })
        .collect();

    rows.sort_by(|a, b| {
        b.cost
            .total_cmp(&a.cost)
            .then_with(|| contract_tokens_total(&b.tokens).cmp(&contract_tokens_total(&a.tokens)))
            .then_with(|| a.agent.cmp(&b.agent))
    });
    rows
}

fn build_source_health_rows(store: &DataStore) -> Vec<SourceHealthRow> {
    if let Some(ref payload) = store.clients_payload {
        return payload
            .clients
            .iter()
            .map(|c| {
                let meta = harness::resolve(&c.client);
                let label = c
                    .label
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .unwrap_or(meta.label);
                SourceHealthRow {
                    client: c.client.clone(),
                    label,
                    sessions_path: c.sessions_path.clone(),
                    sessions_path_exists: c.sessions_path_exists,
                    message_count: c.message_count,
                    headless_supported: c.headless_supported,
                    headless_message_count: c.headless_message_count,
                    status: if c.status.is_empty() {
                        if c.sessions_path_exists || c.message_count > 0 {
                            "ready".to_string()
                        } else {
                            "idle".to_string()
                        }
                    } else {
                        c.status.clone()
                    },
                    credentialed: c.credentialed,
                    api_backed: c.api_backed,
                    last_synced_at: c.last_synced_at.or(store.last_synced_at),
                    warning: c.warning.clone(),
                }
            })
            .collect();
    }

    // Synthesize from cache `daily` source breakdown and `enabled_clients`.
    let mut msg_counts: HashMap<String, u64> = HashMap::new();
    for day in &store.cache.data.daily {
        for (client, sb) in &day.source_breakdown {
            let msgs: u64 = sb.models.iter().map(|(_, m)| m.messages).sum();
            *msg_counts.entry(client.to_ascii_lowercase()).or_insert(0) += msgs;
        }
    }

    let mut clients: Vec<String> = if !store.cache.enabled_clients.is_empty() {
        store.cache.enabled_clients.clone()
    } else {
        msg_counts.keys().cloned().collect()
    };
    clients.sort();
    clients.dedup();

    clients
        .into_iter()
        .filter_map(|client| {
            let count = msg_counts.get(&client.to_ascii_lowercase()).copied().unwrap_or(0);
            // Keep curated/known or active clients to avoid cluttering with 60 empty ones.
            let meta = harness::resolve(&client);
            if count == 0 && !meta.curated {
                return None;
            }
            Some(SourceHealthRow {
                client: meta.id,
                label: meta.label,
                sessions_path: String::new(),
                sessions_path_exists: count > 0,
                message_count: count,
                headless_supported: false,
                headless_message_count: 0,
                status: if count > 0 {
                    "cached".to_string()
                } else {
                    "idle".to_string()
                },
                credentialed: false,
                api_backed: false,
                last_synced_at: store.last_synced_at.or(Some(store.cache.timestamp).filter(|&t| t > 0)),
                warning: None,
            })
        })
        .collect()
}

fn build_dashboard_sources(store: &DataStore) -> DashboardSources {
    let clients = build_source_health_rows(store);
    if let Some(ref payload) = store.clients_payload {
        DashboardSources {
            headless_roots: payload.headless_roots.clone(),
            note: payload.note.clone(),
            clients,
        }
    } else {
        DashboardSources {
            headless_roots: Vec::new(),
            note: "Populated from local cache; run sync to inspect CLI source paths.".to_string(),
            clients,
        }
    }
}
