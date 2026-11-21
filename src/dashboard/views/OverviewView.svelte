<script lang="ts">
  import type {
    DashboardNavigateEvent,
    DashboardQuery,
    DashboardSnapshot,
    TokenCounts,
  } from '$lib/contracts/ipc';
  import { formatPercent, formatUsd } from '$lib/format/money';
  import { formatTokens, formatTokensFull } from '$lib/format/tokens';
  import { harnessColorVar, resolveHarness } from '$lib/harnesses';
  import { theme } from '$lib/state/theme.svelte';
  import {
    DataTable,
    EmptyState,
    HarnessBadge,
    StackedBarChart,
    type ColumnDef,
    type StackedBarItem,
  } from '$lib/ui';

  interface Props {
    snapshot: DashboardSnapshot;
    query: DashboardQuery;
    onnavigate: (handoff: DashboardNavigateEvent) => void;
  }

  interface HarnessSplitRow {
    id: string;
    label: string;
    sessions: number;
    input: number;
    output: number;
    cacheRead: number;
    cacheWrite: number;
    reasoning: number;
    cost: number;
  }

  let { snapshot, query, onnavigate }: Props = $props();
  void (() => [query]);

  let sortKey = $state<keyof HarnessSplitRow>('cost');
  let sortDir = $state<'asc' | 'desc'>('desc');

  let chartBars = $derived.by((): StackedBarItem[] => {
    if (snapshot.chart.length > 0) {
      return snapshot.chart.map((bar) => ({
        label: bar.label,
        slot: bar.slot,
        segments: bar.segments.map((seg) => ({
          id: seg.harnessId,
          label: seg.label,
          color: seg.color || harnessColorVar(seg.harnessId, theme.resolved),
          value: seg.cost,
        })),
      }));
    }
    return [...snapshot.daily]
      .reverse()
      .slice(-30)
      .map((day) => ({
        label: day.date.slice(5),
        slot: day.date,
        segments: day.sourceBreakdown.map((sb) => ({
          id: sb.client,
          label: sb.label,
          color: harnessColorVar(sb.client, theme.resolved),
          value: sb.cost,
        })),
      }));
  });

  let topModels = $derived.by(() => {
    const sorted = [...snapshot.models].sort((a, b) => b.cost - a.cost).slice(0, 5);
    const maxCost = sorted[0]?.cost ?? 0;
    const totalCost = snapshot.kpis.totalCost > 0
      ? snapshot.kpis.totalCost
      : sorted.reduce((sum, item) => sum + item.cost, 0);

    return sorted.map((model) => ({
      ...model,
      sharePercent: totalCost > 0 ? (model.cost / totalCost) * 100 : 0,
      barWidthPercent: maxCost > 0 ? Math.max(4, (model.cost / maxCost) * 100) : 0,
    }));
  });

  let harnessRows = $derived.by((): HarnessSplitRow[] => {
    const tokenMap = new Map<string, TokenCounts>();
    const sessionCountFromDaily = new Map<string, number>();

    for (const day of snapshot.daily) {
      for (const sb of day.sourceBreakdown) {
        const prev = tokenMap.get(sb.client) ?? {
          input: 0,
          output: 0,
          cacheRead: 0,
          cacheWrite: 0,
          reasoning: 0,
        };
        prev.input += sb.tokens.input;
        prev.output += sb.tokens.output;
        prev.cacheRead += sb.tokens.cacheRead;
        prev.cacheWrite += sb.tokens.cacheWrite;
        prev.reasoning += sb.tokens.reasoning;
        tokenMap.set(sb.client, prev);
        sessionCountFromDaily.set(
          sb.client,
          (sessionCountFromDaily.get(sb.client) ?? 0) + 1,
        );
      }
    }

    const modelFallbackMap = new Map<
      string,
      { tokens: TokenCounts; sessions: number; cost: number }
    >();
    for (const m of snapshot.models) {
      const prev = modelFallbackMap.get(m.client) ?? {
        tokens: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, reasoning: 0 },
        sessions: 0,
        cost: 0,
      };
      prev.tokens.input += m.tokens.input;
      prev.tokens.output += m.tokens.output;
      prev.tokens.cacheRead += m.tokens.cacheRead;
      prev.tokens.cacheWrite += m.tokens.cacheWrite;
      prev.tokens.reasoning += m.tokens.reasoning;
      prev.sessions += m.sessionCount;
      prev.cost += m.cost;
      modelFallbackMap.set(m.client, prev);
    }

    const rows: HarnessSplitRow[] = snapshot.harnesses.map((h) => {
      const fromDaily = tokenMap.get(h.id);
      const fromModels = modelFallbackMap.get(h.id);
      const tokens = fromDaily ??
        fromModels?.tokens ?? {
          input: 0,
          output: 0,
          cacheRead: 0,
          cacheWrite: 0,
          reasoning: 0,
        };
      const meta = resolveHarness(h.id);
      return {
        id: h.id,
        label: h.label || meta.label,
        sessions: h.sessions ?? sessionCountFromDaily.get(h.id) ?? fromModels?.sessions ?? 0,
        input: tokens.input,
        output: tokens.output,
        cacheRead: tokens.cacheRead,
        cacheWrite: tokens.cacheWrite,
        reasoning: tokens.reasoning,
        cost: h.cost,
      };
    });

    rows.sort((a, b) => {
      const dir = sortDir === 'asc' ? 1 : -1;
      if (sortKey === 'label') {
        return a.label.localeCompare(b.label) * dir;
      }
      return ((a[sortKey] as number) - (b[sortKey] as number)) * dir;
    });

    return rows;
  });

  const harnessColumns: ColumnDef<HarnessSplitRow>[] = [
    { key: 'label', label: 'Harness', sortable: true },
    {
      key: 'sessions',
      label: 'Sessions',
      numeric: true,
      sortable: true,
      render: (row) => formatTokensFull(row.sessions),
    },
    {
      key: 'input',
      label: 'Input',
      numeric: true,
      sortable: true,
      render: (row) => formatTokens(row.input),
      title: (row) => formatTokensFull(row.input),
    },
    {
      key: 'output',
      label: 'Output',
      numeric: true,
      sortable: true,
      render: (row) => formatTokens(row.output),
      title: (row) => formatTokensFull(row.output),
    },
    {
      key: 'cacheRead',
      label: 'Cache read',
      numeric: true,
      sortable: true,
      render: (row) => formatTokens(row.cacheRead),
      title: (row) => formatTokensFull(row.cacheRead),
    },
    {
      key: 'cacheWrite',
      label: 'Cache write',
      numeric: true,
      sortable: true,
      render: (row) => formatTokens(row.cacheWrite),
      title: (row) => formatTokensFull(row.cacheWrite),
    },
    {
      key: 'reasoning',
      label: 'Reasoning',
      numeric: true,
      sortable: true,
      render: (row) => formatTokens(row.reasoning),
      title: (row) => formatTokensFull(row.reasoning),
    },
    {
      key: 'cost',
      label: 'Cost',
      numeric: true,
      sortable: true,
      render: (row) => formatUsd(row.cost),
    },
  ];

  function handleHarnessSort(key: string): void {
    const typedKey = key as keyof HarnessSplitRow;
    if (sortKey === typedKey) {
      sortDir = sortDir === 'desc' ? 'asc' : 'desc';
    } else {
      sortKey = typedKey;
      sortDir = typedKey === 'label' ? 'asc' : 'desc';
    }
  }
</script>

<section class="overview-view" data-view="overview">
  <div class="overview-top-grid">
    <section class="dash-card overview-chart-card" aria-labelledby="overview-chart-title">
      <header class="dash-card-head">
        <h2 id="overview-chart-title" class="dash-card-title">Daily cost by harness</h2>
      </header>

      {#if chartBars.length > 0}
        <StackedBarChart
          bars={chartBars}
          valueFormat="usd"
          height={220}
          ariaLabel="Daily cost by harness"
        />
      {:else}
        <EmptyState
          title="No usage in this range"
          body="Try expanding the date range or clearing harness filters."
        />
      {/if}
    </section>

    <section class="dash-card overview-top-models-card" aria-labelledby="overview-top-models-title">
      <header class="dash-card-head">
        <h2 id="overview-top-models-title" class="dash-card-title">Top 5 models</h2>
      </header>

      {#if topModels.length > 0}
        <div class="top-models-list">
          {#each topModels as model (`${model.client}:${model.model}`)}
            <button
              type="button"
              class="top-model-row"
              onclick={() => onnavigate({ tab: 'models', query: model.model })}
            >
              <div class="top-model-head">
                <span class="top-model-identity">
                  <HarnessBadge id={model.client} size={16} />
                  <span class="top-model-name" title={model.model}>{model.model}</span>
                </span>
                <span class="top-model-figures">
                  <span class="top-model-cost ui-mono">{formatUsd(model.cost)}</span>
                  <span class="top-model-share ui-mono">{formatPercent(model.sharePercent)}</span>
                </span>
              </div>
              <div class="top-model-bar-track" aria-hidden="true">
                <div
                  class="top-model-bar-fill"
                  style:width={`${model.barWidthPercent}%`}
                  style:background={harnessColorVar(model.client, theme.resolved)}
                ></div>
              </div>
            </button>
          {/each}
        </div>
      {:else}
        <p class="overview-empty-inline">No model usage in this range.</p>
      {/if}
    </section>
  </div>

  <section class="dash-card overview-harness-card" aria-labelledby="overview-harness-title">
    <header class="dash-card-head">
      <h2 id="overview-harness-title" class="dash-card-title">Harness split</h2>
    </header>

    {#if harnessRows.length > 0}
      <DataTable
        columns={harnessColumns}
        rows={harnessRows}
        {sortKey}
        {sortDir}
        onsort={handleHarnessSort}
        rowKey={(row) => row.id}
        onrowclick={(row) => onnavigate({ tab: 'models', query: row.id })}
        ariaLabel="Harness split"
      >
        {#snippet cell(row, col)}
          {#if col.key === 'label'}
            <span class="harness-cell">
              <HarnessBadge id={row.id} size={16} />
              <span>{row.label}</span>
            </span>
          {:else if col.render}
            {col.render(row)}
          {/if}
        {/snippet}
      </DataTable>
    {:else}
      <EmptyState
        title="No harness data"
        body="No harness activity matched the current filters."
      />
    {/if}
  </section>
</section>
