<script lang="ts">
  import type { SyncProgressEvent } from '$lib/contracts/ipc';
  import { formatRelative } from '$lib/format';

  interface Props {
    syncing: boolean;
    syncProgress?: SyncProgressEvent | null;
    syncMessage?: string | null;
    lastSyncedAt?: number;
    hasWarnings: boolean;
    onopen: () => void;
    onquit: () => void;
  }

  let {
    syncing,
    syncProgress = null,
    syncMessage = null,
    lastSyncedAt,
    hasWarnings,
    onopen,
    onquit,
  }: Props = $props();

  let progressPercent = $derived.by(() => {
    if (!syncProgress || syncProgress.totalSources <= 0) {
      return null;
    }
    return Math.max(
      10,
      Math.min(100, Math.round((syncProgress.completedSources / syncProgress.totalSources) * 100)),
    );
  });

  let statusText = $derived.by(() => {
    if (syncing) {
      return syncMessage?.trim() || syncProgress?.message?.trim() || 'Syncing…';
    }
    if (lastSyncedAt) {
      return `Synced ${formatRelative(lastSyncedAt)}`;
    }
    return 'Ready';
  });
</script>

<footer class="tray-footer" class:is-syncing={syncing}>
  {#if syncing}
    <div class="tray-sync-line" aria-hidden="true">
      <div
        class="tray-sync-fill"
        class:is-indeterminate={progressPercent === null}
        style:width={progressPercent !== null ? `${progressPercent}%` : undefined}
      ></div>
    </div>
  {/if}

  <button type="button" class="tray-primary-btn ui-focusable" onclick={onopen}>
    Open dashboard
  </button>

  <div class="tray-footer-status" role="status" aria-live="polite" title={statusText}>
    <span
      class="tray-status-dot"
      class:is-syncing={syncing}
      class:is-warn={!syncing && hasWarnings}
      aria-hidden="true"
    ></span>
    <span class="tray-status-copy">{statusText}</span>
  </div>

  <button type="button" class="tray-quit-btn ui-focusable" onclick={onquit}>
    Quit
  </button>
</footer>

<style>
  .tray-footer {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding-top: 6px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .tray-sync-line {
    position: absolute;
    top: -1px;
    left: -12px;
    right: -12px;
    height: 1.5px;
    background: color-mix(in oklch, var(--accent) 16%, var(--border));
    overflow: hidden;
    pointer-events: none;
  }

  .tray-sync-fill {
    height: 100%;
    border-radius: 999px;
    background: linear-gradient(
      90deg,
      color-mix(in oklch, var(--accent) 65%, transparent) 0%,
      var(--accent) 85%,
      color-mix(in oklch, var(--accent) 82%, white) 100%
    );
    box-shadow: 0 0 8px color-mix(in oklch, var(--accent) 40%, transparent);
    transition: width 240ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .tray-sync-fill.is-indeterminate {
    width: 46%;
    box-shadow: none;
    background: linear-gradient(
      90deg,
      transparent 0%,
      color-mix(in oklch, var(--accent) 36%, transparent) 24%,
      var(--accent) 62%,
      color-mix(in oklch, var(--accent) 84%, white) 82%,
      transparent 100%
    );
    animation: tray-progress-glide 1.35s cubic-bezier(0.4, 0, 0.2, 1) infinite;
  }

  .tray-primary-btn {
    appearance: none;
    flex-shrink: 0;
    height: 32px;
    padding: 0 13px;
    border-radius: var(--radius);
    border: 1px solid transparent;
    background: var(--accent);
    color: var(--on-accent);
    font-family: var(--font-body);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background-color 120ms ease;
  }

  .tray-primary-btn:hover {
    background: var(--accent-hover);
  }

  .tray-primary-btn:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 32%, transparent);
  }

  .tray-footer-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1 1 auto;
    justify-content: center;
    overflow: hidden;
  }

  .tray-status-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--accent);
    flex-shrink: 0;
  }

  .tray-status-dot.is-warn {
    background: var(--warn);
  }

  .tray-status-dot.is-syncing {
    background: var(--accent);
    box-shadow: 0 0 0 2.5px color-mix(in oklch, var(--accent) 22%, transparent);
    animation: tray-pulse 1.35s ease-in-out infinite;
  }

  .tray-status-copy {
    min-width: 0;
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color 140ms ease;
  }

  .tray-footer.is-syncing .tray-status-copy {
    color: color-mix(in oklch, var(--fg) 82%, var(--muted));
  }

  .tray-quit-btn {
    appearance: none;
    flex-shrink: 0;
    height: 32px;
    padding: 0 8px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--muted);
    font-family: var(--font-body);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }

  .tray-quit-btn:hover {
    color: var(--fg);
    background: var(--fg-soft);
  }

  .tray-quit-btn:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 22%, transparent);
    color: var(--fg);
  }

  @keyframes tray-progress-glide {
    0% {
      transform: translateX(-110%);
    }
    100% {
      transform: translateX(225%);
    }
  }

  @keyframes tray-pulse {
    0%,
    100% {
      opacity: 1;
      box-shadow: 0 0 0 3px color-mix(in oklch, var(--accent) 24%, transparent);
    }
    50% {
      opacity: 0.72;
      box-shadow: 0 0 0 1px color-mix(in oklch, var(--accent) 10%, transparent);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tray-status-dot.is-syncing {
      animation: none;
    }

    .tray-sync-fill.is-indeterminate {
      animation: none;
      width: 100%;
    }
  }
</style>
