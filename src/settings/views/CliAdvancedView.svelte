<script lang="ts">
  import { StatusPill } from '$lib/ui';
  import ToggleSwitch from '../components/ToggleSwitch.svelte';
  import type { SettingsStore } from '../store.svelte';

  interface Props {
    store: SettingsStore;
  }

  let { store }: Props = $props();

  const PALETTES = [
    'blue',
    'dusk',
    'green',
    'teal',
    'purple',
    'pink',
    'orange',
    'graphite',
    'lagoon',
    'tokyo-night',
    'catppuccin',
    'monochrome',
    'halloween',
  ];

  const LANGUAGES = [
    { value: 'en', label: 'English (en)' },
    { value: 'ko', label: 'Korean (ko)' },
    { value: 'ja', label: 'Japanese (ja)' },
    { value: 'zh-cn', label: 'Chinese (zh-cn)' },
  ];

  let customPathDraft = $state('');
  let timeoutSecDraft = $state('300');
  let refreshSecDraft = $state('60');

  $effect(() => {
    customPathDraft = store.preferences.customTokscalePath ?? '';
  });

  $effect(() => {
    if (store.tokscaleSettings) {
      timeoutSecDraft = String(Math.round(store.tokscaleSettings.nativeTimeoutMs / 1000));
      refreshSecDraft = String(Math.round(store.tokscaleSettings.autoRefreshMs / 1000));
    }
  });

  async function pickCustomBinary(): Promise<void> {
    const picked = await store.pickPath('file');
    if (picked) {
      customPathDraft = picked;
      await store.updatePreferences({ customTokscalePath: picked });
    }
  }

  function commitCustomPath(): void {
    const trimmed = customPathDraft.trim();
    const current = store.preferences.customTokscalePath ?? '';
    if (trimmed === current) return;
    void store.updatePreferences({
      customTokscalePath: (trimmed.length > 0 ? trimmed : null) as unknown as undefined,
    });
  }

  function commitNativeTimeout(): void {
    const secs = Number(timeoutSecDraft);
    if (!Number.isFinite(secs)) return;
    const clampedMs = Math.max(5_000, Math.min(3_600_000, Math.round(secs * 1000)));
    timeoutSecDraft = String(Math.round(clampedMs / 1000));
    if (clampedMs !== store.tokscaleSettings?.nativeTimeoutMs) {
      void store.updateTokscaleSettings({ nativeTimeoutMs: clampedMs });
    }
  }

  function commitAutoRefreshMs(): void {
    const secs = Number(refreshSecDraft);
    if (!Number.isFinite(secs)) return;
    const clampedMs = Math.max(30_000, Math.min(3_600_000, Math.round(secs * 1000)));
    refreshSecDraft = String(Math.round(clampedMs / 1000));
    if (clampedMs !== store.tokscaleSettings?.autoRefreshMs) {
      void store.updateTokscaleSettings({ autoRefreshMs: clampedMs });
    }
  }
</script>

<section class="s-card" class:is-highlighted={store.highlightedSettingId === 'cli-binary'}>
  <div class="s-card-head">
    <div>
      <h2 class="s-card-title">Tokscale CLI Binary</h2>
      <p class="s-card-desc">Companion locates <code>tokscale</code> via custom path, PATH, probe dirs, or bunx/npx.</p>
    </div>
    <StatusPill
      tone={store.cliInfo?.resolvedVia && store.cliInfo.resolvedVia !== 'none' ? 'ok' : 'danger'}
      label={store.cliInfo?.version
        ? `v${store.cliInfo.version} (${store.cliInfo.resolvedVia})`
        : store.cliInfo?.resolvedVia === 'none'
          ? 'Not found'
          : 'Checking…'}
    />
  </div>

  {#if store.cliInfo?.binaryPath}
    <div class="s-path-item">
      <span class="s-code" title={store.cliInfo.binaryPath}>{store.cliInfo.binaryPath}</span>
      <button
        type="button"
        class="s-btn s-btn-sm s-btn-ghost"
        onclick={() => void store.revealPath(store.cliInfo!.binaryPath!)}
      >
        Reveal
      </button>
    </div>
  {/if}

  <div style="display: flex; flex-direction: column; gap: 5px;">
    <label class="s-row-title" for="custom-cli-path">Custom tokscale binary override</label>
    <div class="s-inline-form">
      <input
        id="custom-cli-path"
        type="text"
        class="s-input s-input-mono s-inline-grow"
        placeholder="Auto-detect (leave blank)"
        aria-label="Custom tokscale binary path"
        bind:value={customPathDraft}
        onblur={commitCustomPath}
        onkeydown={(e) => {
          if (e.key === 'Enter') commitCustomPath();
        }}
      />
      <button type="button" class="s-btn s-btn-sm" onclick={() => void pickCustomBinary()}>
        Browse…
      </button>
      {#if store.preferences.customTokscalePath}
        <button
          type="button"
          class="s-btn s-btn-sm s-btn-ghost"
          onclick={() => {
            customPathDraft = '';
            void store.updatePreferences({
              customTokscalePath: null as unknown as undefined,
            });
          }}
        >
          Reset
        </button>
      {/if}
    </div>
  </div>

  <div class="s-inline-form">
    <button
      type="button"
      class="s-btn s-btn-primary"
      disabled={store.cliInstalling}
      onclick={() => void store.installOrUpgradeCli()}
    >
      {store.cliInstalling ? 'Installing…' : 'Install / Upgrade global tokscale'}
    </button>
    {#if store.cliInstallResult}
      <StatusPill
        tone={store.cliInstallResult.ok ? 'ok' : 'danger'}
        label={store.cliInstallResult.message}
      />
    {/if}
  </div>
</section>

<div class="s-grid-2">
  <section class="s-card" class:is-highlighted={store.highlightedSettingId === 'cli-advanced'}>
    <h2 class="s-card-title">CLI Runtime & Cache</h2>

    <div class="s-row is-first">
      <div class="s-row-copy">
        <span class="s-row-title">Native timeout (seconds)</span>
        <span class="s-row-hint">Max native scan time (5–3600s).</span>
      </div>
      <input
        type="number"
        min="5"
        max="3600"
        class="s-input s-input-num"
        aria-label="Native timeout seconds"
        bind:value={timeoutSecDraft}
        onblur={commitNativeTimeout}
      />
    </div>

    <div class="s-row">
      <div class="s-row-copy">
        <span class="s-row-title">Write cache in <code>--light</code> mode</span>
        <span class="s-row-hint">Atomically refresh TUI cache after light CLI table renders.</span>
      </div>
      <ToggleSwitch
        checked={store.tokscaleSettings?.lightWriteCache ?? false}
        label="Write cache in light mode"
        onchange={(checked) => void store.updateTokscaleSettings({ lightWriteCache: checked })}
      />
    </div>
  </section>

  <section class="s-card" class:is-highlighted={store.highlightedSettingId === 'cli-advanced'}>
    <h2 class="s-card-title">Terminal TUI Preferences</h2>

    <div class="s-row is-first">
      <span class="s-row-title">Color palette</span>
      <select
        class="s-select"
        aria-label="TUI color palette"
        value={store.tokscaleSettings?.colorPalette ?? 'blue'}
        onchange={(e) =>
          void store.updateTokscaleSettings({
            colorPalette: (e.currentTarget as HTMLSelectElement).value,
          })}
      >
        {#each PALETTES as p (p)}
          <option value={p}>{p}</option>
        {/each}
      </select>
    </div>

    <div class="s-row">
      <span class="s-row-title">TUI language</span>
      <select
        class="s-select"
        aria-label="TUI language"
        value={store.tokscaleSettings?.tuiLanguage ?? 'en'}
        onchange={(e) =>
          void store.updateTokscaleSettings({
            tuiLanguage: (e.currentTarget as HTMLSelectElement).value,
          })}
      >
        {#each LANGUAGES as lang (lang.value)}
          <option value={lang.value}>{lang.label}</option>
        {/each}
      </select>
    </div>

    <div class="s-row">
      <div class="s-row-copy">
        <span class="s-row-title">TUI auto-refresh</span>
        <span class="s-row-hint">Interval in seconds (30–3600s).</span>
      </div>
      <div class="s-row-control">
        <input
          type="number"
          min="30"
          max="3600"
          class="s-input s-input-num"
          style="width: 68px;"
          aria-label="TUI auto-refresh seconds"
          disabled={!store.tokscaleSettings?.autoRefreshEnabled}
          bind:value={refreshSecDraft}
          onblur={commitAutoRefreshMs}
        />
        <ToggleSwitch
          checked={store.tokscaleSettings?.autoRefreshEnabled ?? false}
          label="TUI auto-refresh"
          onchange={(checked) =>
            void store.updateTokscaleSettings({ autoRefreshEnabled: checked })}
        />
      </div>
    </div>

    <div class="s-row">
      <div class="s-row-copy">
        <span class="s-row-title">Minutely tab in TUI</span>
        <span class="s-row-hint">Per-minute aggregation (uses extra RAM/CPU).</span>
      </div>
      <ToggleSwitch
        checked={store.tokscaleSettings?.minutelyTabEnabled ?? false}
        label="Minutely tab in TUI"
        onchange={(checked) =>
          void store.updateTokscaleSettings({ minutelyTabEnabled: checked })}
      />
    </div>
  </section>
</div>

<section class="s-card" class:is-highlighted={store.highlightedSettingId === 'config-files'}>
  <h2 class="s-card-title">Configuration Files on Disk</h2>
  <div class="s-path-list">
    {#if store.tokscaleSettings?.path}
      <div class="s-path-item">
        <div style="min-width: 0; flex: 1 1 auto;">
          <div style="font-size: 11.5px; font-weight: 600;">Tokscale CLI settings.json</div>
          <div class="s-code" style="color: var(--muted);" title={store.tokscaleSettings.path}>
            {store.tokscaleSettings.path}
          </div>
        </div>
        <button
          type="button"
          class="s-btn s-btn-sm"
          onclick={() => void store.revealPath(store.tokscaleSettings!.path)}
        >
          Reveal
        </button>
      </div>
    {/if}

    {#if store.customPricing?.path}
      <div class="s-path-item">
        <div style="min-width: 0; flex: 1 1 auto;">
          <div style="font-size: 11.5px; font-weight: 600;">Custom pricing overrides</div>
          <div class="s-code" style="color: var(--muted);" title={store.customPricing.path}>
            {store.customPricing.path}
          </div>
        </div>
        <button
          type="button"
          class="s-btn s-btn-sm"
          onclick={() => void store.revealPath(store.customPricing!.path)}
        >
          Reveal
        </button>
      </div>
    {/if}

    {#if store.tokscaleSettings?.appPreferencesPath}
      <div class="s-path-item">
        <div style="min-width: 0; flex: 1 1 auto;">
          <div style="font-size: 11.5px; font-weight: 600;">Desktop companion preferences.json</div>
          <div
            class="s-code"
            style="color: var(--muted);"
            title={store.tokscaleSettings.appPreferencesPath}
          >
            {store.tokscaleSettings.appPreferencesPath}
          </div>
        </div>
        <button
          type="button"
          class="s-btn s-btn-sm"
          onclick={() => void store.revealPath(store.tokscaleSettings!.appPreferencesPath!)}
        >
          Reveal
        </button>
      </div>
    {/if}
  </div>
</section>
