<script lang="ts">
  import type {
    DashboardNavigateEvent,
    DashboardQuery,
    DashboardSnapshot,
    GraphCell,
  } from '$lib/contracts/ipc';
  import { formatTokens, formatUsd, sumTokenCounts } from '$lib/format';
  import { EmptyState, KpiCard, SegmentedControl, type SegmentOption } from '$lib/ui';
  import { buildHeatmapMetrics, findAdjacentCell } from './analytics/analytics';

  interface Props {
    snapshot: DashboardSnapshot;
    query: DashboardQuery;
    onnavigate: (handoff: DashboardNavigateEvent) => void;
  }

  let { snapshot, query, onnavigate }: Props = $props();
  void (() => [query, onnavigate]);

  const WEEK_OPTIONS: SegmentOption[] = [
    { value: '26', label: '26 weeks' },
    { value: '52', label: '52 weeks' },
  ];

  let weekRange = $state<'26' | '52'>('52');
  let inspectedCell = $state<GraphCell | null>(null);
  let focusedCoord = $state<string | null>(null);

  let weekCount = $derived<26 | 52>(weekRange === '26' ? 26 : 52);
  let metrics = $derived(buildHeatmapMetrics(snapshot.graph.weeks, weekCount, snapshot.stats));

  const cellElements = new Map<string, HTMLButtonElement>();

  function registerCell(node: HTMLButtonElement, coord: string) {
    cellElements.set(coord, node);
    return {
      update(nextCoord: string) {
        if (nextCoord !== coord) {
          cellElements.delete(coord);
          coord = nextCoord;
          cellElements.set(coord, node);
        }
      },
      destroy() {
        cellElements.delete(coord);
      },
    };
  }

  function handleWeekRangeChange(next: string): void {
    weekRange = next === '26' ? '26' : '52';
    inspectedCell = null;
    focusedCoord = null;
  }

  function inspectCell(cell: GraphCell, coord: string): void {
    inspectedCell = cell;
    focusedCoord = coord;
  }

  function handleCellKeydown(
    event: KeyboardEvent,
    wIdx: number,
    dIdx: number,
    cell: GraphCell,
  ): void {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      inspectCell(cell, `${wIdx}:${dIdx}`);
      return;
    }

    if (
      event.key === 'ArrowRight' ||
      event.key === 'ArrowLeft' ||
      event.key === 'ArrowUp' ||
      event.key === 'ArrowDown'
    ) {
      event.preventDefault();
      const next = findAdjacentCell(metrics.visibleWeeks, wIdx, dIdx, event.key);
      if (next) {
        const nextCoord = `${next.w}:${next.d}`;
        focusedCoord = nextCoord;
        inspectedCell = next.cell;
        cellElements.get(nextCoord)?.focus();
      }
    }
  }

  let activeTabStop = $derived(focusedCoord ?? metrics.firstCellCoord);
</script>

<section class="analytics-view" data-view="stats">
  {#if !metrics.hasAnyCell}
    <EmptyState
      title="No activity history yet"
      body="Your contribution heatmap and streaks will appear once usage data is synced."
    />
  {:else}
    <div class="stats-kpi-grid">
      <KpiCard label="Active days" value={String(metrics.activeDays)} />
      <KpiCard label="Longest streak" value={`${metrics.longestStreak} days`} />
      <KpiCard label="Current streak" value={`${metrics.currentStreak} days`} />
      <KpiCard
        label="Best day"
        value={metrics.bestDay ? formatUsd(metrics.bestDay.cost) : '—'}
        hint={metrics.bestDay ? metrics.bestDay.date : 'No activity'}
      />
      <KpiCard label="Avg / active day" value={formatUsd(metrics.avgCostPerActiveDay)} />
    </div>

    <div class="heatmap-card">
      <div class="heatmap-header">
        <div class="heatmap-heading">
          <h2 class="heatmap-title">Activity heatmap</h2>
          <div class="heatmap-readout" aria-live="polite" data-testid="heatmap-readout">
            {#if inspectedCell}
              <span class="ui-mono">{inspectedCell.date}</span>
              <span aria-hidden="true">·</span>
              <span class="ui-mono">{formatTokens(sumTokenCounts(inspectedCell.tokens))} tokens</span>
              <span aria-hidden="true">·</span>
              <span class="ui-mono">{formatUsd(inspectedCell.cost)}</span>
            {:else}
              <span class="readout-hint">Hover or focus a day to inspect usage</span>
            {/if}
          </div>
        </div>

        <SegmentedControl
          options={WEEK_OPTIONS}
          value={weekRange}
          onchange={handleWeekRangeChange}
          ariaLabel="Heatmap week count"
          size="sm"
        />
      </div>

      <div class="heatmap-scroll">
        <div
          class="heatmap-layout"
          style:--week-count={metrics.visibleWeeks.length}
        >
          <div class="heatmap-months" aria-hidden="true">
            {#each metrics.monthLabels as month (month.weekIndex)}
              <span class="heatmap-month-label" style:grid-column={month.weekIndex + 1}>
                {month.label}
              </span>
            {/each}
          </div>

          <div class="heatmap-weekdays" aria-hidden="true">
            <span class="weekday-label" style:grid-row={2}>Mon</span>
            <span class="weekday-label" style:grid-row={4}>Wed</span>
            <span class="weekday-label" style:grid-row={6}>Fri</span>
          </div>

          <div class="heatmap-grid" role="grid" aria-label="Contribution heatmap">
            {#each metrics.visibleWeeks as week, wIdx (wIdx)}
              <div class="heatmap-week" role="row">
                {#each week as cell, dIdx (dIdx)}
                  {#if cell}
                    {@const coord = `${wIdx}:${dIdx}`}
                    {@const totalTokens = sumTokenCounts(cell.tokens)}
                    <button
                      type="button"
                      class="heatmap-cell"
                      role="gridcell"
                      data-intensity={cell.intensity}
                      data-date={cell.date}
                      data-coord={coord}
                      tabindex={activeTabStop === coord ? 0 : -1}
                      aria-label={`${cell.date}: ${formatTokens(totalTokens)} tokens, ${formatUsd(cell.cost)}`}
                      title={`${cell.date} · ${formatTokens(totalTokens)} tokens · ${formatUsd(cell.cost)}`}
                      use:registerCell={coord}
                      onmouseenter={() => inspectCell(cell, coord)}
                      onfocus={() => inspectCell(cell, coord)}
                      onclick={() => inspectCell(cell, coord)}
                      onkeydown={(event) => handleCellKeydown(event, wIdx, dIdx, cell)}
                    ></button>
                  {:else}
                    <span class="heatmap-spacer" aria-hidden="true"></span>
                  {/if}
                {/each}
              </div>
            {/each}
          </div>
        </div>
      </div>

      <div class="heatmap-legend">
        <span class="legend-label">Less</span>
        {#each [0, 1, 2, 3, 4] as level (level)}
          <span class="legend-swatch" data-intensity={level} aria-hidden="true"></span>
        {/each}
        <span class="legend-label">More</span>
      </div>
    </div>
  {/if}
</section>

<style>
  .analytics-view {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
  }

  .stats-kpi-grid {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 10px;
  }

  @media (max-width: 960px) {
    .stats-kpi-grid {
      grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    }
  }

  .heatmap-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px 16px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    border: 1px solid var(--border);
    min-width: 0;
  }

  .heatmap-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: nowrap;
  }

  .heatmap-heading {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1 1 auto;
    min-width: 0;
  }

  .heatmap-title {
    margin: 0;
    font-size: 13.5px;
    font-weight: 600;
    color: var(--fg);
  }

  .heatmap-readout {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 18px;
    min-height: 18px;
    max-height: 18px;
    line-height: 1.2;
    font-size: 12px;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
  }

  .readout-hint {
    color: var(--muted);
  }

  .heatmap-scroll {
    overflow-x: auto;
    padding-bottom: 2px;
  }

  .heatmap-layout {
    display: grid;
    grid-template-columns: 28px auto;
    grid-template-rows: 16px auto;
    column-gap: 6px;
    row-gap: 4px;
    width: max-content;
  }

  .heatmap-months {
    grid-column: 2;
    grid-row: 1;
    display: grid;
    grid-template-columns: repeat(var(--week-count, 52), 12px);
    column-gap: 3px;
    align-items: end;
  }

  .heatmap-month-label {
    font-size: 10.5px;
    line-height: 1;
    color: var(--muted);
    white-space: nowrap;
    overflow: visible;
  }

  .heatmap-weekdays {
    grid-column: 1;
    grid-row: 2;
    display: grid;
    grid-template-rows: repeat(7, 12px);
    row-gap: 3px;
    align-items: center;
  }

  .weekday-label {
    font-size: 10.5px;
    line-height: 1;
    color: var(--muted);
    text-align: end;
    padding-inline-end: 2px;
  }

  .heatmap-grid {
    grid-column: 2;
    grid-row: 2;
    display: flex;
    gap: 3px;
  }

  .heatmap-week {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .heatmap-cell,
  .heatmap-spacer,
  .legend-swatch {
    width: 12px;
    height: 12px;
    border-radius: 2px;
    box-sizing: border-box;
  }

  .heatmap-cell {
    appearance: none;
    border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    padding: 0;
    margin: 0;
    cursor: pointer;
    display: block;
  }

  .heatmap-cell:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    position: relative;
    z-index: 1;
  }

  .legend-swatch {
    display: inline-block;
    border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
  }

  [data-intensity='0'] {
    background: var(--fg-soft);
  }

  [data-intensity='1'] {
    background: color-mix(in oklch, var(--accent) 25%, var(--surface));
  }

  [data-intensity='2'] {
    background: color-mix(in oklch, var(--accent) 45%, var(--surface));
  }

  [data-intensity='3'] {
    background: color-mix(in oklch, var(--accent) 70%, var(--surface));
  }

  [data-intensity='4'] {
    background: var(--accent-strong);
  }

  .heatmap-legend {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
  }

  .legend-label {
    font-size: 11px;
    color: var(--muted);
    margin: 0 2px;
  }
</style>
