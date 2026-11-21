<script lang="ts">
  import { untrack } from 'svelte';
  import type {
    DashboardNavigateEvent,
    DashboardQuery,
    DashboardSnapshot,
    MonthlyViewRow,
  } from '$lib/contracts/ipc';
  import { formatTokens, formatUsd } from '$lib/format';
  import { DataTable, EmptyState, type ColumnDef } from '$lib/ui';
  import {
    computeMonthlyTotals,
    sortMonthlyRows,
    type MonthlySortKey,
    type SortDirection,
  } from './analytics/analytics';

  interface Props {
    snapshot: DashboardSnapshot;
    query: DashboardQuery;
    onnavigate: (handoff: DashboardNavigateEvent) => void;
  }

  let { snapshot, query, onnavigate }: Props = $props();
  void (() => onnavigate);

  let sortKey = $state<MonthlySortKey>('month');
  let sortDir = $state<SortDirection>('desc');
  let lastQuerySort = untrack(() => query.sort);

  $effect(() => {
    if (query.sort && query.sort !== lastQuerySort) {
      lastQuerySort = query.sort;
      sortKey = query.sort === 'date' ? 'month' : query.sort;
      sortDir = 'desc';
    }
  });

  let sortedRows = $derived(sortMonthlyRows(snapshot.monthly, sortKey, sortDir));
  let totals = $derived(computeMonthlyTotals(snapshot.monthly));
  let maxCost = $derived(Math.max(0.01, ...snapshot.monthly.map((row) => row.cost)));

  function handleSort(key: string): void {
    if (key !== 'month' && key !== 'models' && key !== 'cost') return;
    if (sortKey === key) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortKey = key;
      sortDir = 'desc';
    }
  }

  const columns: ColumnDef<MonthlyViewRow>[] = [
    {
      key: 'month',
      label: 'Month',
      sortable: true,
      cell: monthCell,
    },
    {
      key: 'models',
      label: 'Models',
      align: 'end',
      numeric: true,
      sortable: true,
      title: (row) => (row.models.length > 0 ? row.models.join(', ') : undefined),
      render: (row) => row.models.length,
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
      key: 'cost',
      label: 'Cost',
      align: 'end',
      numeric: true,
      sortable: true,
      cell: costCell,
    },
  ];
</script>

{#snippet monthCell(row: MonthlyViewRow)}
  <span class="ui-mono">{row.month}</span>
{/snippet}

{#snippet costCell(row: MonthlyViewRow)}
  {@const pct = Math.min(100, Math.max(0, (row.cost / maxCost) * 100))}
  <span class="cost-cell">
    <span class="cost-spark-track" aria-hidden="true">
      <span class="cost-spark-fill" style:width={`${pct}%`}></span>
    </span>
    <span class="cost-value ui-mono">{formatUsd(row.cost)}</span>
  </span>
{/snippet}

{#snippet footer()}
  <tr class="monthly-totals-row">
    <td>Total</td>
    <td class="is-end ui-table-num">{totals.uniqueModels}</td>
    <td class="is-end ui-table-num">{formatTokens(totals.tokens.input)}</td>
    <td class="is-end ui-table-num">{formatTokens(totals.tokens.output)}</td>
    <td class="is-end ui-table-num">{formatTokens(totals.tokens.cacheRead)}</td>
    <td class="is-end ui-table-num">{formatTokens(totals.tokens.cacheWrite)}</td>
    <td class="is-end ui-table-num">{formatTokens(totals.tokens.reasoning)}</td>
    <td class="is-end ui-table-num">{formatUsd(totals.cost)}</td>
  </tr>
{/snippet}

<section class="analytics-view" data-view="monthly">
  {#if snapshot.monthly.length === 0}
    <EmptyState
      title="No monthly data in this range"
      body="Monthly totals will appear here once usage logs are synced."
    />
  {:else}
    <DataTable
      {columns}
      rows={sortedRows}
      {sortKey}
      {sortDir}
      onsort={handleSort}
      rowKey={(row) => row.month}
      {footer}
      ariaLabel="Monthly usage"
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

  .cost-cell {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }

  .cost-spark-track {
    width: 56px;
    max-width: 64px;
    height: 6px;
    border-radius: 999px;
    background: var(--fg-soft);
    overflow: hidden;
    flex-shrink: 0;
  }

  .cost-spark-fill {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
  }

  .cost-value {
    min-width: 58px;
    text-align: end;
  }

  .monthly-totals-row td {
    color: var(--fg);
  }
</style>
