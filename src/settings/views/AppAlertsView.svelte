<script lang="ts">
  import type { CompanionPreferences } from '$lib/contracts/ipc';
  import { BrandMark, SegmentedControl, StatusPill, type SegmentOption } from '$lib/ui';
  import ShortcutRecorder from '../components/ShortcutRecorder.svelte';
  import ToggleSwitch from '../components/ToggleSwitch.svelte';
  import type { SettingsStore } from '../store.svelte';

  interface Props {
    store: SettingsStore;
  }

  let { store }: Props = $props();

  const menubarOptions: SegmentOption[] = [
    { value: 'cost', label: 'Cost' },
    { value: 'tokens', label: 'Tokens' },
    { value: 'icon', label: 'Icon' },
  ];

  const themeOptions: SegmentOption[] = [
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
    { value: 'system', label: 'System' },
  ];

  let quotaValue = $derived(store.preferences.quotaAlertPercent ?? 85);
  let budgetEnabled = $derived(store.preferences.dailyBudgetUsd !== undefined && store.preferences.dailyBudgetUsd !== null);
  let budgetDraft = $state('25.00');

  $effect(() => {
    const val = store.preferences.dailyBudgetUsd;
    if (typeof val === 'number' && Number.isFinite(val)) {
      budgetDraft = val.toFixed(2);
    }
  });

  function commitQuota(raw: string | number): void {
    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) return;
    const clamped = Math.max(50, Math.min(99, Math.round(parsed)));
    if (clamped !== store.preferences.quotaAlertPercent) {
      void store.updatePreferences({ quotaAlertPercent: clamped });
    }
  }

  function toggleDailyBudget(enabled: boolean): void {
    if (!enabled) {
      void store.updatePreferences({ dailyBudgetUsd: null as unknown as undefined });
      return;
    }
    const parsed = Number(budgetDraft);
    const next = Number.isFinite(parsed) && parsed > 0 ? parsed : 25;
    budgetDraft = next.toFixed(2);
    void store.updatePreferences({ dailyBudgetUsd: next });
  }

  function commitBudgetAmount(raw: string): void {
    const parsed = Number(raw);
    if (!Number.isFinite(parsed) || parsed < 0) return;
    const rounded = Math.round(parsed * 100) / 100;
    budgetDraft = rounded.toFixed(2);
    if (rounded !== store.preferences.dailyBudgetUsd) {
      void store.updatePreferences({ dailyBudgetUsd: rounded });
    }
  }

  let menubarPreviewText = $derived.by(() => {
    if (store.preferences.menubarDisplay === 'tokens') return '14.2M';
    if (store.preferences.menubarDisplay === 'icon') return '';
    return '$14.82';
  });

  function formatInstallMethod(method: string | undefined): string {
    switch (method) {
      case 'macos-bundle':
        return 'macOS App Bundle';
      case 'dmg-mounted':
        return 'Mounted DMG (read-only)';
      case 'translocated':
        return 'App Translocation (read-only)';
      case 'macos-readonly':
        return 'Read-only macOS Bundle';
      case 'windows-nsis':
        return 'Windows Per-User Installer';
      case 'scoop':
        return 'Scoop Package';
      case 'windows-readonly':
        return 'Read-only Windows Install';
      case 'appimage':
        return 'Linux AppImage';
      case 'appimage-readonly':
        return 'Read-only AppImage';
      case 'linux-package':
        return 'Linux System Package';
      case 'dev-build':
        return 'Development Build';
      default:
        return method ?? 'Desktop App';
    }
  }

  let updateStatus = $derived(store.updateStatus);

  let updateBadge = $derived.by((): { tone: 'ok' | 'warn' | 'danger' | 'neutral'; label: string } => {
    const u = updateStatus;
    if (!u) return { tone: 'neutral', label: 'Ready' };
    switch (u.phase) {
      case 'checking':
        return { tone: 'neutral', label: 'Checking…' };
      case 'upToDate':
        return { tone: 'ok', label: 'Up to date' };
      case 'available':
        return { tone: 'warn', label: `v${u.availableVersion ?? ''} available` };
      case 'downloading':
        return { tone: 'neutral', label: 'Downloading…' };
      case 'ready':
        return { tone: 'ok', label: `v${u.availableVersion ?? ''} ready` };
      case 'error':
        return { tone: 'danger', label: 'Check failed' };
      default:
        return { tone: 'neutral', label: `v${u.currentVersion}` };
    }
  });

  let downloadPercent = $derived.by(() => {
    const u = updateStatus;
    if (!u || u.phase !== 'downloading') return null;
    const total = u.totalBytes ?? 0;
    const downloaded = u.downloadedBytes ?? 0;
    if (total <= 0) return null;
    return Math.min(100, Math.round((downloaded / total) * 100));
  });

  async function copyFallbackHint(text: string): Promise<void> {
    try {
      await navigator.clipboard?.writeText(text);
      store.showToast('ok', 'Copied command');
    } catch {
      store.showToast('info', 'Select the command to copy');
    }
  }
</script>

<section class="menubar-hero" class:is-highlighted={store.highlightedSettingId === 'menubar' || store.highlightedSettingId === 'theme'}>
  <div class="menubar-preview-box">
    <span class="s-section-label">Menubar Live Preview</span>
    <div class="menubar-pill-stage" aria-label="Menubar preview">
      <BrandMark size={16} />
      {#if menubarPreviewText}
        <span class="menubar-pill-value">{menubarPreviewText}</span>
      {/if}
    </div>
    <span class="s-row-hint">Updates immediately in your macOS / Linux menubar.</span>
  </div>

  <div class="menubar-controls-stack">
    <div class="menubar-control-row">
      <span class="menubar-control-label">Readout</span>
      <SegmentedControl
        options={menubarOptions}
        value={store.preferences.menubarDisplay}
        size="sm"
        ariaLabel="Menubar display"
        onchange={(val) =>
          void store.updatePreferences({
            menubarDisplay: val as CompanionPreferences['menubarDisplay'],
          })}
      />
    </div>

    <div class="menubar-control-row">
      <span class="menubar-control-label">Theme</span>
      <SegmentedControl
        options={themeOptions}
        value={store.preferences.theme}
        size="sm"
        ariaLabel="Theme"
        onchange={(val) =>
          void store.updatePreferences({ theme: val as CompanionPreferences['theme'] })}
      />
    </div>
  </div>
</section>

<div class="s-grid-2">
  <section
    class="s-card"
    class:is-highlighted={store.highlightedSettingId === 'launch' || store.highlightedSettingId === 'shortcut' || store.highlightedSettingId === 'titles'}
  >
    <div class="s-card-head">
      <h2 class="s-card-title">Behavior</h2>
    </div>

    <div class="s-row is-first">
      <div class="s-row-copy">
        <span class="s-row-title">Launch at login</span>
        <span class="s-row-hint">Start the menubar companion automatically when you sign in.</span>
      </div>
      <ToggleSwitch
        checked={store.preferences.launchAtLogin}
        label="Launch at login"
        onchange={(checked) => void store.updatePreferences({ launchAtLogin: checked })}
      />
    </div>

    <div class="s-row">
      <div class="s-row-copy">
        <span class="s-row-title">Global shortcut</span>
        <span class="s-row-hint">Toggle the tray popover from any app.</span>
      </div>
      <ShortcutRecorder
        value={store.preferences.globalShortcut}
        onchange={(next) =>
          void store.updatePreferences({
            globalShortcut: (next ?? null) as unknown as undefined,
          })}
      />
    </div>

    <div class="s-row">
      <div class="s-row-copy">
        <span class="s-row-title">Session title enrichment</span>
        <span class="s-row-hint">Uses Tokscale's free local summarizer for today / 7d sessions.</span>
      </div>
      <ToggleSwitch
        checked={store.preferences.sessionTitleEnrichment}
        label="Session title enrichment"
        onchange={(checked) => void store.updatePreferences({ sessionTitleEnrichment: checked })}
      />
    </div>
  </section>

  <section
    class="s-card"
    class:is-highlighted={store.highlightedSettingId === 'notifications'}
  >
    <div class="s-card-head">
      <h2 class="s-card-title">Alerts & Budgets</h2>
    </div>

    <div class="s-row is-first">
      <div class="s-row-copy">
        <span class="s-row-title">Desktop notifications</span>
        <span class="s-row-hint">Alert once per metric per day when thresholds are crossed.</span>
      </div>
      <ToggleSwitch
        checked={store.preferences.notificationsEnabled}
        label="Desktop notifications"
        onchange={(checked) => void store.updatePreferences({ notificationsEnabled: checked })}
      />
    </div>

    <div class="s-row">
      <div class="s-row-copy" style="flex: 1 1 auto;">
        <div style="display: flex; justify-content: space-between; align-items: center; gap: 8px;">
          <span class="s-row-title">Quota alert threshold</span>
          <span class="ui-mono" style="font-size: 12px; font-weight: 600;">{quotaValue}%</span>
        </div>
        <input
          type="range"
          class="s-slider"
          aria-label="Quota alert threshold"
          min={50}
          max={99}
          value={quotaValue}
          disabled={!store.preferences.notificationsEnabled}
          onchange={(e) => commitQuota((e.currentTarget as HTMLInputElement).value)}
        />
      </div>
    </div>

    <div class="s-row">
      <div class="s-row-copy">
        <span class="s-row-title">Daily spend budget</span>
        <span class="s-row-hint">Notify when today's total spend reaches this USD cap.</span>
      </div>
      <ToggleSwitch
        checked={budgetEnabled}
        label="Enable daily spend budget"
        onchange={toggleDailyBudget}
      />
    </div>

    {#if budgetEnabled}
      <div class="s-inline-form" style="padding-top: 2px;">
        <label class="s-row-hint" for="daily-budget-input">Daily cap (USD)</label>
        <input
          id="daily-budget-input"
          type="number"
          class="s-input s-input-num"
          min={0}
          step="0.5"
          aria-label="Daily spend budget USD"
          value={budgetDraft}
          onchange={(e) => commitBudgetAmount((e.currentTarget as HTMLInputElement).value)}
        />
        <span class="s-row-hint">/ day</span>
      </div>
    {/if}
  </section>
</div>

<section class="s-card" class:is-highlighted={store.highlightedSettingId === 'updates'}>
  <div class="s-card-head">
    <div>
      <h2 class="s-card-title">App & Updates</h2>
      <p class="s-card-desc">
        Tokscale UI v{updateStatus?.currentVersion ?? '0.1.0'} · {formatInstallMethod(updateStatus?.installMethod)}
      </p>
    </div>
    <StatusPill tone={updateBadge.tone} label={updateBadge.label} />
  </div>

  <div class="s-row is-first">
    <div class="s-row-copy">
      <span class="s-row-title">Check for updates automatically</span>
      <span class="s-row-hint">Silently checks signed GitHub releases on launch and every 6 hours.</span>
    </div>
    <ToggleSwitch
      checked={store.preferences.autoCheckUpdates ?? true}
      label="Check for updates automatically"
      onchange={(checked) => void store.updatePreferences({ autoCheckUpdates: checked })}
    />
  </div>

  {#if updateStatus?.phase === 'downloading'}
    <div class="s-update-progress-wrap" role="status" aria-live="polite">
      <div class="s-update-progress-meta">
        <span>Downloading v{updateStatus.availableVersion ?? ''}…</span>
        <span class="ui-mono">{downloadPercent !== null ? `${downloadPercent}%` : '…'}</span>
      </div>
      <div class="s-update-progress-bar">
        <div
          class="s-update-progress-fill"
          style:width={`${downloadPercent ?? 25}%`}
        ></div>
      </div>
    </div>
  {:else if updateStatus?.phase === 'ready'}
    <div class="s-inline-form">
      <button
        type="button"
        class="s-btn s-btn-primary"
        onclick={() => void store.restartToApplyUpdate()}
      >
        Restart to Apply v{updateStatus.availableVersion ?? ''}
      </button>
      <span class="s-row-hint">Update is staged and verified. Restart to launch the new version.</span>
    </div>
  {:else if updateStatus?.phase === 'available' && updateStatus.availableVersion}
    <div class="s-inline-form">
      {#if updateStatus.capability === 'selfUpdate'}
        <button
          type="button"
          class="s-btn s-btn-primary"
          onclick={() => void store.downloadAndInstallUpdate()}
        >
          Download & Install v{updateStatus.availableVersion}
        </button>
      {:else if updateStatus.fallbackHint}
        <div class="s-path-item" style="flex: 1 1 280px;">
          <span class="s-code" title={updateStatus.fallbackHint}>{updateStatus.fallbackHint}</span>
          <button
            type="button"
            class="s-btn s-btn-sm s-btn-ghost"
            onclick={() => void copyFallbackHint(updateStatus!.fallbackHint!)}
          >
            Copy
          </button>
        </div>
      {/if}
      <button
        type="button"
        class="s-btn s-btn-ghost"
        onclick={() => void store.skipUpdateVersion(updateStatus!.availableVersion!)}
      >
        Skip v{updateStatus.availableVersion}
      </button>
    </div>

    {#if updateStatus.releaseNotes && updateStatus.releaseNotes.trim().length > 0}
      <pre class="s-output-pre">{updateStatus.releaseNotes}</pre>
    {/if}
  {:else}
    <div class="s-inline-form">
      <button
        type="button"
        class="s-btn"
        disabled={updateStatus?.phase === 'checking'}
        onclick={() => void store.checkForUpdates()}
      >
        {updateStatus?.phase === 'checking' ? 'Checking…' : 'Check for Updates'}
      </button>
      {#if updateStatus?.phase === 'error' && updateStatus.error}
        <span class="s-row-hint" style="color: var(--danger-ink);">{updateStatus.error}</span>
      {:else if updateStatus?.capability === 'notifyOnly' && updateStatus.fallbackHint}
        <span class="s-row-hint">Manual updates via {formatInstallMethod(updateStatus.installMethod)}</span>
      {/if}
    </div>
  {/if}
</section>
