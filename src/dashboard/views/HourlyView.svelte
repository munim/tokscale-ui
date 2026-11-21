<script lang="ts">
  import { untrack } from 'svelte';
  import type {
    DashboardNavigateEvent,
    DashboardQuery,
    DashboardSnapshot,
    HourlyViewRow,
  } from '$lib/contracts/ipc';
  import { formatTokens, formatUsd, formatWeekdayShort, sumTokenCounts } from '$lib/format';
  import { HarnessBadge } from '$lib/harnesses';
  import { DataTable, EmptyState, type ColumnDef } from '$lib/ui';
  import {
    buildHourlyRows,
    extractDatePart,
    extractHourPart,
    type HourlySortKey,
    type SortDirection,
  } from './analytics/analytics';

  interface Props {
    snapshot: DashboardSnapshot;
    query: DashboardQuery;
    onnavigate: (handoff: DashboardNavigateEvent) => void;
  }

  let { snapshot, query, onnavigate }: Props = $props();
  void (() => onnavigate);

  let sortKey = $state<HourlySortKey>('hour');
  let sortDir = $state<SortDirection>('desc');
  let lastQuerySort = untrack(() => query.sort);

  $effect(() => {
    if (query.sort && query.sort !== lastQuerySort) {
      lastQuerySort = query.sort;
      sortKey = query.sort === 'date' ? 'hour' : query.sort;
      sortDir = 'desc';
    }
  });

  let sortedRows = $derived(buildHourlyRows(snapshot.hourly, sortKey, sortDir));

  function handleSort(key: string): void {
    if (key !== 'hour' && key !== 'tokens' && key !== 'cost') return;
    if (sortKey === key) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortKey = key;
      sortDir = 'desc';
    }
  }

  const columns: ColumnDef<HourlyViewRow>[] = [
    {
      key: 'hour',
      label: 'Hour',
      sortable: true,
      cell: hourCell,
    },
    {
      key: 'harnesses',
      label: 'Harnesses',
      cell: harnessesCell,
    },
    {
      key: 'topModel',
      label: 'Top model',
      render: (row) => row.topModel || '—',
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

{#snippet hourCell(row: HourlyViewRow)}
  <span class="ui-mono">{extractHourPart(row.datetime)}</span>
{/snippet}

{#snippet harnessesCell(row: HourlyViewRow)}
  {#if row.clients.length === 0}
    <span>—</span>
  {:else}
    {@const visible = row.clients.slice(0, 4)}
    {@const overflow = row.clients.length - visible.length}
    <span class="harnesses-list">
      {#each visible as client (client)}
        <HarnessBadge id={client} size={16} />
      {/each}
      {#if overflow > 0}
        <span class="harness-overflow">+{overflow}</span>
      {/if}
    </span>
  {/if}
{/snippet}

{#snippet groupHeader(dayKey: string, dayRows: HourlyViewRow[])}
  {@const weekday = formatWeekdayShort(dayKey)}
  {@const dayTokens = dayRows.reduce((sum, r) => sum + sumTokenCounts(r.tokens), 0)}
  {@const dayCost = dayRows.reduce((sum, r) => sum + r.cost, 0)}
  <div class="day-group-header">
    <span class="day-group-title">
      <span class="ui-mono">{dayKey}</span>
      {#if weekday}
        <span class="day-group-weekday">{weekday}</span>
      {/if}
    </span>
    <span class="day-group-spacer"></span>
    <span class="day-group-stat ui-mono">{formatTokens(dayTokens)} tok</span>
    <span class="day-group-stat ui-mono">{formatUsd(dayCost)}</span>
  </div>
{/snippet}

<section class="analytics-view" data-view="hourly">
  {#if snapshot.hourly.length === 0}
    <EmptyState
      title="No hourly activity in this range"
      body="Try selecting a wider date range or enabling additional harnesses."
    />
  {:else}
    <DataTable
      {columns}
      rows={sortedRows}
      {sortKey}
      {sortDir}
      onsort={handleSort}
      groupBy={(row) => extractDatePart(row.datetime)}
      {groupHeader}
      rowKey={(row) => row.datetime}
      rowHeight={32}
      overscan={10}
      maxHeight={560}
      ariaLabel="Hourly usage"
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

  .harnesses-list {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .harness-overflow {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
    padding: 0 5px;
    height: 16px;
    line-height: 16px;
    border-radius: 999px;
    background: var(--fg-soft);
  }

  .day-group-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .day-group-title {
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
    color: var(--fg);
  }

  .day-group-weekday {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--muted);
  }

  .day-group-spacer {
    flex: 1;
  }

  .day-group-stat {
    font-size: 12px;
    color: var(--muted);
    margin-inline-start: 10px;
  }
</style>
