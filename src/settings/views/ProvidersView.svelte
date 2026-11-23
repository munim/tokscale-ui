<script lang="ts">
  import type { ProviderActionRequest, ProviderStatusItem } from '$lib/contracts/ipc';
  import { HarnessBadge, StatusPill } from '$lib/ui';
  import type { SettingsStore } from '../store.svelte';

  interface Props {
    store: SettingsStore;
  }

  let { store }: Props = $props();

  let selectedId = $state('cursor');
  let busyAction = $state<string | null>(null);
  let lastResultOutput = $state<string | null>(null);

  // Contextual per-provider inputs (secrets cleared immediately after submit)
  let accountLabel = $state('');
  let purgeCache = $state(false);
  let traeVariant = $state('ide');
  let warpToken = $state('');
  let warpIsCookie = $state(false);
  let capyApiKey = $state('');
  let capyUsers = $state('');
  let hindsightApiUrl = $state('http://127.0.0.1:8888');
  let hindsightTenant = $state('default');
  let hindsightToken = $state('');

  let providers = $derived(store.providersStatus?.providers ?? []);
  let connectedProviders = $derived(providers.filter((p) => p.connected));
  let availableProviders = $derived(providers.filter((p) => !p.connected));

  let selected = $derived<ProviderStatusItem | null>(
    providers.find((p) => p.id === selectedId) ?? providers[0] ?? null,
  );

  $effect(() => {
    const hit = store.highlightedSettingId;
    if (hit && providers.some((p) => p.id === hit)) {
      selectedId = hit;
    }
  });

  async function runAction(req: ProviderActionRequest): Promise<void> {
    busyAction = `${req.provider}:${req.action}`;
    lastResultOutput = null;
    // Always clear secret fields immediately on submit before awaiting IPC.
    warpToken = '';
    capyApiKey = '';
    hindsightToken = '';
    try {
      const res = await store.runProviderAction(req);
      if (res.output) {
        lastResultOutput = res.output;
      }
      if (res.ok) {
        accountLabel = '';
      }
    } finally {
      busyAction = null;
    }
  }
</script>

<div class="providers-split">
  <aside class="providers-list-pane" aria-label="Providers list">
    {#if connectedProviders.length > 0}
      <div class="providers-group">
        <div class="providers-group-title">Connected ({connectedProviders.length})</div>
        {#each connectedProviders as p (p.id)}
          <button
            type="button"
            class="provider-list-btn"
            class:is-selected={selected?.id === p.id}
            onclick={() => {
              selectedId = p.id;
              lastResultOutput = null;
            }}
          >
            <span class="provider-list-left">
              <HarnessBadge id={p.id} size={16} />
              <span>{p.label}</span>
            </span>
            <span style="display: inline-flex; align-items: center; gap: 5px;">
              {#if p.accounts.length > 0}
                <span class="settings-nav-badge">{p.accounts.length}</span>
              {/if}
              <span class="provider-status-dot is-connected" aria-hidden="true"></span>
            </span>
          </button>
        {/each}
      </div>
    {/if}

    <div class="providers-group">
      <div class="providers-group-title">Available ({availableProviders.length})</div>
      {#each availableProviders as p (p.id)}
        <button
          type="button"
          class="provider-list-btn"
          class:is-selected={selected?.id === p.id}
          onclick={() => {
            selectedId = p.id;
            lastResultOutput = null;
          }}
        >
          <span class="provider-list-left">
            <HarnessBadge id={p.id} size={16} />
            <span>{p.label}</span>
          </span>
          <span class="provider-status-dot" aria-hidden="true"></span>
        </button>
      {/each}
    </div>
  </aside>

  <section class="providers-detail-pane" aria-label="Provider details">
    {#if store.loadingProviders && !selected}
      <div class="s-card">
        <span class="s-row-hint">Inspecting provider integrations…</span>
      </div>
    {:else if selected}
      <div class="s-card">
        <div class="s-card-head">
          <div style="display: inline-flex; align-items: center; gap: 10px;">
            <HarnessBadge id={selected.id} size={24} />
            <div>
              <h2 class="s-card-title" style="font-size: 14px;">{selected.label}</h2>
              <p class="s-card-desc">{selected.summary}</p>
            </div>
          </div>
          <StatusPill
            tone={selected.connected ? 'ok' : 'neutral'}
            label={selected.connected ? 'Connected' : 'Not connected'}
          />
        </div>

        {#if selected.diagnostics.length > 0}
          <div class="s-path-list">
            {#each selected.diagnostics as diag, idx (idx)}
              <div class="s-row-hint">• {diag}</div>
            {/each}
          </div>
        {/if}
      </div>

      {#if selected.id === 'cursor'}
        <div class="s-card">
          <div class="s-card-head">
            <h3 class="s-card-title">Saved Cursor Accounts</h3>
            <button
              type="button"
              class="s-btn s-btn-sm"
              disabled={busyAction !== null}
              onclick={() => void runAction({ provider: 'cursor', action: 'sync' })}
            >
              {busyAction === 'cursor:sync' ? 'Syncing…' : 'Sync usage'}
            </button>
          </div>

          {#if selected.accounts.length > 0}
            <div class="s-path-list">
              {#each selected.accounts as acct (acct.name)}
                <div class="s-path-item">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span class="provider-status-dot" class:is-connected={acct.active}></span>
                    <div>
                      <div style="font-size: 12.5px; font-weight: 600;">{acct.name}</div>
                      {#if acct.email || acct.plan}
                        <div class="s-row-hint">{[acct.email, acct.plan].filter(Boolean).join(' · ')}</div>
                      {/if}
                    </div>
                  </div>
                  <div class="s-row-control">
                    {#if acct.active}
                      <StatusPill tone="ok" label="Active" />
                    {:else}
                      <button
                        type="button"
                        class="s-btn s-btn-sm"
                        disabled={busyAction !== null}
                        onclick={() =>
                          void runAction({
                            provider: 'cursor',
                            action: 'switch',
                            accountName: acct.name,
                          })}
                      >
                        Switch
                      </button>
                    {/if}
                    <button
                      type="button"
                      class="s-btn s-btn-sm s-btn-danger"
                      disabled={busyAction !== null}
                      onclick={() =>
                        void runAction({
                          provider: 'cursor',
                          action: 'logout',
                          accountName: acct.name,
                          purgeCache,
                        })}
                    >
                      Logout
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <span class="s-row-hint">No saved Cursor accounts yet.</span>
          {/if}

          <div class="s-inline-form">
            <input
              type="text"
              class="s-input s-inline-grow"
              placeholder="Account label (optional, e.g. work)"
              aria-label="Cursor account label"
              bind:value={accountLabel}
            />
            <button
              type="button"
              class="s-btn s-btn-primary"
              disabled={busyAction !== null}
              onclick={() =>
                void runAction({
                  provider: 'cursor',
                  action: 'login',
                  accountName: accountLabel.trim() || undefined,
                })}
            >
              {busyAction === 'cursor:login' ? 'Detecting…' : 'Auto-detect Login'}
            </button>
          </div>

          <label class="s-checkbox-row">
            <input type="checkbox" bind:checked={purgeCache} />
            <span>Also purge cached Cursor usage when logging out</span>
          </label>
        </div>
      {:else if selected.id === 'codex'}
        <div class="s-card">
          <div class="s-card-head">
            <h3 class="s-card-title">Saved Codex Accounts</h3>
            <button
              type="button"
              class="s-btn s-btn-sm"
              disabled={busyAction !== null}
              onclick={() => void runAction({ provider: 'codex', action: 'status' })}
            >
              {busyAction === 'codex:status' ? 'Checking…' : 'Check Quota Status'}
            </button>
          </div>

          {#if selected.accounts.length > 0}
            <div class="s-path-list">
              {#each selected.accounts as acct (acct.name)}
                <div class="s-path-item">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span class="provider-status-dot" class:is-connected={acct.active}></span>
                    <div>
                      <div style="font-size: 12.5px; font-weight: 600;">{acct.name}</div>
                      {#if acct.plan}
                        <div class="s-row-hint">{acct.plan}</div>
                      {/if}
                    </div>
                  </div>
                  <div class="s-row-control">
                    {#if acct.active}
                      <StatusPill tone="ok" label="Active" />
                    {:else}
                      <button
                        type="button"
                        class="s-btn s-btn-sm"
                        disabled={busyAction !== null}
                        onclick={() =>
                          void runAction({
                            provider: 'codex',
                            action: 'switch',
                            accountName: acct.name,
                          })}
                      >
                        Switch
                      </button>
                    {/if}
                    <button
                      type="button"
                      class="s-btn s-btn-sm s-btn-danger"
                      disabled={busyAction !== null}
                      onclick={() =>
                        void runAction({
                          provider: 'codex',
                          action: 'remove',
                          accountName: acct.name,
                        })}
                    >
                      Remove
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <span class="s-row-hint">Import your active Codex CLI OAuth login to track or switch accounts.</span>
          {/if}

          <div class="s-inline-form">
            <input
              type="text"
              class="s-input s-inline-grow"
              placeholder="Account label (optional, e.g. personal)"
              aria-label="Codex account label"
              bind:value={accountLabel}
            />
            <button
              type="button"
              class="s-btn s-btn-primary"
              disabled={busyAction !== null}
              onclick={() =>
                void runAction({
                  provider: 'codex',
                  action: 'import',
                  accountName: accountLabel.trim() || undefined,
                })}
            >
              {busyAction === 'codex:import' ? 'Importing…' : 'Import Current OAuth'}
            </button>
          </div>
        </div>
      {:else if selected.id === 'trae'}
        <div class="s-card">
          <h3 class="s-card-title">Trae IDE Credentials & Sync</h3>
          <p class="s-card-desc">
            Auto-detects credentials from an installed Trae desktop client (IDE or Solo).
          </p>

          <div class="s-inline-form">
            <select class="s-select" aria-label="Trae variant" bind:value={traeVariant}>
              <option value="ide">Trae IDE</option>
              <option value="solo">Trae Solo</option>
            </select>
            <button
              type="button"
              class="s-btn s-btn-primary"
              disabled={busyAction !== null}
              onclick={() =>
                void runAction({ provider: 'trae', action: 'login', variant: traeVariant })}
            >
              {busyAction === 'trae:login' ? 'Detecting…' : 'Auto-detect Login'}
            </button>
            <button
              type="button"
              class="s-btn"
              disabled={busyAction !== null}
              onclick={() => void runAction({ provider: 'trae', action: 'sync' })}
            >
              {busyAction === 'trae:sync' ? 'Syncing…' : 'Sync Usage'}
            </button>
            {#if selected.connected}
              <button
                type="button"
                class="s-btn s-btn-danger"
                disabled={busyAction !== null}
                onclick={() =>
                  void runAction({ provider: 'trae', action: 'logout', variant: traeVariant })}
              >
                Logout
              </button>
            {/if}
          </div>
        </div>
      {:else if selected.id === 'warp'}
        <div class="s-card">
          <h3 class="s-card-title">Warp / Oz Authentication</h3>
          <p class="s-card-desc">
            Paste a Warp GraphQL bearer token or Cookie header. Passed directly to <code>tokscale warp login</code> and cleared immediately.
          </p>

          <div class="s-inline-form">
            <input
              type="password"
              class="s-input s-input-mono s-inline-grow"
              placeholder="Bearer token or Cookie header"
              aria-label="Warp token"
              bind:value={warpToken}
            />
            <button
              type="button"
              class="s-btn s-btn-primary"
              disabled={busyAction !== null || !warpToken.trim()}
              onclick={() =>
                void runAction({
                  provider: 'warp',
                  action: 'login',
                  token: warpToken,
                  isCookie: warpIsCookie,
                })}
            >
              Save Token
            </button>
          </div>

          <label class="s-checkbox-row">
            <input type="checkbox" bind:checked={warpIsCookie} />
            <span>Treat value as a Cookie header (<code>--cookie</code>)</span>
          </label>

          <div class="s-inline-form">
            <button
              type="button"
              class="s-btn"
              disabled={busyAction !== null}
              onclick={() => void runAction({ provider: 'warp', action: 'sync' })}
            >
              {busyAction === 'warp:sync' ? 'Syncing…' : 'Sync Usage'}
            </button>
            {#if selected.connected}
              <button
                type="button"
                class="s-btn s-btn-danger"
                disabled={busyAction !== null}
                onclick={() =>
                  void runAction({ provider: 'warp', action: 'logout', purgeCache })}
              >
                Logout
              </button>
            {/if}
          </div>
        </div>
      {:else if selected.id === 'capy'}
        <div class="s-card">
          <h3 class="s-card-title">Capy API Key & Sync</h3>
          <p class="s-card-desc">
            Create an API key under Settings → API in Capy. Optional member IDs/emails filter team usage.
          </p>

          <div class="s-inline-form">
            <input
              type="password"
              class="s-input s-input-mono s-inline-grow"
              placeholder="Capy API key"
              aria-label="Capy API key"
              bind:value={capyApiKey}
            />
          </div>
          <div class="s-inline-form">
            <input
              type="text"
              class="s-input s-inline-grow"
              placeholder="Optional member filter (comma-separated emails/IDs)"
              aria-label="Capy user filter"
              bind:value={capyUsers}
            />
            <button
              type="button"
              class="s-btn s-btn-primary"
              disabled={busyAction !== null || !capyApiKey.trim()}
              onclick={() =>
                void runAction({
                  provider: 'capy',
                  action: 'login',
                  apiKey: capyApiKey,
                  users: capyUsers
                    .split(',')
                    .map((s) => s.trim())
                    .filter(Boolean),
                })}
            >
              Save Key
            </button>
          </div>

          <div class="s-inline-form">
            <button
              type="button"
              class="s-btn"
              disabled={busyAction !== null}
              onclick={() => void runAction({ provider: 'capy', action: 'sync' })}
            >
              {busyAction === 'capy:sync' ? 'Syncing…' : 'Sync Usage'}
            </button>
            {#if selected.connected}
              <button
                type="button"
                class="s-btn s-btn-danger"
                disabled={busyAction !== null}
                onclick={() => void runAction({ provider: 'capy', action: 'logout' })}
              >
                Remove Key
              </button>
            {/if}
          </div>
        </div>
      {:else if selected.id === 'antigravity'}
        <div class="s-card">
          <h3 class="s-card-title">Antigravity Language Servers</h3>
          <p class="s-card-desc">
            Collects usage from running local Antigravity language servers into Tokscale's cache.
          </p>
          <div class="s-inline-form">
            <button
              type="button"
              class="s-btn s-btn-primary"
              disabled={busyAction !== null}
              onclick={() => void runAction({ provider: 'antigravity', action: 'sync' })}
            >
              {busyAction === 'antigravity:sync' ? 'Syncing…' : 'Sync Running Servers'}
            </button>
            <button
              type="button"
              class="s-btn s-btn-danger"
              disabled={busyAction !== null}
              onclick={() => void runAction({ provider: 'antigravity', action: 'purge_cache' })}
            >
              Purge Cache
            </button>
          </div>
        </div>
      {:else if selected.id === 'amp'}
        <div class="s-card">
          <h3 class="s-card-title">Amp Thread Sync</h3>
          <p class="s-card-desc">
            Exports Amp threads into Tokscale's local cache via the installed <code>amp</code> CLI.
          </p>
          <div class="s-inline-form">
            <button
              type="button"
              class="s-btn s-btn-primary"
              disabled={busyAction !== null}
              onclick={() => void runAction({ provider: 'amp', action: 'sync' })}
            >
              {busyAction === 'amp:sync' ? 'Exporting…' : 'Sync Amp Threads'}
            </button>
          </div>
        </div>
      {:else if selected.id === 'hindsight'}
        <div class="s-card">
          <h3 class="s-card-title">Hindsight Memory Backend</h3>
          <p class="s-card-desc">
            Syncs Hindsight LLM request logs into the local ledger cache.
          </p>
          <div class="s-inline-form">
            <input
              type="text"
              class="s-input s-input-mono s-inline-grow"
              placeholder="http://127.0.0.1:8888"
              aria-label="Hindsight API URL"
              bind:value={hindsightApiUrl}
            />
            <input
              type="text"
              class="s-input"
              style="width: 120px;"
              placeholder="default"
              aria-label="Hindsight tenant"
              bind:value={hindsightTenant}
            />
          </div>
          <div class="s-inline-form">
            <input
              type="password"
              class="s-input s-input-mono s-inline-grow"
              placeholder="Optional bearer token"
              aria-label="Hindsight bearer token"
              bind:value={hindsightToken}
            />
            <button
              type="button"
              class="s-btn s-btn-primary"
              disabled={busyAction !== null}
              onclick={() =>
                void runAction({
                  provider: 'hindsight',
                  action: 'sync',
                  apiUrl: hindsightApiUrl.trim() || undefined,
                  tenant: hindsightTenant.trim() || undefined,
                  token: hindsightToken.trim() || undefined,
                })}
            >
              {busyAction === 'hindsight:sync' ? 'Syncing…' : 'Sync Hindsight'}
            </button>
          </div>
        </div>
      {/if}

      {#if lastResultOutput}
        <div class="s-card">
          <div class="s-card-head">
            <span class="s-section-label">CLI Output</span>
            <button
              type="button"
              class="s-btn s-btn-sm s-btn-ghost"
              onclick={() => (lastResultOutput = null)}
            >
              Dismiss
            </button>
          </div>
          <pre class="s-output-pre">{lastResultOutput}</pre>
        </div>
      {/if}
    {/if}
  </section>
</div>
