<script lang="ts">
  import type { UpdateStatusPayload } from '$lib/contracts/ipc';

  interface Props {
    update: UpdateStatusPayload | null;
    onupdate: () => void;
    onrestart: () => void;
    ondetails: () => void;
    ondismiss: () => void;
  }

  let { update, onupdate, onrestart, ondetails, ondismiss }: Props = $props();

  let progressPercent = $derived.by(() => {
    if (!update || update.phase !== 'downloading') return null;
    const downloaded = update.downloadedBytes ?? 0;
    const total = update.totalBytes ?? 0;
    if (total <= 0) return null;
    return Math.min(100, Math.round((downloaded / total) * 100));
  });
</script>

{#if update}
  <div
    class="tray-update-banner"
    data-phase={update.phase}
    role="status"
    aria-live="polite"
  >
    <div class="tray-update-copy">
      {#if update.phase === 'ready'}
        <span class="tray-update-title">
          v{update.availableVersion ?? update.currentVersion} ready
        </span>
        <span class="tray-update-sub">Restart Tokscale UI to finish updating.</span>
      {:else if update.phase === 'downloading'}
        <span class="tray-update-title">
          Downloading v{update.availableVersion ?? ''}…
          {#if progressPercent !== null}{progressPercent}%{/if}
        </span>
        <div class="tray-update-progress" aria-hidden="true">
          <div
            class="tray-update-progress-fill"
            style:width={`${progressPercent ?? 25}%`}
          ></div>
        </div>
      {:else}
        <span class="tray-update-title">
          Update available · v{update.availableVersion}
        </span>
        <span class="tray-update-sub">
          {update.capability === 'selfUpdate'
            ? 'Ready to download and install.'
            : `Install via ${update.installMethod}`}
        </span>
      {/if}
    </div>

    <div class="tray-update-actions">
      {#if update.phase === 'ready'}
        <button type="button" class="tray-update-btn is-primary ui-focusable" onclick={onrestart}>
          Restart
        </button>
      {:else if update.phase === 'available'}
        {#if update.capability === 'selfUpdate'}
          <button type="button" class="tray-update-btn is-primary ui-focusable" onclick={onupdate}>
            Update
          </button>
        {:else}
          <button type="button" class="tray-update-btn ui-focusable" onclick={ondetails}>
            Details
          </button>
        {/if}
        <button
          type="button"
          class="tray-update-dismiss ui-focusable"
          aria-label="Dismiss update notice"
          onclick={ondismiss}
        >
          ×
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .tray-update-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 7px 9px;
    border-radius: var(--radius);
    font-size: 11.5px;
    flex-shrink: 0;
    background: color-mix(in oklch, var(--accent) 12%, transparent);
    border: 1px solid color-mix(in oklch, var(--accent) 32%, transparent);
    color: var(--fg);
  }

  .tray-update-banner[data-phase='ready'] {
    background: color-mix(in oklch, var(--ok) 14%, transparent);
    border-color: color-mix(in oklch, var(--ok) 36%, transparent);
  }

  .tray-update-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1 1 auto;
  }

  .tray-update-title {
    font-weight: 650;
    font-size: 11.5px;
    line-height: 1.25;
  }

  .tray-update-sub {
    font-size: 11px;
    color: var(--muted);
    line-height: 1.25;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tray-update-progress {
    width: 100%;
    height: 4px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--fg) 12%, transparent);
    overflow: hidden;
    margin-top: 2px;
  }

  .tray-update-progress-fill {
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
    transition: width 120ms ease;
  }

  .tray-update-actions {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .tray-update-btn {
    appearance: none;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: var(--radius-sm);
    padding: 3px 8px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }

  .tray-update-btn.is-primary {
    border-color: transparent;
    background: var(--accent);
    color: var(--on-accent);
  }

  .tray-update-btn.is-primary:hover {
    background: var(--accent-hover);
  }

  .tray-update-dismiss {
    appearance: none;
    border: none;
    background: transparent;
    color: var(--muted);
    width: 20px;
    height: 20px;
    border-radius: 4px;
    font-size: 14px;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .tray-update-dismiss:hover {
    background: var(--fg-soft);
    color: var(--fg);
  }
</style>
