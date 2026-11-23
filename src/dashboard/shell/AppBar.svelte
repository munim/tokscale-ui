<script lang="ts">
import {
  ipc,
  isMockMode,
  type DashboardQuery,
  type SourceHealthRow,
} from '$lib/contracts/ipc';
import { BrandMark } from '$lib/ui';

  interface Props {
    search: string;
    query: DashboardQuery;
    sources: SourceHealthRow[];
    syncing: boolean;
    onSearchChange: (value: string) => void;
    onClearSearch: () => void;
    onOpenSources: () => void;
    onOpenSettings: () => void;
    onRefresh: () => void;
  }

  let {
    search,
    query,
    sources,
    syncing,
    onSearchChange,
    onClearSearch,
    onOpenSources,
    onOpenSettings,
    onRefresh,
  }: Props = $props();

  let searchEl = $state<HTMLInputElement | undefined>(undefined);
  let exporting = $state(false);

  const isMac =
    typeof navigator !== 'undefined' && /Mac/i.test(navigator.userAgent);

  let activeCount = $derived(
    sources.filter((client) => client.status === 'ok').length,
  );
  let totalCount = $derived(sources.length);
  let sourcesLabel = $derived(
    totalCount > 0 ? `Sources ${activeCount} / ${totalCount}` : 'Sources 0 / 0',
  );

  export function focusSearch(): void {
    searchEl?.focus();
    searchEl?.select();
  }

  function triggerBrowserDownload(filename: string, content: string): void {
    const blob = new Blob([content], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(url);
  }

  async function handleExportJson(): Promise<void> {
    if (exporting) {
      return;
    }
    exporting = true;
    try {
      const json = await ipc.exportJson({
        ...query,
        activeClients: [...query.activeClients],
      });
      if (isMockMode() && json) {
        triggerBrowserDownload(`tokscale-${query.range}.json`, json);
      }
    } finally {
      exporting = false;
    }
  }
</script>

<header class="app-bar" class:is-mac={isMac} data-tauri-drag-region>
  <div class="app-brand-wrap" data-tauri-drag-region>
    <BrandMark size={20} />
    <span class="app-brand">tokscale</span>
  </div>

  <div class="app-search" data-tauri-drag-region="false">
    <svg
      class="app-search-icon"
      viewBox="0 0 16 16"
      width="14"
      height="14"
      fill="none"
      aria-hidden="true"
    >
      <circle cx="7" cy="7" r="4.75" stroke="currentColor" stroke-width="1.4" />
      <path d="M10.5 10.5L13.75 13.75" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
    </svg>
    <input
      bind:this={searchEl}
      class="app-search-input"
      type="search"
      placeholder="Search models, sessions, projects…"
      aria-label="Search models, sessions, projects"
      value={search}
      oninput={(event) => onSearchChange(event.currentTarget.value)}
    />
    {#if search.length > 0}
      <button
        type="button"
        class="app-search-clear"
        aria-label="Clear search"
        title="Clear search (Esc)"
        onclick={onClearSearch}
      >
        ×
      </button>
    {/if}
  </div>

  <div class="app-actions" data-tauri-drag-region="false">
    <button
      type="button"
      class="app-btn"
      onclick={onOpenSources}
      title="View sources and agents"
    >
      <span class="app-sources-dot" data-ok={activeCount > 0} aria-hidden="true"></span>
      <span>{sourcesLabel}</span>
    </button>

    <button
      type="button"
      class="app-btn"
      disabled={exporting}
      onclick={() => void handleExportJson()}
    >
      <span>{exporting ? 'Exporting…' : 'Export JSON'}</span>
    </button>

    <button
      type="button"
      class="app-btn app-btn-refresh"
      class:is-syncing={syncing}
      disabled={syncing}
      aria-label="Refresh (r)"
      title="Refresh (r)"
      onclick={onRefresh}
    >
      <svg
        class="app-refresh-icon"
        viewBox="0 0 16 16"
        width="14"
        height="14"
        fill="none"
        aria-hidden="true"
      >
        <path
          d="M13.25 8a5.25 5.25 0 1 1-1.54-3.71L13.25 5.8"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M13.25 2.75V5.8H10.2"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
      <span>Refresh</span>
      <kbd class="app-kbd" aria-hidden="true">r</kbd>
    </button>

    <button
      type="button"
      class="app-btn"
      onclick={onOpenSettings}
      aria-label="Settings"
      title={isMac ? 'Settings (⌘,)' : 'Settings (Ctrl+,)'}
    >
      <svg
        viewBox="0 0 16 16"
        width="14"
        height="14"
        fill="none"
        aria-hidden="true"
      >
        <circle cx="8" cy="8" r="2.25" stroke="currentColor" stroke-width="1.4" />
        <path
          d="M8 1.75v1.4M8 12.85v1.4M2.59 4.88l1.21.7M12.2 10.42l1.21.7M2.59 11.12l1.21-.7M12.2 5.58l1.21-.7"
          stroke="currentColor"
          stroke-width="1.4"
          stroke-linecap="round"
        />
      </svg>
      <span>Settings</span>
    </button>
  </div>
</header>
