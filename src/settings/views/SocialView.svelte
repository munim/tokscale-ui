<script lang="ts">
  import { formatRelative } from '$lib/format';
  import { SegmentedControl, StatusPill, type SegmentOption } from '$lib/ui';
  import ToggleSwitch from '../components/ToggleSwitch.svelte';
  import type { SettingsStore } from '../store.svelte';

  interface Props {
    store: SettingsStore;
  }

  let { store }: Props = $props();

  const intervalOptions: SegmentOption[] = [
    { value: '30m', label: '30m' },
    { value: '1h', label: '1h' },
    { value: '6h', label: '6h' },
    { value: '24h', label: '24h' },
  ];

  const rangeOptions: SegmentOption[] = [
    { value: 'all', label: 'All' },
    { value: 'today', label: 'Today' },
    { value: 'week', label: '7d' },
    { value: 'month', label: '30d' },
  ];

  let apiTokenDraft = $state('');
  let busy = $state<string | null>(null);
  let submitRange = $state('all');
  let submitClients = $state('');

  let autoInterval = $state('24h');
  let autoRange = $state('all');
  let autoClients = $state('');

  $effect(() => {
    const auto = store.socialStatus?.autosubmit;
    if (!auto) return;
    if (auto.intervalMinutes <= 30) autoInterval = '30m';
    else if (auto.intervalMinutes <= 60) autoInterval = '1h';
    else if (auto.intervalMinutes <= 360) autoInterval = '6h';
    else autoInterval = '24h';

    if (auto.today) autoRange = 'today';
    else if (auto.week) autoRange = 'week';
    else if (auto.month) autoRange = 'month';
    else autoRange = 'all';

    autoClients = auto.clients.join(', ');
  });

  function parseClientList(raw: string): string[] {
    return raw
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
  }

  async function runSocial(
    action: 'login_browser' | 'login_token' | 'logout' | 'submit_dry_run' | 'submit' | 'autosubmit_run',
  ): Promise<void> {
    busy = action;
    try {
      await store.runSocialAction({
        action,
        token: action === 'login_token' ? apiTokenDraft : undefined,
        clients:
          action === 'submit' || action === 'submit_dry_run'
            ? parseClientList(submitClients)
            : undefined,
        range:
          (action === 'submit' || action === 'submit_dry_run') && submitRange !== 'all'
            ? submitRange
            : undefined,
      });
      apiTokenDraft = '';
    } finally {
      busy = null;
    }
  }

  async function saveAutosubmit(enabled: boolean): Promise<void> {
    busy = 'autosubmit';
    try {
      await store.configureAutosubmit({
        enabled,
        interval: enabled ? autoInterval : undefined,
        clients: enabled ? parseClientList(autoClients) : [],
        range: enabled && autoRange !== 'all' ? autoRange : undefined,
      });
    } finally {
      busy = null;
    }
  }
</script>

<section class="s-card" class:is-highlighted={store.highlightedSettingId === 'social-auth'}>
  <div class="s-card-head">
    <div>
      <h2 class="s-card-title">Tokscale Social Account</h2>
      <p class="s-card-desc">Authenticate to submit usage stats to the Tokscale leaderboard.</p>
    </div>
    <StatusPill
      tone={store.socialStatus?.loggedIn ? 'ok' : 'neutral'}
      label={store.socialStatus?.loggedIn
        ? `@${store.socialStatus.username ?? 'connected'}`
        : 'Not signed in'}
    />
  </div>

  {#if store.socialStatus?.loggedIn}
    <div class="s-inline-form">
      <button
        type="button"
        class="s-btn s-btn-danger"
        disabled={busy !== null}
        onclick={() => void runSocial('logout')}
      >
        {busy === 'logout' ? 'Logging out…' : 'Logout'}
      </button>
    </div>
  {:else}
    <div class="s-inline-form">
      <button
        type="button"
        class="s-btn s-btn-primary"
        disabled={busy !== null}
        onclick={() => void runSocial('login_browser')}
      >
        {busy === 'login_browser' ? 'Waiting for browser…' : 'Login with GitHub (Browser)'}
      </button>
    </div>

    <div class="s-inline-form">
      <input
        type="password"
        class="s-input s-input-mono s-inline-grow"
        placeholder="Or paste personal API token (tt_...)"
        aria-label="Tokscale API token"
        bind:value={apiTokenDraft}
      />
      <button
        type="button"
        class="s-btn"
        disabled={busy !== null || !apiTokenDraft.trim()}
        onclick={() => void runSocial('login_token')}
      >
        Save Token
      </button>
    </div>
  {/if}
</section>

<div class="s-grid-2">
  <section class="s-card" class:is-highlighted={store.highlightedSettingId === 'manual-submit'}>
    <h2 class="s-card-title">Manual Submit</h2>
    <p class="s-card-desc">Run a dry-run preview first to verify priced tokens before submitting.</p>

    <div class="s-row is-first">
      <span class="s-row-title">Range</span>
      <SegmentedControl
        options={rangeOptions}
        value={submitRange}
        size="sm"
        ariaLabel="Submit range"
        onchange={(v) => (submitRange = v)}
      />
    </div>

    <div style="display: flex; flex-direction: column; gap: 4px;">
      <label class="s-row-hint" for="submit-clients-input">Optional client filter (comma-separated)</label>
      <input
        id="submit-clients-input"
        type="text"
        class="s-input s-input-mono"
        placeholder="All clients (e.g. claude, codex)"
        bind:value={submitClients}
      />
    </div>

    <div class="s-inline-form">
      <button
        type="button"
        class="s-btn"
        disabled={busy !== null}
        onclick={() => void runSocial('submit_dry_run')}
      >
        {busy === 'submit_dry_run' ? 'Previewing…' : 'Dry-Run Preview'}
      </button>
      <button
        type="button"
        class="s-btn s-btn-primary"
        disabled={busy !== null || !store.socialStatus?.loggedIn}
        onclick={() => void runSocial('submit')}
      >
        {busy === 'submit' ? 'Submitting…' : 'Submit Now'}
      </button>
    </div>
  </section>

  <section class="s-card" class:is-highlighted={store.highlightedSettingId === 'autosubmit'}>
    <div class="s-card-head">
      <div>
        <h2 class="s-card-title">Scheduled Autosubmit</h2>
        <p class="s-card-desc">
          OS scheduler ({store.socialStatus?.autosubmit.scheduler ?? 'launchd / systemd / cron'}).
        </p>
      </div>
      <ToggleSwitch
        checked={store.socialStatus?.autosubmit.enabled ?? false}
        label="Enable scheduled autosubmit"
        disabled={busy !== null}
        onchange={(enabled) => void saveAutosubmit(enabled)}
      />
    </div>

    <div class="s-row is-first">
      <span class="s-row-title">Interval</span>
      <SegmentedControl
        options={intervalOptions}
        value={autoInterval}
        size="sm"
        ariaLabel="Autosubmit interval"
        onchange={(v) => (autoInterval = v)}
      />
    </div>

    <div class="s-row">
      <span class="s-row-title">Range</span>
      <SegmentedControl
        options={rangeOptions}
        value={autoRange}
        size="sm"
        ariaLabel="Autosubmit range"
        onchange={(v) => (autoRange = v)}
      />
    </div>

    <div style="display: flex; flex-direction: column; gap: 4px;">
      <label class="s-row-hint" for="auto-clients-input">Client filter (optional)</label>
      <input
        id="auto-clients-input"
        type="text"
        class="s-input s-input-mono"
        placeholder="All clients"
        bind:value={autoClients}
      />
    </div>

    <div class="s-inline-form">
      <button
        type="button"
        class="s-btn s-btn-sm s-btn-primary"
        disabled={busy !== null}
        onclick={() => void saveAutosubmit(true)}
      >
        {busy === 'autosubmit' ? 'Applying…' : 'Apply Schedule'}
      </button>
      <button
        type="button"
        class="s-btn s-btn-sm"
        disabled={busy !== null}
        onclick={() => void runSocial('autosubmit_run')}
      >
        Run Once Now
      </button>
    </div>

    {#if store.socialStatus?.autosubmit.lastRunAtMs}
      <span class="s-row-hint">
        Last run: {formatRelative(store.socialStatus.autosubmit.lastRunAtMs)}
      </span>
    {/if}
    {#if store.socialStatus?.autosubmit.lastError}
      <StatusPill tone="danger" label={store.socialStatus.autosubmit.lastError} />
    {/if}
  </section>
</div>

{#if store.lastActionOutput}
  <section class="s-card">
    <span class="s-section-label">Latest Output</span>
    <pre class="s-output-pre">{store.lastActionOutput}</pre>
  </section>
{/if}
