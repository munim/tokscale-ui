import type {
  DailyViewRow,
  GraphCell,
  HourlyViewRow,
  ModelViewRow,
  MonthlyViewRow,
  StatsSummary,
  TokenCounts,
} from '$lib/contracts/ipc';
import { sumTokenCounts } from '$lib/format';
import { resolveHarness } from '$lib/harnesses';

export type ModelGroupMode = 'model' | 'client,model' | 'workspace,model' | 'client,session,model';
export type ModelSortKey = 'model' | 'cost' | 'tokens';
export type DailySortKey = 'date' | 'tokens' | 'cost';
export type HourlySortKey = 'hour' | 'tokens' | 'cost';
export type MonthlySortKey = 'month' | 'models' | 'tokens' | 'cost';
export type SortDirection = 'asc' | 'desc';

export interface ModelGroupMeta {
  id: string;
  label: string;
  rows: ModelViewRow[];
  subtotalCost: number;
  subtotalTokens: number;
}

export type ModelTableRow =
  | {
      kind: 'model';
      key: string;
      groupId: string | null;
      row: ModelViewRow;
    }
  | {
      kind: 'collapsed';
      key: string;
      groupId: string;
      row: ModelViewRow;
    };

export interface ModelTableModel {
  isGrouped: boolean;
  isSessionMode: boolean;
  groups: Map<string, ModelGroupMeta>;
  rows: ModelTableRow[];
}

const MONTH_SHORT_NAMES = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec',
] as const;

export function emptyTokenCounts(): TokenCounts {
  return { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, reasoning: 0 };
}

export function normalizeGroupMode(groupBy: string | undefined): ModelGroupMode {
  if (
    groupBy === 'client,model' ||
    groupBy === 'workspace,model' ||
    groupBy === 'client,session,model'
  ) {
    return groupBy;
  }
  return 'model';
}

export function shortSessionId(sessionId: string): string {
  const trimmed = sessionId.trim();
  if (trimmed.length <= 8) {
    return trimmed;
  }
  return trimmed.slice(-6);
}

export function extractDatePart(datetime: string): string {
  return datetime.slice(0, 10);
}

export function extractHourPart(datetime: string): string {
  const match = /(\d{2}):\d{2}/.exec(datetime);
  if (match) {
    return `${match[1]}:00`;
  }
  return datetime.slice(11, 16) || datetime;
}

export function formatMonthLabel(dateStr: string): string {
  const monthIdx = Number(dateStr.slice(5, 7)) - 1;
  return MONTH_SHORT_NAMES[monthIdx] ?? '';
}

function compareStrings(a: string, b: string, dir: SortDirection): number {
  const cmp = a.localeCompare(b);
  return dir === 'asc' ? cmp : -cmp;
}

function compareNumbers(a: number, b: number, dir: SortDirection): number {
  const cmp = a - b;
  return dir === 'asc' ? cmp : -cmp;
}

function compareModelRows(a: ModelViewRow, b: ModelViewRow, key: ModelSortKey, dir: SortDirection): number {
  if (key === 'model') {
    const primary = compareStrings(a.model, b.model, dir);
    return primary !== 0 ? primary : b.cost - a.cost;
  }
  if (key === 'tokens') {
    const primary = compareNumbers(sumTokenCounts(a.tokens), sumTokenCounts(b.tokens), dir);
    return primary !== 0 ? primary : b.cost - a.cost;
  }
  const primary = compareNumbers(a.cost, b.cost, dir);
  return primary !== 0 ? primary : a.model.localeCompare(b.model);
}

function resolveModelGroup(
  row: ModelViewRow,
  mode: Exclude<ModelGroupMode, 'model'>,
): { id: string; label: string } {
  if (mode === 'client,model') {
    const harness = resolveHarness(row.client);
    return {
      id: `client:${harness.id}`,
      label: harness.label,
    };
  }
  if (mode === 'workspace,model') {
    const label = row.workspaceLabel || row.workspaceKey || 'No workspace';
    const key = row.workspaceKey ?? row.workspaceLabel ?? '__none__';
    return {
      id: `workspace:${key}`,
      label,
    };
  }
  const harness = resolveHarness(row.client);
  if (row.sessionId) {
    return {
      id: `session:${harness.id}:${row.sessionId}`,
      label: `${harness.label} · ${shortSessionId(row.sessionId)}`,
    };
  }
  return {
    id: `session:${harness.id}:__none__`,
    label: harness.label,
  };
}

export function buildModelTableModel(
  models: ModelViewRow[],
  groupBy: string | undefined,
  sortKey: ModelSortKey,
  sortDir: SortDirection,
  collapsedGroupIds: ReadonlySet<string>,
): ModelTableModel {
  const mode = normalizeGroupMode(groupBy);
  const isSessionMode = mode === 'client,session,model';

  if (mode === 'model') {
    const sorted = [...models].sort((a, b) => compareModelRows(a, b, sortKey, sortDir));
    const rows: ModelTableRow[] = sorted.map((row, idx) => ({
      kind: 'model',
      key: `model::${idx}::${row.client}::${row.model}`,
      groupId: null,
      row,
    }));
    return {
      isGrouped: false,
      isSessionMode: false,
      groups: new Map(),
      rows,
    };
  }

  const groupBuckets = new Map<string, ModelGroupMeta>();
  for (const row of models) {
    const { id, label } = resolveModelGroup(row, mode);
    const rowTokens = sumTokenCounts(row.tokens);
    const existing = groupBuckets.get(id);
    if (existing) {
      existing.rows.push(row);
      existing.subtotalCost += row.cost;
      existing.subtotalTokens += rowTokens;
    } else {
      groupBuckets.set(id, {
        id,
        label,
        rows: [row],
        subtotalCost: row.cost,
        subtotalTokens: rowTokens,
      });
    }
  }

  const sortedGroups = Array.from(groupBuckets.values()).sort((a, b) => {
    if (sortKey === 'tokens') {
      const cmp = compareNumbers(a.subtotalTokens, b.subtotalTokens, sortDir);
      return cmp !== 0 ? cmp : b.subtotalCost - a.subtotalCost;
    }
    if (sortKey === 'model') {
      return a.label.localeCompare(b.label);
    }
    const cmp = compareNumbers(a.subtotalCost, b.subtotalCost, sortDir);
    return cmp !== 0 ? cmp : a.label.localeCompare(b.label);
  });

  const rows: ModelTableRow[] = [];
  for (const group of sortedGroups) {
    group.rows.sort((a, b) => compareModelRows(a, b, sortKey, sortDir));
    if (collapsedGroupIds.has(group.id)) {
      rows.push({
        kind: 'collapsed',
        key: `collapsed::${group.id}`,
        groupId: group.id,
        row: group.rows[0],
      });
    } else {
      for (let idx = 0; idx < group.rows.length; idx++) {
        const row = group.rows[idx];
        rows.push({
          kind: 'model',
          key: `model::${group.id}::${idx}::${row.model}`,
          groupId: group.id,
          row,
        });
      }
    }
  }

  return {
    isGrouped: true,
    isSessionMode,
    groups: groupBuckets,
    rows,
  };
}

export function sortDailyRows(
  daily: DailyViewRow[],
  sortKey: DailySortKey,
  sortDir: SortDirection,
): DailyViewRow[] {
  return [...daily].sort((a, b) => {
    if (sortKey === 'cost') {
      const cmp = compareNumbers(a.cost, b.cost, sortDir);
      return cmp !== 0 ? cmp : b.date.localeCompare(a.date);
    }
    if (sortKey === 'tokens') {
      const cmp = compareNumbers(sumTokenCounts(a.tokens), sumTokenCounts(b.tokens), sortDir);
      return cmp !== 0 ? cmp : b.date.localeCompare(a.date);
    }
    return compareStrings(a.date, b.date, sortDir);
  });
}

export function buildHourlyRows(
  hourly: HourlyViewRow[],
  sortKey: HourlySortKey,
  sortDir: SortDirection,
): HourlyViewRow[] {
  const byDay = new Map<string, { day: string; rows: HourlyViewRow[]; totalCost: number; totalTokens: number }>();

  for (const row of hourly) {
    const day = extractDatePart(row.datetime);
    const rowTokens = sumTokenCounts(row.tokens);
    const existing = byDay.get(day);
    if (existing) {
      existing.rows.push(row);
      existing.totalCost += row.cost;
      existing.totalTokens += rowTokens;
    } else {
      byDay.set(day, {
        day,
        rows: [row],
        totalCost: row.cost,
        totalTokens: rowTokens,
      });
    }
  }

  const orderedDays = Array.from(byDay.values()).sort((a, b) => {
    if (sortKey === 'cost') {
      const cmp = compareNumbers(a.totalCost, b.totalCost, sortDir);
      return cmp !== 0 ? cmp : b.day.localeCompare(a.day);
    }
    if (sortKey === 'tokens') {
      const cmp = compareNumbers(a.totalTokens, b.totalTokens, sortDir);
      return cmp !== 0 ? cmp : b.day.localeCompare(a.day);
    }
    return compareStrings(a.day, b.day, sortDir);
  });

  const result: HourlyViewRow[] = [];
  for (const group of orderedDays) {
    group.rows.sort((a, b) => {
      if (sortKey === 'cost') {
        const cmp = compareNumbers(a.cost, b.cost, sortDir);
        return cmp !== 0 ? cmp : b.datetime.localeCompare(a.datetime);
      }
      if (sortKey === 'tokens') {
        const cmp = compareNumbers(sumTokenCounts(a.tokens), sumTokenCounts(b.tokens), sortDir);
        return cmp !== 0 ? cmp : b.datetime.localeCompare(a.datetime);
      }
      return compareStrings(a.datetime, b.datetime, sortDir);
    });
    result.push(...group.rows);
  }
  return result;
}

export function sortMonthlyRows(
  monthly: MonthlyViewRow[],
  sortKey: MonthlySortKey,
  sortDir: SortDirection,
): MonthlyViewRow[] {
  return [...monthly].sort((a, b) => {
    if (sortKey === 'cost') {
      const cmp = compareNumbers(a.cost, b.cost, sortDir);
      return cmp !== 0 ? cmp : b.month.localeCompare(a.month);
    }
    if (sortKey === 'tokens') {
      const cmp = compareNumbers(sumTokenCounts(a.tokens), sumTokenCounts(b.tokens), sortDir);
      return cmp !== 0 ? cmp : b.month.localeCompare(a.month);
    }
    if (sortKey === 'models') {
      const cmp = compareNumbers(a.models.length, b.models.length, sortDir);
      return cmp !== 0 ? cmp : b.month.localeCompare(a.month);
    }
    return compareStrings(a.month, b.month, sortDir);
  });
}

export interface MonthlyTotals {
  uniqueModels: number;
  tokens: TokenCounts;
  cost: number;
}

export function computeMonthlyTotals(monthly: MonthlyViewRow[]): MonthlyTotals {
  const uniqueModels = new Set<string>();
  const tokens = emptyTokenCounts();
  let cost = 0;

  for (const row of monthly) {
    for (const model of row.models) {
      uniqueModels.add(model);
    }
    tokens.input += row.tokens.input;
    tokens.output += row.tokens.output;
    tokens.cacheRead += row.tokens.cacheRead;
    tokens.cacheWrite += row.tokens.cacheWrite;
    tokens.reasoning += row.tokens.reasoning;
    cost += row.cost;
  }

  return {
    uniqueModels: uniqueModels.size,
    tokens,
    cost,
  };
}

export interface HeatmapMonthLabel {
  weekIndex: number;
  label: string;
}

export interface HeatmapMetrics {
  visibleWeeks: Array<Array<GraphCell | null>>;
  monthLabels: HeatmapMonthLabel[];
  hasAnyCell: boolean;
  activeDays: number;
  longestStreak: number;
  currentStreak: number;
  bestDay?: { date: string; cost: number };
  avgCostPerActiveDay: number;
  firstCellCoord: string | null;
}

export function buildHeatmapMetrics(
  weeks: Array<Array<GraphCell | null>>,
  weekCount: 26 | 52,
  stats: StatsSummary,
): HeatmapMetrics {
  const visibleWeeks = weeks.slice(-weekCount);
  const monthLabels: HeatmapMonthLabel[] = [];
  let prevMonth = '';
  let lastLabelWeek = -3;

  let hasAnyCell = false;
  let activeDays = 0;
  let totalCost = 0;
  let bestDay: { date: string; cost: number } | undefined;
  let firstCellCoord: string | null = null;

  for (let w = 0; w < visibleWeeks.length; w++) {
    const col = visibleWeeks[w] ?? [];
    const firstNonNull = col.find((c): c is GraphCell => c !== null);
    if (firstNonNull) {
      const monthKey = firstNonNull.date.slice(0, 7);
      if (monthKey !== prevMonth) {
        if (w - lastLabelWeek >= 2) {
          monthLabels.push({
            weekIndex: w,
            label: formatMonthLabel(firstNonNull.date),
          });
          lastLabelWeek = w;
        }
        prevMonth = monthKey;
      }
    }

    for (let d = 0; d < 7; d++) {
      const cell = col[d] ?? null;
      if (!cell) continue;
      hasAnyCell = true;
      if (firstCellCoord === null) {
        firstCellCoord = `${w}:${d}`;
      }
      if (cell.intensity > 0) {
        activeDays += 1;
        totalCost += cell.cost;
      }
      if (!bestDay || cell.cost > bestDay.cost) {
        bestDay = { date: cell.date, cost: cell.cost };
      }
    }
  }

  const resolvedActiveDays = hasAnyCell ? activeDays : stats.activeDays;
  const resolvedBestDay = bestDay ?? stats.bestDay;
  const avgCostPerActiveDay = hasAnyCell
    ? resolvedActiveDays > 0
      ? totalCost / resolvedActiveDays
      : 0
    : stats.avgCostPerActiveDay;

  return {
    visibleWeeks,
    monthLabels,
    hasAnyCell,
    activeDays: resolvedActiveDays,
    longestStreak: stats.longestStreak,
    currentStreak: stats.currentStreak,
    bestDay: resolvedBestDay,
    avgCostPerActiveDay,
    firstCellCoord,
  };
}

export interface GridCoord {
  w: number;
  d: number;
  cell: GraphCell;
}

function findClosestInWeek(
  weeks: Array<Array<GraphCell | null>>,
  w: number,
  targetDay: number,
): GridCoord | null {
  const col = weeks[w];
  if (!col) return null;
  for (let offset = 0; offset < 7; offset++) {
    const up = targetDay - offset;
    if (up >= 0 && col[up]) {
      return { w, d: up, cell: col[up]! };
    }
    const down = targetDay + offset;
    if (offset > 0 && down < 7 && col[down]) {
      return { w, d: down, cell: col[down]! };
    }
  }
  return null;
}

export function findAdjacentCell(
  weeks: Array<Array<GraphCell | null>>,
  w: number,
  d: number,
  key: string,
): GridCoord | null {
  if (key === 'ArrowRight') {
    for (let nw = w + 1; nw < weeks.length; nw++) {
      const exact = weeks[nw]?.[d];
      if (exact) return { w: nw, d, cell: exact };
    }
    for (let nw = w + 1; nw < weeks.length; nw++) {
      const closest = findClosestInWeek(weeks, nw, d);
      if (closest) return closest;
    }
    return null;
  }

  if (key === 'ArrowLeft') {
    for (let nw = w - 1; nw >= 0; nw--) {
      const exact = weeks[nw]?.[d];
      if (exact) return { w: nw, d, cell: exact };
    }
    for (let nw = w - 1; nw >= 0; nw--) {
      const closest = findClosestInWeek(weeks, nw, d);
      if (closest) return closest;
    }
    return null;
  }

  if (key === 'ArrowDown') {
    for (let nd = d + 1; nd < 7; nd++) {
      const cell = weeks[w]?.[nd];
      if (cell) return { w, d: nd, cell };
    }
    for (let nw = w + 1; nw < weeks.length; nw++) {
      for (let nd = 0; nd < 7; nd++) {
        const cell = weeks[nw]?.[nd];
        if (cell) return { w: nw, d: nd, cell };
      }
    }
    return null;
  }

  if (key === 'ArrowUp') {
    for (let nd = d - 1; nd >= 0; nd--) {
      const cell = weeks[w]?.[nd];
      if (cell) return { w, d: nd, cell };
    }
    for (let nw = w - 1; nw >= 0; nw--) {
      for (let nd = 6; nd >= 0; nd--) {
        const cell = weeks[nw]?.[nd];
        if (cell) return { w: nw, d: nd, cell };
      }
    }
    return null;
  }

  return null;
}
