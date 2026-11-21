<script lang="ts">
  import type {
    AgentPersonaRow,
    DashboardNavigateEvent,
    DashboardQuery,
    DashboardSnapshot,
    SourceHealthRow,
  } from '$lib/contracts/ipc';
  import { formatRelative } from '$lib/format/date';
  import { formatUsd } from '$lib/format/money';
  import { formatTokens, formatTokensFull } from '$lib/format/tokens';
  import { DataTable, EmptyState, HarnessBadge, StatusPill, type ColumnDef } from '$lib/ui';
  import { triggerSync } from './sessions/api';
  import {
    isApiBackedClient,
    sortAgentRows,
    truncateMiddle,
    type AgentSortKey,
  } from './sessions/helpers';

  interface Props {
    snapshot: DashboardSnapshot;
    query: DashboardQuery;
    onnavigate: (handoff: DashboardNavigateEvent) => void;
  }

  let { snapshot, query, onnavigate }: Props = $props();
  void (() => [query, onnavigate]);

  interface ClientSyncState {
    syncing: boolean;
    synced: boolean;
    warning?: string;
    lastSyncedAt?: number;
  }

  let agentSortKey = $state<AgentSortKey>('cost');
  let agentSortDir = $state<'asc' | 'desc'>('desc');
  let syncStates = $state<Record<string, ClientSyncState>>({});

  let sortedAgents = $derived(sortAgentRows(snapshot.agents, agentSortKey, agentSortDir));

  function handleAgentSort(key: string): void {
    const nextKey = key as AgentSortKey;
    if (agentSortKey === nextKey) {
      agentSortDir = agentSortDir === 'asc' ? 'desc' : 'asc';
      return;
    }
    agentSortKey = nextKey;
    agentSortDir = nextKey === 'agent' || nextKey === 'clients' ? 'asc' : 'desc';
  }

  async function handleClientSync(client: string): Promise<void> {
    syncStates = {
      ...syncStates,
      [client]: { syncing: true, synced: false },
    };

    try {
      const status = await triggerSync({ retryClient: client });
      const matchingWarning =
        status.warnings.find((w) => w.source === client)?.message ??
        status.warnings[0]?.message;

      syncStates = {
        ...syncStates,
        [client]: {
          syncing: false,
          synced: !matchingWarning,
          warning: matchingWarning,
          lastSyncedAt: status.lastSyncedAt,
        },
      };
    } catch (err) {
      syncStates = {
        ...syncStates,
        [client]: {
          syncing: false,
          synced: false,
          warning: err instanceof Error ? err.message : 'Sync failed',
        },
      };
    }
  }

  const agentColumns: ColumnDef<AgentPersonaRow>[] = [
    {
      key: 'agent',
      label: 'Agent',
      sortable: true,
      render: (row) => row.agent,
    },
    {
      key: 'clients',
      label: 'Harnesses',
      sortable: true,
      render: (row) => row.clients,
    },
    {
      key: 'messageCount',
      label: 'Messages',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatTokensFull(row.messageCount),
    },
    {
      key: 'input',
      label: 'Input',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatTokens(row.tokens.input),
      title: (row) => formatTokensFull(row.tokens.input),
    },
    {
      key: 'output',
      label: 'Output',
      align: 'end',
      numeric: true,
      sortable: true,
      render: (row) => formatTokens(row.tokens.output),
      title: (row) => formatTokensFull(row.tokens.output),
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

  const sourceColumns: ColumnDef<SourceHealthRow>[] = [
    {
      key: 'harness',
      label: 'Harness',
      cell: sourceHarnessCell,
      title: (row) => {
        const ts = syncStates[row.client]?.lastSyncedAt ?? row.lastSyncedAt;
        return ts ? `Last synced ${formatRelative(ts)}` : row.label;
      },
    },
    {
      key: 'sessionsPath',
      label: 'Scan path',
      cell: scanPathCell,
      title: (row) => row.sessionsPath,
    },
    {
      key: 'sessionsPathExists',
      label: 'Path exists',
      cell: pathExistsCell,
    },
    {
      key: 'messageCount',
      label: 'Messages',
      align: 'end',
      numeric: true,
      render: (row) => formatTokensFull(row.messageCount),
    },
    {
      key: 'headless',
      label: 'Headless',
      align: 'end',
      numeric: true,
      render: (row) =>
        row.headlessSupported ? formatTokensFull(row.headlessMessageCount) : '—',
    },
    {
      key: 'apiSync',
      label: 'API sync',
      cell: apiSyncCell,
    },
  ];
</script>

{#snippet sourceHarnessCell(row: SourceHealthRow)}
  <span class="source-harness-cell">
    <HarnessBadge id={row.client} size={16} />
    <span>{row.label}</span>
  </span>
{/snippet}

{#snippet scanPathCell(row: SourceHealthRow)}
  <code class="scan-path" title={row.sessionsPath}>
    {truncateMiddle(row.sessionsPath, 44)}
  </code>
{/snippet}

{#snippet pathExistsCell(row: SourceHealthRow)}
  {#if row.sessionsPathExists}
    <StatusPill tone="ok" label="Detected" />
  {:else}
    <StatusPill tone="warn" label="Not found on this machine" />
  {/if}
{/snippet}

{#snippet apiSyncCell(row: SourceHealthRow)}
  {#if isApiBackedClient(row)}
    {#if row.credentialed}
      {@const state = syncStates[row.client]}
      <div class="api-sync-cell">
        <button
          type="button"
          class="sync-btn"
          disabled={state?.syncing}
          onclick={() => handleClientSync(row.client)}
        >
          {#if state?.syncing}
            Syncing…
          {:else if state?.synced}
            Synced
          {:else}
            Sync
          {/if}
        </button>
        {#if state?.warning}
          <StatusPill tone="warn" label={state.warning} />
        {/if}
      </div>
    {:else}
      <StatusPill tone="neutral" label="Not signed in" />
    {/if}
  {:else}
    <span class="muted-dash">—</span>
  {/if}
{/snippet}

<section class="agents-view" data-view="agents">
  <section class="agents-panel" aria-labelledby="agents-personas-heading">
    <header class="panel-header">
      <div class="panel-title-group">
        <h2 id="agents-personas-heading" class="panel-heading">Sub-agent personas</h2>
        <span class="panel-count">{sortedAgents.length}</span>
      </div>
    </header>

    {#if sortedAgents.length === 0}
      <EmptyState
        title="No sub-agent activity"
        body="Sub-agent personas will appear here once sessions record agent usage."
      />
    {:else}
      <DataTable
        columns={agentColumns}
        rows={sortedAgents}
        sortKey={agentSortKey}
        sortDir={agentSortDir}
        onsort={handleAgentSort}
        rowKey={(row) => row.agent}
        maxHeight={360}
        ariaLabel="Sub-agent personas"
      />
    {/if}
  </section>

  <section class="agents-panel" aria-labelledby="agents-sources-heading">
    <header class="panel-header">
      <div class="panel-title-group">
        <h2 id="agents-sources-heading" class="panel-heading">
          Local harness sources and scan paths
        </h2>
        <span class="panel-count">{snapshot.sources.clients.length}</span>
      </div>
    </header>

    <div class="sources-note-block">
      <p class="sources-note">{snapshot.sources.note}</p>
      <div class="sources-roots">
        <span class="sources-roots-label">Headless roots:</span>
        {#if snapshot.sources.headlessRoots.length > 0}
          <ul class="sources-roots-list">
            {#each snapshot.sources.headlessRoots as root (root)}
              <li><code class="sources-root-chip">{root}</code></li>
            {/each}
          </ul>
        {:else}
          <span class="muted-dash">—</span>
        {/if}
      </div>
    </div>

    {#if snapshot.sources.clients.length === 0}
      <EmptyState
        title="No harness sources configured"
        body="No local or API-backed harness sources were reported."
      />
    {:else}
      <DataTable
        columns={sourceColumns}
        rows={snapshot.sources.clients}
        rowKey={(row) => row.client}
        rowHeight={36}
        maxHeight={420}
        ariaLabel="Local harness sources and scan paths"
      />
    {/if}
  </section>
</section>

<style>
  .agents-view {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .agents-panel {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .panel-title-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .panel-heading {
    margin: 0;
    font-size: 14px;
    font-weight: 650;
    color: var(--fg);
  }

  .panel-count {
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

  .sources-note-block {
    padding: 10px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--fg-soft);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .sources-note {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
  }

  .sources-roots {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 12px;
  }

  .sources-roots-label {
    color: var(--muted);
    font-weight: 600;
  }

  .sources-roots-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .sources-root-chip {
    display: inline-block;
    padding: 2px 7px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--fg);
  }

  .source-harness-cell {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .scan-path {
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--muted);
  }

  .api-sync-cell {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .sync-btn {
    appearance: none;
    height: 24px;
    padding: 0 10px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    font-family: var(--font-body);
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .sync-btn:hover:not(:disabled) {
    background: var(--accent-soft);
    border-color: color-mix(in oklch, var(--accent) 45%, var(--border));
  }

  .sync-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .sync-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .muted-dash {
    color: var(--muted);
  }
</style>
