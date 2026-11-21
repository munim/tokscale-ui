<script lang="ts">
  import type {
    DashboardNavigateEvent,
    DashboardQuery,
    DashboardSnapshot,
    ProjectRow,
  } from '$lib/contracts/ipc';
  import { formatUsd } from '$lib/format/money';
  import { formatTokens, formatTokensFull } from '$lib/format/tokens';
  import { DataTable, EmptyState, type ColumnDef } from '$lib/ui';
  import { getSessionsView } from './sessions/api';
  import {
    formatLocalDateTime,
    sortProjectRows,
    type ProjectSortKey,
  } from './sessions/helpers';

  interface Props {
    snapshot: DashboardSnapshot;
    query: DashboardQuery;
    onnavigate: (handoff: DashboardNavigateEvent) => void;
  }

  let { snapshot, query, onnavigate }: Props = $props();
  void (() => [snapshot]);

  let projects = $state<ProjectRow[]>([]);
  let loading = $state(true);
  let errorMessage = $state<string | null>(null);

  let sortKey = $state<ProjectSortKey>('totalCost');
  let sortDir = $state<'asc' | 'desc'>('desc');

  let requestSeq = 0;

  function cloneQuery(q: DashboardQuery): DashboardQuery {
    return {
      range: q.range,
      customSince: q.customSince,
      customUntil: q.customUntil,
      groupBy: q.groupBy,
      activeClients: [...q.activeClients],
      search: q.search,
      sort: q.sort,
    };
  }

  async function fetchProjects(currentQuery: DashboardQuery): Promise<void> {
    const seq = ++requestSeq;
    loading = true;
    errorMessage = null;
    try {
      const payload = await getSessionsView({
        query: currentQuery,
        enrichTitles: false,
      });
      if (seq !== requestSeq) return;
      projects = payload.projects;
    } catch (err) {
      if (seq !== requestSeq) return;
      errorMessage = err instanceof Error ? err.message : 'Unable to load projects';
    } finally {
      if (seq === requestSeq) {
        loading = false;
      }
    }
  }

  $effect(() => {
    void fetchProjects(cloneQuery(query));
  });

  let sortedProjects = $derived(sortProjectRows(projects, sortKey, sortDir));

  function handleSort(key: string): void {
    const nextKey = key as ProjectSortKey;
    if (sortKey === nextKey) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
      return;
    }
    sortKey = nextKey;
    sortDir = nextKey === 'workspaceLabel' ? 'asc' : 'desc';
  }

  function handleRowClick(row: ProjectRow): void {
    onnavigate({
      tab: 'sessions',
      query: row.workspaceLabel,
    });
  }

  const columns: ColumnDef<ProjectRow>[] = [
    {
      key: 'workspaceLabel',
      label: 'Project',
      sortable: true,
      render: (row) => row.workspaceLabel || '(no workspace)',
      title: (row) => row.workspaceKey ?? undefined,
    },
    {
      key: 'sessionCount',
      label: 'Sessions',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatTokensFull(row.sessionCount),
    },
    {
      key: 'totalTokens',
      label: 'Tokens',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatTokens(row.totalTokens),
      title: (row) => formatTokensFull(row.totalTokens),
    },
    {
      key: 'totalCost',
      label: 'Cost',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatUsd(row.totalCost),
    },
    {
      key: 'models',
      label: 'Models',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => row.models.length,
      title: (row) => (row.models.length > 0 ? row.models.join(', ') : undefined),
    },
    {
      key: 'lastActive',
      label: 'Last active',
      align: 'end',
      sortable: true,
      render: (row) => formatLocalDateTime(row.lastActive),
    },
  ];
</script>

<section class="projects-view" data-view="projects">
  <header class="view-toolbar">
    <div class="view-toolbar-left">
      <h2 class="view-heading">Projects</h2>
      <span class="view-count">{sortedProjects.length}</span>
    </div>
  </header>

  {#if errorMessage}
    <EmptyState
      title="Could not load projects"
      body={errorMessage}
      actionLabel="Retry"
      onaction={() => fetchProjects(cloneQuery(query))}
    />
  {:else if loading && projects.length === 0}
    <div class="view-loading" role="status" aria-live="polite">Loading projects…</div>
  {:else if sortedProjects.length === 0}
    <EmptyState
      title="No projects in this range"
      body="Try expanding the date range or clearing the search and harness filters."
    />
  {:else}
    <DataTable
      {columns}
      rows={sortedProjects}
      {sortKey}
      {sortDir}
      onsort={handleSort}
      rowKey={(row) => row.workspaceKey ?? `no-ws:${row.workspaceLabel}`}
      onrowclick={handleRowClick}
      ariaLabel="Projects"
    />
  {/if}
</section>

<style>
  .projects-view {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .view-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .view-toolbar-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .view-heading {
    margin: 0;
    font-size: 14px;
    font-weight: 650;
    color: var(--fg);
  }

  .view-count {
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 7px;
    border-radius: 999px;
    background: var(--fg-soft);
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
  }

  .view-loading {
    padding: 24px 16px;
    text-align: center;
    color: var(--muted);
    font-size: 12.5px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }
</style>
