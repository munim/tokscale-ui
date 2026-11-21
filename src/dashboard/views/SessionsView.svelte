<script lang="ts">
  import { onMount } from 'svelte';
  import type {
    DashboardNavigateEvent,
    DashboardQuery,
    DashboardSnapshot,
    SessionRow,
    SessionsQuery,
  } from '$lib/contracts/ipc';
  import { formatDuration } from '$lib/format/date';
  import { formatUsd } from '$lib/format/money';
  import { formatTokens, formatTokensFull } from '$lib/format/tokens';
  import { resolveHarness } from '$lib/harnesses/registry';
  import { DataTable, EmptyState, HarnessBadge, StatusPill, type ColumnDef } from '$lib/ui';
  import SessionDrawer from '../components/SessionDrawer.svelte';
  import {
    getPreferences,
    getSessionsView,
    onSessionTitlesUpdated,
    updatePreferences,
  } from './sessions/api';
  import {
    formatSessionCode,
    formatSessionTitle,
    sortSessionRows,
    type SessionSortKey,
  } from './sessions/helpers';

  interface Props {
    snapshot: DashboardSnapshot;
    query: DashboardQuery;
    onnavigate: (handoff: DashboardNavigateEvent) => void;
  }

  let { snapshot, query, onnavigate }: Props = $props();
  void (() => [snapshot, onnavigate]);

  let sessions = $state<SessionRow[]>([]);
  let loading = $state(true);
  let errorMessage = $state<string | null>(null);
  let enrichTitles = $state(true);
  let userToggledEnrich = false;

  let sortKey = $state<SessionSortKey>('cost');
  let sortDir = $state<'asc' | 'desc'>('desc');

  let titlesUpdatedCount = $state<number | null>(null);
  let titlesBadgeTimer: ReturnType<typeof setTimeout> | null = null;

  let selectedSession = $state<SessionRow | null>(null);
  let openerEl = $state<HTMLElement | null>(null);
  let tableHostEl = $state<HTMLDivElement | undefined>(undefined);

  let requestSeq = 0;

  function clearTitlesTimer(): void {
    if (titlesBadgeTimer !== null) {
      clearTimeout(titlesBadgeTimer);
      titlesBadgeTimer = null;
    }
  }

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

  async function fetchSessions(sessionsQuery: SessionsQuery): Promise<void> {
    const seq = ++requestSeq;
    loading = true;
    errorMessage = null;
    try {
      const payload = await getSessionsView(sessionsQuery);
      if (seq !== requestSeq) return;
      sessions = payload.sessions;
      if (selectedSession) {
        const refreshed = payload.sessions.find(
          (item) => item.sessionId === selectedSession?.sessionId,
        );
        if (refreshed) {
          selectedSession = refreshed;
        }
      }
    } catch (err) {
      if (seq !== requestSeq) return;
      errorMessage = err instanceof Error ? err.message : 'Unable to load sessions';
    } finally {
      if (seq === requestSeq) {
        loading = false;
      }
    }
  }

  $effect(() => {
    const currentQuery: SessionsQuery = {
      query: cloneQuery(query),
      enrichTitles,
    };
    void fetchSessions(currentQuery);
  });

  onMount(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    void getPreferences()
      .then((prefs) => {
        if (!active || userToggledEnrich) return;
        if (prefs.sessionTitleEnrichment !== enrichTitles) {
          enrichTitles = prefs.sessionTitleEnrichment;
        }
      })
      .catch(() => {
        // Keep default enrichment state if preferences fail to load
      });

    void onSessionTitlesUpdated((event) => {
      if (!active) return;
      titlesUpdatedCount = event.updatedCount;
      clearTitlesTimer();
      titlesBadgeTimer = setTimeout(() => {
        titlesUpdatedCount = null;
        titlesBadgeTimer = null;
      }, 4000);
      void fetchSessions({
        query: cloneQuery(query),
        enrichTitles,
      });
    }).then((fn) => {
      if (!active) {
        fn();
        return;
      }
      unlisten = fn;
    });

    return () => {
      active = false;
      clearTitlesTimer();
      unlisten?.();
    };
  });

  let sortedSessions = $derived(sortSessionRows(sessions, sortKey, sortDir));

  function handleSort(key: string): void {
    const nextKey = key as SessionSortKey;
    if (sortKey === nextKey) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
      return;
    }
    sortKey = nextKey;
    const textKeys = new Set<SessionSortKey>(['session', 'title', 'client']);
    sortDir = textKeys.has(nextKey) ? 'asc' : 'desc';
  }

  function handleToggleEnrich(event: Event): void {
    const input = event.currentTarget as HTMLInputElement;
    userToggledEnrich = true;
    enrichTitles = input.checked;
    void updatePreferences({ sessionTitleEnrichment: enrichTitles }).catch(() => {
      // Preference persistence is best-effort
    });
  }

  function findRowElement(sessionId: string): HTMLElement | null {
    if (!tableHostEl) return null;
    const rows = tableHostEl.querySelectorAll<HTMLElement>('tr[data-row-key]');
    for (const el of rows) {
      if (el.getAttribute('data-row-key') === sessionId) {
        return el;
      }
    }
    return null;
  }

  function handleRowClick(row: SessionRow): void {
    const active =
      typeof document !== 'undefined' && document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    const rowEl = findRowElement(row.sessionId);
    openerEl = (active && tableHostEl?.contains(active) ? active : rowEl) ?? null;
    selectedSession = row;
  }

  function handleCloseDrawer(): void {
    selectedSession = null;
  }

  const columns: ColumnDef<SessionRow>[] = [
    {
      key: 'session',
      label: 'Session',
      sortable: true,
      cell: sessionCell,
      title: (row) => formatSessionCode(row),
    },
    {
      key: 'title',
      label: 'Title',
      sortable: true,
      cell: titleCell,
      title: (row) => formatSessionTitle(row),
    },
    {
      key: 'client',
      label: 'Harness',
      sortable: true,
      cell: harnessCell,
      title: (row) => resolveHarness(row.client).label,
    },
    {
      key: 'models',
      label: 'Models',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => row.modelsUsed.length,
      title: (row) => (row.modelsUsed.length > 0 ? row.modelsUsed.join(', ') : undefined),
    },
    {
      key: 'messages',
      label: 'Messages',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatTokensFull(row.messageCount),
    },
    {
      key: 'duration',
      label: 'Duration',
      align: 'end',
      sortable: true,
      render: (row) => formatDuration(row.durationMinutes),
    },
    {
      key: 'input',
      label: 'Input',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatTokens(row.inputTokens),
      title: (row) => formatTokensFull(row.inputTokens),
    },
    {
      key: 'cacheRead',
      label: 'Cache read',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatTokens(row.cacheReadTokens),
      title: (row) => formatTokensFull(row.cacheReadTokens),
    },
    {
      key: 'cost',
      label: 'Cost',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatUsd(row.totalCost),
    },
  ];
</script>

{#snippet sessionCell(row: SessionRow)}
  <span class="session-code">{formatSessionCode(row)}</span>
{/snippet}

{#snippet titleCell(row: SessionRow)}
  <span class="session-title-text">{formatSessionTitle(row)}</span>
{/snippet}

{#snippet harnessCell(row: SessionRow)}
  {@const meta = resolveHarness(row.client)}
  <span class="session-harness-cell">
    <HarnessBadge id={row.client} size={16} />
    <span>{meta.label}</span>
  </span>
{/snippet}

<section class="sessions-view" data-view="sessions">
  <header class="view-toolbar">
    <div class="view-toolbar-left">
      <h2 class="view-heading">Sessions</h2>
      <span class="view-count">{sortedSessions.length}</span>
      {#if titlesUpdatedCount !== null}
        <StatusPill tone="neutral" label={`Titles updated (${titlesUpdatedCount})`} />
      {/if}
    </div>

    <div class="view-toolbar-right">
      <label
        class="enrich-toggle"
        title="Uses tokscale's free local summarizer. No cloud LLM is called."
      >
        <input
          type="checkbox"
          checked={enrichTitles}
          onchange={handleToggleEnrich}
          title="Uses tokscale's free local summarizer. No cloud LLM is called."
        />
        <span>Enrich titles locally</span>
      </label>
    </div>
  </header>

  {#if errorMessage}
    <EmptyState
      title="Could not load sessions"
      body={errorMessage}
      actionLabel="Retry"
      onaction={() => fetchSessions({ query: cloneQuery(query), enrichTitles })}
    />
  {:else if loading && sessions.length === 0}
    <div class="view-loading" role="status" aria-live="polite">Loading sessions…</div>
  {:else if sortedSessions.length === 0}
    <EmptyState
      title="No sessions in this range"
      body="Try expanding the date range or clearing the search and harness filters."
    />
  {:else}
    <div bind:this={tableHostEl} class="view-table-host">
      <DataTable
        {columns}
        rows={sortedSessions}
        {sortKey}
        {sortDir}
        onsort={handleSort}
        rowKey={(row) => row.sessionId}
        onrowclick={handleRowClick}
        ariaLabel="Sessions"
      />
    </div>
  {/if}

  <SessionDrawer
    session={selectedSession}
    sessions={sortedSessions}
    onselect={(row) => (selectedSession = row)}
    onclose={handleCloseDrawer}
    returnFocusTo={openerEl}
  />
</section>

<style>
  .sessions-view {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .view-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .view-toolbar-left,
  .view-toolbar-right {
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

  .enrich-toggle {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    color: var(--fg);
    cursor: pointer;
    user-select: none;
  }

  .enrich-toggle input[type='checkbox'] {
    accent-color: var(--accent);
    cursor: pointer;
  }

  .enrich-toggle input[type='checkbox']:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
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

  .view-table-host {
    width: 100%;
  }

  .session-code {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--muted);
  }

  .session-title-text {
    display: inline-block;
    max-width: 34ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    vertical-align: bottom;
    color: var(--fg);
    font-weight: 500;
  }

  .session-harness-cell {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
