<script lang="ts">
  import { untrack } from 'svelte';
  import type {
    DashboardNavigateEvent,
    DashboardQuery,
    DashboardSnapshot,
  } from '$lib/contracts/ipc';
  import { formatTokens, formatUsd } from '$lib/format';
  import { HarnessBadge, resolveHarness } from '$lib/harnesses';
  import { DataTable, EmptyState, type ColumnDef } from '$lib/ui';
  import {
    buildModelTableModel,
    shortSessionId,
    type ModelSortKey,
    type ModelTableRow,
    type SortDirection,
  } from './analytics/analytics';

  interface Props {
    snapshot: DashboardSnapshot;
    query: DashboardQuery;
    onnavigate: (handoff: DashboardNavigateEvent) => void;
  }

  let { snapshot, query, onnavigate }: Props = $props();
  void (() => onnavigate);

  let sortKey = $state<ModelSortKey>(untrack(() => (query.sort === 'tokens' ? 'tokens' : 'cost')));
  let sortDir = $state<SortDirection>('desc');
  let collapsedGroupIds = $state<Set<string>>(new Set());

  let lastQuerySort = untrack(() => query.sort);
  let lastGroupBy = untrack(() => query.groupBy);

  $effect(() => {
    if (query.sort !== lastQuerySort) {
      lastQuerySort = query.sort;
      if (query.sort === 'tokens') {
        sortKey = 'tokens';
        sortDir = 'desc';
      } else if (query.sort === 'cost') {
        sortKey = 'cost';
        sortDir = 'desc';
      }
    }
  });

  $effect(() => {
    if (query.groupBy !== lastGroupBy) {
      lastGroupBy = query.groupBy;
      collapsedGroupIds = new Set();
    }
  });

  let tableModel = $derived(
    buildModelTableModel(snapshot.models, query.groupBy, sortKey, sortDir, collapsedGroupIds),
  );

  function handleSort(key: string): void {
    if (key !== 'model' && key !== 'cost') return;
    if (sortKey === key) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortKey = key;
      sortDir = key === 'model' ? 'asc' : 'desc';
    }
  }

  function toggleGroup(groupId: string): void {
    const next = new Set(collapsedGroupIds);
    if (next.has(groupId)) {
      next.delete(groupId);
    } else {
      next.add(groupId);
    }
    collapsedGroupIds = next;
  }

  let columns = $derived<ColumnDef<ModelTableRow>[]>([
    {
      key: 'model',
      label: 'Model',
      sortable: true,
      title: (item) =>
        item.kind === 'model' && item.row.msPer1KTokens !== undefined
          ? `${item.row.model} · ${item.row.msPer1KTokens} ms / 1K tokens`
          : undefined,
      cell: modelCell,
    },
    {
      key: 'harness',
      label: 'Harness',
      cell: harnessCell,
    },
    {
      key: 'provider',
      label: 'Provider',
      render: (item) => (item.kind === 'model' ? item.row.provider : ''),
    },
    {
      key: 'workspace',
      label: 'Workspace',
      render: (item) => (item.kind === 'model' ? item.row.workspaceLabel || '—' : ''),
    },
    {
      key: 'session',
      label: 'Session',
      render: (item) =>
        item.kind === 'model'
          ? tableModel.isSessionMode && item.row.sessionId
            ? shortSessionId(item.row.sessionId)
            : '—'
          : '',
    },
    {
      key: 'sessions',
      label: 'Sessions',
      align: 'end',
      numeric: true,
      render: (item) => (item.kind === 'model' ? item.row.sessionCount : ''),
    },
    {
      key: 'messages',
      label: 'Messages',
      align: 'end',
      numeric: true,
      render: (item) => (item.kind === 'model' ? item.row.messageCount : ''),
    },
    {
      key: 'input',
      label: 'Input',
      align: 'end',
      numeric: true,
      render: (item) => (item.kind === 'model' ? formatTokens(item.row.tokens.input) : ''),
    },
    {
      key: 'output',
      label: 'Output',
      align: 'end',
      numeric: true,
      render: (item) => (item.kind === 'model' ? formatTokens(item.row.tokens.output) : ''),
    },
    {
      key: 'cacheRead',
      label: 'Cache read',
      align: 'end',
      numeric: true,
      render: (item) => (item.kind === 'model' ? formatTokens(item.row.tokens.cacheRead) : ''),
    },
    {
      key: 'cacheWrite',
      label: 'Cache write',
      align: 'end',
      numeric: true,
      render: (item) => (item.kind === 'model' ? formatTokens(item.row.tokens.cacheWrite) : ''),
    },
    {
      key: 'reasoning',
      label: 'Reasoning',
      align: 'end',
      numeric: true,
      render: (item) => (item.kind === 'model' ? formatTokens(item.row.tokens.reasoning) : ''),
    },
    {
      key: 'cost',
      label: 'Cost',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (item) => (item.kind === 'model' ? formatUsd(item.row.cost) : ''),
    },
  ]);
</script>

{#snippet modelCell(item: ModelTableRow)}
  {#if item.kind === 'model'}
    <span
      class="model-name"
      title={item.row.msPer1KTokens !== undefined
        ? `${item.row.msPer1KTokens} ms / 1K tokens`
        : undefined}
    >
      {item.row.model}
    </span>
  {/if}
{/snippet}

{#snippet harnessCell(item: ModelTableRow)}
  {#if item.kind === 'model'}
    {@const harness = resolveHarness(item.row.client)}
    <span class="harness-cell">
      <HarnessBadge id={item.row.client} size={16} />
      <span>{harness.label}</span>
    </span>
  {/if}
{/snippet}

{#snippet groupHeader(groupId: string)}
  {@const meta = tableModel.groups.get(groupId)}
  {@const isCollapsed = collapsedGroupIds.has(groupId)}
  {#if meta}
    <button
      type="button"
      class="group-toggle"
      aria-expanded={!isCollapsed}
      onclick={() => toggleGroup(groupId)}
    >
      <span class="group-chevron" aria-hidden="true">{isCollapsed ? '▸' : '▾'}</span>
      <span class="group-label">{meta.label}</span>
      <span class="group-count">({meta.rows.length})</span>
      <span class="group-spacer"></span>
      <span class="group-stat ui-mono">{formatTokens(meta.subtotalTokens)} tok</span>
      <span class="group-stat ui-mono">{formatUsd(meta.subtotalCost)}</span>
    </button>
  {/if}
{/snippet}

<section class="analytics-view" data-view="models">
  {#if snapshot.models.length === 0}
    <EmptyState
      title="No models in this range"
      body="Try widening the date range, clearing the search filter, or enabling more harnesses."
    />
  {:else}
    <DataTable
      {columns}
      rows={tableModel.rows}
      {sortKey}
      {sortDir}
      onsort={handleSort}
      groupBy={tableModel.isGrouped ? (item) => item.groupId : undefined}
      {groupHeader}
      rowKey={(item) => item.key}
      ariaLabel="Models breakdown"
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

  .model-name {
    font-weight: 500;
    color: var(--fg);
  }

  .harness-cell {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .group-toggle {
    appearance: none;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 0;
    text-align: start;
    cursor: pointer;
  }

  .group-toggle:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: 4px;
  }

  .group-chevron {
    width: 12px;
    color: var(--muted);
    flex-shrink: 0;
  }

  .group-label {
    font-weight: 600;
    color: var(--fg);
  }

  .group-count {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--muted);
  }

  .group-spacer {
    flex: 1;
  }

  .group-stat {
    font-size: 12px;
    color: var(--muted);
    margin-inline-start: 10px;
  }

  .analytics-view :global(tr.ui-table-row[data-row-key^='collapsed::']) {
    display: none;
  }
</style>
