<script lang="ts">
  import { untrack } from 'svelte';
  import type {
    DailyViewRow,
    DashboardNavigateEvent,
    DashboardQuery,
    DashboardSnapshot,
  } from '$lib/contracts/ipc';
  import { formatTokens, formatUsd, formatWeekdayShort, sumTokenCounts } from '$lib/format';
  import { HarnessBadge, resolveHarness } from '$lib/harnesses';
  import { DataTable, EmptyState, type ColumnDef } from '$lib/ui';
  import { sortDailyRows, type DailySortKey, type SortDirection } from './analytics/analytics';

  interface Props {
    snapshot: DashboardSnapshot;
    query: DashboardQuery;
    onnavigate: (handoff: DashboardNavigateEvent) => void;
  }

  let { snapshot, query, onnavigate }: Props = $props();
  void (() => onnavigate);

  let sortKey = $state<DailySortKey>('date');
  let sortDir = $state<SortDirection>('desc');
  let expandedDate = $state<string | null>(null);
  let lastQuerySort = untrack(() => query.sort);

  $effect(() => {
    if (query.sort && query.sort !== lastQuerySort) {
      lastQuerySort = query.sort;
      sortKey = query.sort;
      sortDir = 'desc';
    }
  });

  let sortedRows = $derived(sortDailyRows(snapshot.daily, sortKey, sortDir));

  function handleSort(key: string): void {
    if (key !== 'date' && key !== 'tokens' && key !== 'cost') return;
    if (sortKey === key) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortKey = key;
      sortDir = 'desc';
    }
  }

  function handleRowClick(row: DailyViewRow): void {
    expandedDate = expandedDate === row.date ? null : row.date;
  }

  const columns: ColumnDef<DailyViewRow>[] = [
    {
      key: 'date',
      label: 'Date',
      sortable: true,
      cell: dateCell,
    },
    {
      key: 'harnesses',
      label: 'Harnesses',
      align: 'end',
      numeric: true,
      render: (row) => row.sourceBreakdown.length,
    },
    {
      key: 'input',
      label: 'Input',
      align: 'end',
      numeric: true,
      render: (row) => formatTokens(row.tokens.input),
    },
    {
      key: 'output',
      label: 'Output',
      align: 'end',
      numeric: true,
      render: (row) => formatTokens(row.tokens.output),
    },
    {
      key: 'cacheRead',
      label: 'Cache read',
      align: 'end',
      numeric: true,
      render: (row) => formatTokens(row.tokens.cacheRead),
    },
    {
      key: 'cacheWrite',
      label: 'Cache write',
      align: 'end',
      numeric: true,
      render: (row) => formatTokens(row.tokens.cacheWrite),
    },
    {
      key: 'reasoning',
      label: 'Reasoning',
      align: 'end',
      numeric: true,
      render: (row) => formatTokens(row.tokens.reasoning),
    },
    {
      key: 'tokens',
      label: 'Total tokens',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatTokens(sumTokenCounts(row.tokens)),
    },
    {
      key: 'messages',
      label: 'Messages',
      align: 'end',
      numeric: true,
      render: (row) => row.messageCount,
    },
    {
      key: 'turns',
      label: 'Turns',
      align: 'end',
      numeric: true,
      render: (row) => row.turnCount ?? '—',
    },
    {
      key: 'cost',
      label: 'Cost',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatUsd(row.cost),
    },
  ];
</script>

{#snippet dateCell(row: DailyViewRow)}
  {@const weekday = formatWeekdayShort(row.date)}
  <span class="date-cell">
    <span class="ui-mono">{row.date}</span>
    {#if weekday}
      <span class="date-weekday">{weekday}</span>
    {/if}
  </span>
{/snippet}

{#snippet expandedRow(row: DailyViewRow)}
  <div class="breakdown-panel" data-expanded-date={row.date}>
    {#if row.sourceBreakdown.length === 0}
      <p class="breakdown-empty">No harness activity recorded for this day.</p>
    {:else}
      <div class="breakdown-list">
        {#each [...row.sourceBreakdown].sort((a, b) => b.cost - a.cost) as source (source.client)}
          {@const harness = resolveHarness(source.client)}
          <div class="breakdown-item">
            <span class="breakdown-harness">
              <HarnessBadge id={source.client} size={16} />
              <span>{source.label || harness.label}</span>
            </span>
            <span class="breakdown-tokens ui-mono">
              {formatTokens(sumTokenCounts(source.tokens))} tok
            </span>
            <span class="breakdown-detail ui-mono">
              in {formatTokens(source.tokens.input)} · out {formatTokens(source.tokens.output)} · cache {formatTokens(source.tokens.cacheRead)}
            </span>
            <span class="breakdown-cost ui-mono">{formatUsd(source.cost)}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<section class="analytics-view" data-view="daily">
  {#if snapshot.daily.length === 0}
    <EmptyState
      title="No daily activity in this range"
      body="Try selecting a wider date range or enabling additional harnesses."
    />
  {:else}
    <DataTable
      {columns}
      rows={sortedRows}
      {sortKey}
      {sortDir}
      onsort={handleSort}
      rowKey={(row) => row.date}
      onrowclick={handleRowClick}
      expandedRowKey={expandedDate}
      {expandedRow}
      ariaLabel="Daily usage"
    />
  {/if}
</section>

<style>
  .analytics-view {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }

  .date-cell {
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
  }

  .date-weekday {
    font-size: 11.5px;
    color: var(--muted);
  }

  .breakdown-panel {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .breakdown-empty {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  .breakdown-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .breakdown-item {
    display: grid;
    grid-template-columns: minmax(140px, 1fr) auto 1fr auto;
    align-items: center;
    gap: 14px;
    font-size: 12px;
    color: var(--fg);
  }

  .breakdown-harness {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
  }

  .breakdown-tokens {
    color: var(--fg);
  }

  .breakdown-detail {
    color: var(--muted);
    font-size: 11.5px;
  }

  .breakdown-cost {
    font-weight: 600;
    color: var(--fg);
    text-align: end;
  }
</style>
