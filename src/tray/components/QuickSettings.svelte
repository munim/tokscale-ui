<script lang="ts">
  import type {
    CliInstallResult,
    CliResolutionInfo,
    CompanionPreferences,
    SourceHealthRow,
  } from '$lib/contracts/ipc';
  import { SegmentedControl, StatusPill, type SegmentOption } from '$lib/ui';
  import SourceHealth from './SourceHealth.svelte';

  interface Props {
    sources: SourceHealthRow[];
    preferences: CompanionPreferences;
    cliInfo: CliResolutionInfo | null;
    cliInstalling: boolean;
    cliInstallResult: CliInstallResult | null;
    onretry: (client: string) => void;
    onupdatepreferences: (patch: Partial<CompanionPreferences>) => void;
    oninstallcli: () => void;
  }

  let {
    sources,
    preferences,
    cliInfo,
    cliInstalling,
    cliInstallResult,
    onretry,
    onupdatepreferences,
    oninstallcli,
  }: Props = $props();

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

  let shortcutInput = $derived(preferences.globalShortcut ?? '');
  let quotaInput = $derived(preferences.quotaAlertPercent ?? 85);

  function commitShortcut(rawValue: string): void {
    const trimmed = rawValue.trim();
    const nextValue = trimmed.length > 0 ? trimmed : undefined;
    if (nextValue !== preferences.globalShortcut) {
      onupdatepreferences({ globalShortcut: nextValue });
    }
  }

  function commitQuotaPercent(rawValue: string): void {
    const parsed = Number(rawValue);
    if (!Number.isFinite(parsed)) {
      return;
    }
    const clamped = Math.max(50, Math.min(99, Math.round(parsed)));
    if (clamped !== preferences.quotaAlertPercent) {
      onupdatepreferences({ quotaAlertPercent: clamped });
    }
  }
</script>

<section class="tray-settings" aria-label="Sources and quick settings">
  <div class="tray-settings-block">
    <h3 class="tray-settings-heading">Sources</h3>
    <SourceHealth {sources} {onretry} />
  </div>

  <div class="tray-settings-block">
    <span class="tray-settings-heading">Menubar display</span>
    <SegmentedControl
      options={menubarOptions}
      value={preferences.menubarDisplay}
      size="sm"
      ariaLabel="Menubar display"
      onchange={(val) =>
        onupdatepreferences({ menubarDisplay: val as CompanionPreferences['menubarDisplay'] })}
    />
  </div>

  <div class="tray-settings-block">
    <span class="tray-settings-heading">Theme</span>
    <SegmentedControl
      options={themeOptions}
      value={preferences.theme}
      size="sm"
      ariaLabel="Theme"
      onchange={(val) => onupdatepreferences({ theme: val as CompanionPreferences['theme'] })}
    />
  </div>

  <div class="tray-settings-block">
    <label class="tray-check-row">
      <input
        type="checkbox"
        checked={preferences.launchAtLogin}
        onchange={(event) =>
          onupdatepreferences({ launchAtLogin: (event.currentTarget as HTMLInputElement).checked })}
      />
      <span>Launch at login</span>
    </label>
  </div>

  <div class="tray-settings-block">
    <label class="tray-field">
      <span class="tray-settings-heading">Global shortcut</span>
      <input
        type="text"
        class="tray-input ui-focusable"
        placeholder="Not set"
        value={shortcutInput}
        onchange={(event) => commitShortcut((event.currentTarget as HTMLInputElement).value)}
        onkeydown={(event) => {
          if (event.key === 'Enter') {
            commitShortcut((event.currentTarget as HTMLInputElement).value);
          }
        }}
      />
    </label>
  </div>

  <div class="tray-settings-block">
    <span class="tray-settings-heading">Alerts</span>
    <label class="tray-check-row">
      <input
        type="checkbox"
        checked={preferences.notificationsEnabled}
        onchange={(event) =>
          onupdatepreferences({
            notificationsEnabled: (event.currentTarget as HTMLInputElement).checked,
          })}
      />
      <span>Notify near quota limit</span>
    </label>

    <label class="tray-quota-row">
      <span class="tray-quota-label">Quota alert %</span>
      <input
        type="number"
        class="tray-input tray-input-num ui-mono ui-focusable"
        min={50}
        max={99}
        value={quotaInput}
        onchange={(event) => commitQuotaPercent((event.currentTarget as HTMLInputElement).value)}
      />
    </label>
  </div>

  <div class="tray-settings-block">
    <span class="tray-settings-heading">tokscale</span>
    <div class="tray-cli-meta">
      <span class="tray-cli-version ui-mono">
        {#if cliInfo?.version}
          v{cliInfo.version}
        {:else}
          Not resolved
        {/if}
      </span>
      {#if cliInfo?.binaryPath}
        <span class="tray-cli-path ui-mono" title={cliInfo.binaryPath}>{cliInfo.binaryPath}</span>
      {/if}
    </div>

    <button
      type="button"
      class="tray-cli-btn ui-focusable"
      disabled={cliInstalling}
      onclick={oninstallcli}
    >
      {cliInstalling ? 'Installing…' : 'Install / Upgrade global tokscale'}
    </button>

    {#if cliInstallResult}
      <div class="tray-cli-result">
        <StatusPill
          tone={cliInstallResult.ok ? 'ok' : 'danger'}
          label={cliInstallResult.message}
        />
      </div>
    {/if}
  </div>
</section>

<style>
  .tray-settings {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 2px;
  }

  .tray-settings-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .tray-settings-heading {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .tray-settings :global(.ui-seg) {
    width: 100%;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .tray-settings :global(.ui-seg button) {
    text-align: center;
  }

  .tray-check-row {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--fg);
    cursor: pointer;
  }

  .tray-check-row input[type='checkbox'] {
    accent-color: var(--accent);
    width: 14px;
    height: 14px;
    margin: 0;
  }

  .tray-field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .tray-input {
    height: 30px;
    padding: 0 9px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--fg);
    font-family: var(--font-body);
    font-size: 12px;
    box-sizing: border-box;
  }

  .tray-input::placeholder {
    color: var(--muted);
  }

  .tray-quota-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .tray-quota-label {
    font-size: 12px;
    color: var(--muted);
  }

  .tray-input-num {
    width: 72px;
    text-align: right;
  }

  .tray-cli-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .tray-cli-version {
    font-size: 12px;
    font-weight: 600;
    color: var(--fg);
  }

  .tray-cli-path {
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tray-cli-btn {
    appearance: none;
    height: 32px;
    padding: 0 12px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--fg-soft);
    color: var(--fg);
    font-family: var(--font-body);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 120ms ease;
  }

  .tray-cli-btn:hover:not(:disabled) {
    background: var(--accent-soft);
    border-color: color-mix(in oklch, var(--accent) 45%, var(--border));
  }

  .tray-cli-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .tray-cli-result {
    display: flex;
    align-items: center;
    max-width: 100%;
  }

  .tray-cli-result :global(.ui-status-pill) {
    max-width: 100%;
  }

  .tray-cli-result :global(.ui-status-label) {
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
