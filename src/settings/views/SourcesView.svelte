<script lang="ts">
  import { HarnessBadge, StatusPill } from '$lib/ui';
  import type { SettingsStore } from '../store.svelte';

  interface Props {
    store: SettingsStore;
  }

  let { store }: Props = $props();

  const COMMON_CLIENTS = [
    'claude',
    'codex',
    'opencode',
    'cursor',
    'gemini',
    'copilot',
    'amp',
    'trae',
    'warp',
    'capy',
    'antigravity',
    'cline',
    'roocode',
    'kilo',
    'zed',
    'kiro',
    'goose',
    'hermes',
    'openclaw',
    'droid',
  ];

  let showAllSources = $state(false);
  let newScanClient = $state('codex');
  let newScanPath = $state('');
  let newOpencodeDb = $state('');
  let machineLabelDraft = $state('');
  let recoveringTz = $state(false);

  $effect(() => {
    machineLabelDraft = store.tokscaleSettings?.localMachineLabel ?? '';
  });

  let allSources = $derived(store.sources?.clients ?? []);
  let visibleSources = $derived.by(() => {
    if (showAllSources) return allSources;
    const detected = allSources.filter((s) => s.sessionsPathExists || s.status === 'danger' || s.warning);
    return detected.length > 0 ? detected : allSources.slice(0, 8);
  });

  let defaultClients = $derived(store.tokscaleSettings?.defaultClients ?? []);
  let extraScanEntries = $derived.by(() => {
    const map = store.tokscaleSettings?.extraScanPaths ?? {};
    const flat: Array<{ client: string; path: string }> = [];
    for (const [client, paths] of Object.entries(map)) {
      for (const p of paths) {
        flat.push({ client, path: p });
      }
    }
    return flat;
  });
  let opencodeDbPaths = $derived(store.tokscaleSettings?.opencodeDbPaths ?? []);

  function toggleDefaultClient(clientId: string): void {
    const set = new Set(defaultClients);
    if (set.has(clientId)) {
      set.delete(clientId);
    } else {
      set.add(clientId);
    }
    void store.updateTokscaleSettings({ defaultClients: Array.from(set) });
  }

  async function pickExtraScanFolder(): Promise<void> {
    const picked = await store.pickPath('folder');
    if (picked) {
      newScanPath = picked;
    }
  }

  function addExtraScanPath(): void {
    const client = newScanClient.trim();
    const path = newScanPath.trim();
    if (!client || !path) return;

    const next: Record<string, string[]> = structuredClone(
      store.tokscaleSettings?.extraScanPaths ?? {},
    );
    const existing = next[client] ?? [];
    if (!existing.includes(path)) {
      next[client] = [...existing, path];
      void store.updateTokscaleSettings({ extraScanPaths: next });
    }
    newScanPath = '';
  }

  function removeExtraScanPath(client: string, path: string): void {
    const next: Record<string, string[]> = structuredClone(
      store.tokscaleSettings?.extraScanPaths ?? {},
    );
    const remaining = (next[client] ?? []).filter((p) => p !== path);
    if (remaining.length === 0) {
      delete next[client];
    } else {
      next[client] = remaining;
    }
    void store.updateTokscaleSettings({ extraScanPaths: next });
  }

  async function pickOpencodeDb(): Promise<void> {
    const picked = await store.pickPath('file');
    if (picked) {
      newOpencodeDb = picked;
    }
  }

  function addOpencodeDb(): void {
    const path = newOpencodeDb.trim();
    if (!path || opencodeDbPaths.includes(path)) return;
    void store.updateTokscaleSettings({ opencodeDbPaths: [...opencodeDbPaths, path] });
    newOpencodeDb = '';
  }

  function removeOpencodeDb(path: string): void {
    void store.updateTokscaleSettings({
      opencodeDbPaths: opencodeDbPaths.filter((p) => p !== path),
    });
  }

  function commitMachineLabel(): void {
    const trimmed = machineLabelDraft.trim();
    const current = store.tokscaleSettings?.localMachineLabel ?? '';
    if (trimmed === current) return;
    void store.updateTokscaleSettings({
      localMachineLabel: trimmed.length > 0 ? trimmed : null,
    });
  }

  async function handleRecoverTimezone(): Promise<void> {
    recoveringTz = true;
    try {
      await store.recoverTimezone();
    } finally {
      recoveringTz = false;
    }
  }
</script>

<p class="settings-subtitle">
  Manage detected AI coding harnesses, CLI default filters, extra scan directories, and day-bucketing timezone.
</p>

<section class="s-card" class:is-highlighted={store.highlightedSettingId === 'sources-health'}>
  <div class="s-card-head">
    <div>
      <h2 class="s-card-title">Detected Local Harnesses ({store.detectedSourcesCount} / {allSources.length})</h2>
      <p class="s-card-desc">Scan locations reported by <code>tokscale clients</code>.</p>
    </div>
    {#if allSources.length > visibleSources.length || showAllSources}
      <button
        type="button"
        class="s-btn s-btn-sm"
        onclick={() => (showAllSources = !showAllSources)}
      >
        {showAllSources ? 'Detected only' : `Show all (${allSources.length})`}
      </button>
    {/if}
  </div>

  <div class="s-path-list">
    {#each visibleSources as row (row.client)}
      <div class="s-path-item">
        <div style="display: flex; align-items: center; gap: 8px; min-width: 0; flex: 1 1 auto;">
          <HarnessBadge id={row.client} size={16} />
          <div style="min-width: 0; flex: 1 1 auto;">
            <div style="font-size: 12px; font-weight: 600; color: var(--fg);">{row.label}</div>
            <div class="s-code" style="font-size: 11px; color: var(--muted);" title={row.sessionsPath}>
              {row.warning ?? row.sessionsPath}
            </div>
          </div>
        </div>
        <div class="s-row-control">
          <StatusPill
            tone={row.status}
            label={row.status === 'ok' ? 'Detected' : row.status === 'danger' ? 'Error' : 'Not found'}
          />
          {#if row.status === 'warn' || row.status === 'danger' || row.apiBacked}
            <button
              type="button"
              class="s-btn s-btn-sm"
              disabled={store.syncingClient === row.client}
              onclick={() => void store.retrySource(row.client)}
            >
              {store.syncingClient === row.client ? 'Syncing…' : 'Retry'}
            </button>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</section>

<section class="s-card" class:is-highlighted={store.highlightedSettingId === 'default-clients'}>
  <div class="s-card-head">
    <div>
      <h2 class="s-card-title">Default CLI Clients Filter</h2>
      <p class="s-card-desc">
        Applied when running <code>tokscale</code> without <code>-c / --client</code>. Leave empty to scan all clients.
      </p>
    </div>
    {#if defaultClients.length > 0}
      <button
        type="button"
        class="s-btn s-btn-sm s-btn-ghost"
        onclick={() => void store.updateTokscaleSettings({ defaultClients: [] })}
      >
        Clear ({defaultClients.length})
      </button>
    {/if}
  </div>

  <div class="s-chip-wrap" role="group" aria-label="Default CLI clients">
    {#each COMMON_CLIENTS as clientId (clientId)}
      {@const active = defaultClients.includes(clientId)}
      <button
        type="button"
        class="s-chip"
        class:is-active={active}
        aria-pressed={active}
        onclick={() => toggleDefaultClient(clientId)}
      >
        {clientId}
      </button>
    {/each}
  </div>
</section>

<div class="s-grid-2">
  <section class="s-card" class:is-highlighted={store.highlightedSettingId === 'extra-paths'}>
    <div>
      <h2 class="s-card-title">Extra Scan Paths</h2>
      <p class="s-card-desc">Persistent <code>scanner.extraScanPaths</code> roots outside default home directories.</p>
    </div>

    <div class="s-inline-form">
      <select class="s-select" aria-label="Client for extra scan path" bind:value={newScanClient}>
        {#each COMMON_CLIENTS as clientId (clientId)}
          <option value={clientId}>{clientId}</option>
        {/each}
      </select>
      <input
        type="text"
        class="s-input s-input-mono s-inline-grow"
        placeholder="/path/to/.codex"
        aria-label="Extra scan directory path"
        bind:value={newScanPath}
      />
      <button type="button" class="s-btn s-btn-sm" onclick={() => void pickExtraScanFolder()}>
        Browse…
      </button>
      <button
        type="button"
        class="s-btn s-btn-sm s-btn-primary"
        disabled={!newScanPath.trim()}
        onclick={addExtraScanPath}
      >
        Add
      </button>
    </div>

    {#if extraScanEntries.length > 0}
      <div class="s-path-list">
        {#each extraScanEntries as entry (`${entry.client}:${entry.path}`)}
          <div class="s-path-item">
            <span class="s-code" title={entry.path}>
              <strong>{entry.client}:</strong> {entry.path}
            </span>
            <button
              type="button"
              class="s-btn s-btn-sm s-btn-ghost"
              aria-label={`Remove ${entry.path}`}
              onclick={() => removeExtraScanPath(entry.client, entry.path)}
            >
              ×
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <span class="s-row-hint">No extra scan paths configured.</span>
    {/if}
  </section>

  <section class="s-card" class:is-highlighted={store.highlightedSettingId === 'opencode-db'}>
    <div>
      <h2 class="s-card-title">OpenCode SQLite Databases</h2>
      <p class="s-card-desc">Extra <code>scanner.opencodeDbPaths</code> merged with auto-discovery.</p>
    </div>

    <div class="s-inline-form">
      <input
        type="text"
        class="s-input s-input-mono s-inline-grow"
        placeholder="/path/to/opencode.db"
        aria-label="OpenCode database path"
        bind:value={newOpencodeDb}
      />
      <button type="button" class="s-btn s-btn-sm" onclick={() => void pickOpencodeDb()}>
        Browse…
      </button>
      <button
        type="button"
        class="s-btn s-btn-sm s-btn-primary"
        disabled={!newOpencodeDb.trim()}
        onclick={addOpencodeDb}
      >
        Add
      </button>
    </div>

    {#if opencodeDbPaths.length > 0}
      <div class="s-path-list">
        {#each opencodeDbPaths as dbPath (dbPath)}
          <div class="s-path-item">
            <span class="s-code" title={dbPath}>{dbPath}</span>
            <button
              type="button"
              class="s-btn s-btn-sm s-btn-ghost"
              aria-label={`Remove ${dbPath}`}
              onclick={() => removeOpencodeDb(dbPath)}
            >
              ×
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <span class="s-row-hint">Using default auto-discovered OpenCode database.</span>
    {/if}
  </section>
</div>

<section class="s-card" class:is-highlighted={store.highlightedSettingId === 'timezone'}>
  <h2 class="s-card-title">Device Identity & Bucket Timezone</h2>

  <div class="s-row is-first">
    <div class="s-row-copy">
      <span class="s-row-title">Local machine label</span>
      <span class="s-row-hint">Tag assigned to sessions outside named machineFolders (default: <code>local</code>).</span>
    </div>
    <div class="s-row-control">
      <input
        type="text"
        class="s-input s-input-mono"
        style="width: 170px;"
        placeholder="local"
        aria-label="Local machine label"
        bind:value={machineLabelDraft}
        onblur={commitMachineLabel}
        onkeydown={(e) => {
          if (e.key === 'Enter') commitMachineLabel();
        }}
      />
    </div>
  </div>

  <div class="s-row">
    <div class="s-row-copy">
      <span class="s-row-title">Bucket timezone</span>
      <span class="s-row-hint">
        Pinned automatically on first run to keep daily totals stable. Established valid pins are locked by Tokscale.
      </span>
    </div>
    <div class="s-row-control">
      <span class="s-code" style="padding: 4px 8px; background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius);">
        {store.tokscaleSettings?.bucketTimezone ?? 'Auto-detected'}
      </span>
      <button
        type="button"
        class="s-btn s-btn-sm"
        disabled={recoveringTz}
        onclick={() => void handleRecoverTimezone()}
      >
        {recoveringTz ? 'Checking…' : 'Verify / Auto'}
      </button>
    </div>
  </div>
</section>
