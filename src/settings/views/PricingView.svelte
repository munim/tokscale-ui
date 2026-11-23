<script lang="ts">
  import type { CustomPricingEntry } from '$lib/contracts/ipc';
  import { StatusPill } from '$lib/ui';
  import ToggleSwitch from '../components/ToggleSwitch.svelte';
  import type { SettingsStore } from '../store.svelte';

  interface Props {
    store: SettingsStore;
    showAddForm?: boolean;
    oncloseaddform?: () => void;
  }

  let { store, showAddForm = false, oncloseaddform }: Props = $props();

  let lookupModelId = $state('claude-sonnet-4-5');

  // Override editor state
  let editingIndex = $state<number | null>(null);
  let editorOpen = $state(false);
  let draftScope = $state<'model' | 'provider'>('model');
  let draftProviderId = $state('');
  let draftModelId = $state('');
  let draftFree = $state(false);
  let draftInput = $state('3');
  let draftOutput = $state('15');
  let draftCacheRead = $state('0.3');
  let draftCacheWrite = $state('');
  let draftNotes = $state('');
  let savingOverrides = $state(false);

  // Model aliases editor
  let aliasFrom = $state('');
  let aliasTo = $state('');

  $effect(() => {
    if (showAddForm) {
      openAddEditor();
    }
  });

  let entries = $derived(store.customPricing?.entries ?? []);
  let aliasEntries = $derived(Object.entries(store.tokscaleSettings?.modelAliases ?? {}));

  function openAddEditor(): void {
    editingIndex = null;
    draftScope = 'model';
    draftProviderId = '';
    draftModelId = '';
    draftFree = false;
    draftInput = '';
    draftOutput = '';
    draftCacheRead = '';
    draftCacheWrite = '';
    draftNotes = '';
    editorOpen = true;
  }

  function openEditEditor(index: number, entry: CustomPricingEntry): void {
    editingIndex = index;
    draftScope = entry.scope;
    draftProviderId = entry.providerId ?? '';
    draftModelId = entry.modelId;
    const isFree =
      entry.inputPerMillion === 0 &&
      entry.outputPerMillion === 0 &&
      (entry.cacheReadPerMillion === undefined || entry.cacheReadPerMillion === 0) &&
      (entry.cacheWritePerMillion === undefined || entry.cacheWritePerMillion === 0);
    draftFree = isFree;
    draftInput = entry.inputPerMillion !== undefined ? String(entry.inputPerMillion) : '';
    draftOutput = entry.outputPerMillion !== undefined ? String(entry.outputPerMillion) : '';
    draftCacheRead = entry.cacheReadPerMillion !== undefined ? String(entry.cacheReadPerMillion) : '';
    draftCacheWrite =
      entry.cacheWritePerMillion !== undefined ? String(entry.cacheWritePerMillion) : '';
    draftNotes = entry.notes ?? '';
    editorOpen = true;
  }

  function closeEditor(): void {
    editorOpen = false;
    editingIndex = null;
    oncloseaddform?.();
  }

  function parseOptionalNumber(raw: string): number | undefined {
    const trimmed = raw.trim();
    if (!trimmed) return undefined;
    const n = Number(trimmed);
    return Number.isFinite(n) && n >= 0 ? n : undefined;
  }

  async function saveDraftOverride(): Promise<void> {
    const modelId = draftModelId.trim();
    if (!modelId) return;
    if (draftScope === 'provider' && !draftProviderId.trim()) return;

    const nextEntry: CustomPricingEntry = {
      scope: draftScope,
      providerId: draftScope === 'provider' ? draftProviderId.trim() : undefined,
      modelId,
      inputPerMillion: draftFree ? 0 : parseOptionalNumber(draftInput),
      outputPerMillion: draftFree ? 0 : parseOptionalNumber(draftOutput),
      cacheReadPerMillion: draftFree ? 0 : parseOptionalNumber(draftCacheRead),
      cacheWritePerMillion: draftFree ? 0 : parseOptionalNumber(draftCacheWrite),
      notes: draftNotes.trim() || undefined,
    };

    const nextList = [...entries];
    if (editingIndex !== null && editingIndex >= 0 && editingIndex < nextList.length) {
      nextList[editingIndex] = nextEntry;
    } else {
      nextList.push(nextEntry);
    }

    savingOverrides = true;
    try {
      const ok = await store.saveCustomPricing(nextList);
      if (ok) {
        closeEditor();
      }
    } finally {
      savingOverrides = false;
    }
  }

  async function deleteOverride(index: number): Promise<void> {
    savingOverrides = true;
    try {
      const nextList = entries.filter((_, i) => i !== index);
      await store.saveCustomPricing(nextList);
    } finally {
      savingOverrides = false;
    }
  }

  function addAlias(): void {
    const from = aliasFrom.trim();
    const to = aliasTo.trim();
    if (!from || !to) return;
    const next = { ...(store.tokscaleSettings?.modelAliases ?? {}), [from]: to };
    void store.updateTokscaleSettings({ modelAliases: next });
    aliasFrom = '';
    aliasTo = '';
  }

  function removeAlias(key: string): void {
    const next = { ...(store.tokscaleSettings?.modelAliases ?? {}) };
    delete next[key];
    void store.updateTokscaleSettings({ modelAliases: next });
  }

  function fmtRate(val: number | undefined): string {
    if (val === undefined) return '—';
    return `$${val.toFixed(val < 1 && val > 0 ? 3 : 2)}`;
  }

  function isFreeEntry(entry: CustomPricingEntry): boolean {
    return entry.inputPerMillion === 0 && entry.outputPerMillion === 0;
  }
</script>

<section class="s-card" class:is-highlighted={store.highlightedSettingId === 'price-lookup'}>
  <div class="s-card-head">
    <div>
      <h2 class="s-card-title">Quick Model Price Lookup</h2>
      <p class="s-card-desc">Verify exact per-1M rates resolved by <code>tokscale pricing</code>.</p>
    </div>
  </div>

  <div class="s-inline-form">
    <input
      type="text"
      class="s-input s-input-mono s-inline-grow"
      placeholder="e.g. claude-sonnet-4-5"
      aria-label="Model ID to look up"
      bind:value={lookupModelId}
      onkeydown={(e) => {
        if (e.key === 'Enter') void store.lookupPrice(lookupModelId);
      }}
    />
    <button
      type="button"
      class="s-btn s-btn-primary"
      disabled={store.lookingUpPrice || !lookupModelId.trim()}
      onclick={() => void store.lookupPrice(lookupModelId)}
    >
      {store.lookingUpPrice ? 'Looking up…' : 'Lookup'}
    </button>
  </div>

  {#if store.priceLookupError}
    <StatusPill tone="danger" label={store.priceLookupError} />
  {:else if store.priceLookupResult}
    {@const r = store.priceLookupResult}
    <div class="s-path-item">
      <div style="display: flex; flex-direction: column; gap: 2px;">
        <div style="display: flex; align-items: center; gap: 8px;">
          <span class="s-code" style="font-weight: 600;">{r.matchedKey ?? r.modelId}</span>
          {#if r.source}
            <StatusPill tone="ok" label={r.source} />
          {/if}
          {#if r.submissionSafe === false}
            <StatusPill tone="warn" label="Unpriced for submit" />
          {/if}
        </div>
        <span class="s-row-hint ui-mono">
          In {fmtRate(r.inputPerMillion)} · Out {fmtRate(r.outputPerMillion)} · Cache R {fmtRate(r.cacheReadPerMillion)} · Cache W {fmtRate(r.cacheWritePerMillion)} / 1M
        </span>
      </div>
    </div>
  {/if}
</section>

<section class="s-card" class:is-highlighted={store.highlightedSettingId === 'custom-pricing'}>
  <div class="s-card-head">
    <div>
      <h2 class="s-card-title">Custom Pricing Overrides</h2>
      <p class="s-card-desc">
        Exact-match overrides in <code>custom-pricing.json</code>. Checked before LiteLLM & OpenRouter.
      </p>
    </div>
    <div class="s-row-control">
      {#if store.customPricing}
        <StatusPill
          tone={store.customPricing.valid ? 'ok' : 'warn'}
          label={store.customPricing.valid ? 'Valid' : 'Needs fix'}
        />
      {/if}
      <button type="button" class="s-btn s-btn-sm" onclick={openAddEditor}>
        + Add Override
      </button>
    </div>
  </div>

  {#if editorOpen}
    <div
      style="padding: 12px; border-radius: var(--radius); border: 1px solid var(--border); background: var(--bg); display: flex; flex-direction: column; gap: 10px;"
    >
      <div class="s-inline-form">
        <select class="s-select" aria-label="Override scope" bind:value={draftScope}>
          <option value="model">Global model ID</option>
          <option value="provider">Provider-scoped</option>
        </select>

        {#if draftScope === 'provider'}
          <input
            type="text"
            class="s-input s-input-mono"
            style="width: 140px;"
            placeholder="provider (e.g. fireworks)"
            aria-label="Provider ID"
            bind:value={draftProviderId}
          />
        {/if}

        <input
          type="text"
          class="s-input s-input-mono s-inline-grow"
          placeholder="Exact model ID"
          aria-label="Override model ID"
          bind:value={draftModelId}
        />
      </div>

      <label class="s-checkbox-row">
        <input type="checkbox" bind:checked={draftFree} />
        <span>Free model ($0.00 / 1M tokens across all buckets)</span>
      </label>

      {#if !draftFree}
        <div class="s-inline-form">
          <label style="display: flex; flex-direction: column; gap: 3px;">
            <span class="s-row-hint">Input $/1M</span>
            <input type="number" step="0.01" min="0" class="s-input s-input-num" bind:value={draftInput} />
          </label>
          <label style="display: flex; flex-direction: column; gap: 3px;">
            <span class="s-row-hint">Output $/1M</span>
            <input type="number" step="0.01" min="0" class="s-input s-input-num" bind:value={draftOutput} />
          </label>
          <label style="display: flex; flex-direction: column; gap: 3px;">
            <span class="s-row-hint">Cache Read $/1M</span>
            <input type="number" step="0.01" min="0" class="s-input s-input-num" bind:value={draftCacheRead} />
          </label>
          <label style="display: flex; flex-direction: column; gap: 3px;">
            <span class="s-row-hint">Cache Write $/1M</span>
            <input type="number" step="0.01" min="0" class="s-input s-input-num" bind:value={draftCacheWrite} />
          </label>
        </div>
      {/if}

      <div class="s-inline-form" style="justify-content: flex-end;">
        <button type="button" class="s-btn s-btn-sm s-btn-ghost" onclick={closeEditor}>
          Cancel
        </button>
        <button
          type="button"
          class="s-btn s-btn-sm s-btn-primary"
          disabled={savingOverrides || !draftModelId.trim() || (draftScope === 'provider' && !draftProviderId.trim())}
          onclick={() => void saveDraftOverride()}
        >
          {savingOverrides ? 'Validating…' : 'Save & Validate'}
        </button>
      </div>
    </div>
  {/if}

  {#if entries.length > 0}
    <table class="s-table">
      <thead>
        <tr>
          <th>Model / Scope</th>
          <th class="is-num">In / 1M</th>
          <th class="is-num">Out / 1M</th>
          <th class="is-num">Cache R</th>
          <th class="is-num">Actions</th>
        </tr>
      </thead>
      <tbody>
        {#each entries as entry, idx (`${entry.scope}:${entry.providerId ?? ''}:${entry.modelId}:${idx}`)}
          <tr>
            <td>
              <div class="s-code" style="max-width: 240px;" title={entry.modelId}>{entry.modelId}</div>
              {#if entry.scope === 'provider' && entry.providerId}
                <span class="s-row-hint">provider: {entry.providerId}</span>
              {:else if isFreeEntry(entry)}
                <span class="s-row-hint">Free ($0)</span>
              {/if}
            </td>
            <td class="is-num">{fmtRate(entry.inputPerMillion)}</td>
            <td class="is-num">{fmtRate(entry.outputPerMillion)}</td>
            <td class="is-num">{fmtRate(entry.cacheReadPerMillion)}</td>
            <td class="is-num">
              <div class="s-row-control" style="justify-content: flex-end;">
                <button
                  type="button"
                  class="s-btn s-btn-sm s-btn-ghost"
                  onclick={() => openEditEditor(idx, entry)}
                >
                  Edit
                </button>
                <button
                  type="button"
                  class="s-btn s-btn-sm s-btn-ghost"
                  aria-label={`Delete ${entry.modelId}`}
                  disabled={savingOverrides}
                  onclick={() => void deleteOverride(idx)}
                >
                  ×
                </button>
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <span class="s-row-hint">No custom pricing overrides configured yet.</span>
  {/if}
</section>

<section class="s-card" class:is-highlighted={store.highlightedSettingId === 'model-aliases'}>
  <h2 class="s-card-title">Model Aliases & Report Options</h2>

  <div class="s-row is-first">
    <div class="s-row-copy">
      <span class="s-row-title">Include unused models</span>
      <span class="s-row-hint">Show models with zero recorded tokens in CLI reports.</span>
    </div>
    <ToggleSwitch
      checked={store.tokscaleSettings?.includeUnusedModels ?? false}
      label="Include unused models"
      onchange={(checked) => void store.updateTokscaleSettings({ includeUnusedModels: checked })}
    />
  </div>

  <div style="display: flex; flex-direction: column; gap: 8px; padding-top: 6px; border-top: 1px solid color-mix(in oklch, var(--border) 72%, transparent);">
    <span class="s-row-title">Model aliases (<code>modelAliases</code>)</span>
    <div class="s-inline-form">
      <input
        type="text"
        class="s-input s-input-mono s-inline-grow"
        placeholder="Alias (e.g. big-pickle)"
        aria-label="Model alias name"
        bind:value={aliasFrom}
      />
      <span class="s-row-hint">→</span>
      <input
        type="text"
        class="s-input s-input-mono s-inline-grow"
        placeholder="Canonical model ID (e.g. claude-opus-4-6)"
        aria-label="Canonical model ID"
        bind:value={aliasTo}
      />
      <button
        type="button"
        class="s-btn s-btn-sm s-btn-primary"
        disabled={!aliasFrom.trim() || !aliasTo.trim()}
        onclick={addAlias}
      >
        Add
      </button>
    </div>

    {#if aliasEntries.length > 0}
      <div class="s-path-list">
        {#each aliasEntries as [from, to] (from)}
          <div class="s-path-item">
            <span class="s-code">{from} → {to}</span>
            <button
              type="button"
              class="s-btn s-btn-sm s-btn-ghost"
              aria-label={`Remove alias ${from}`}
              onclick={() => removeAlias(from)}
            >
              ×
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <span class="s-row-hint">No custom model aliases configured.</span>
    {/if}
  </div>
</section>
