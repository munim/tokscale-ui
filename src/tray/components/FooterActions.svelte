<script lang="ts">
  import { formatRelative } from '$lib/format';

  interface Props {
    syncing: boolean;
    lastSyncedAt?: number;
    hasWarnings: boolean;
    onopen: () => void;
    onquit: () => void;
  }

  let { syncing, lastSyncedAt, hasWarnings, onopen, onquit }: Props = $props();

  let statusText = $derived.by(() => {
    if (syncing) {
      return 'Syncing…';
    }
    if (lastSyncedAt) {
      return `Synced ${formatRelative(lastSyncedAt)}`;
    }
    return 'Ready';
  });
</script>

<footer class="tray-footer">
  <button type="button" class="tray-primary-btn ui-focusable" onclick={onopen}>
    Open dashboard
  </button>

  <div class="tray-footer-status">
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
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding-top: 6px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .tray-primary-btn {
    appearance: none;
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

  .tray-footer-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1 1 auto;
    justify-content: center;
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
    background: var(--accent-strong);
    animation: tray-pulse 1.1s ease-in-out infinite;
  }

  .tray-status-copy {
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tray-quit-btn {
    appearance: none;
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

  @keyframes tray-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tray-status-dot.is-syncing {
      animation: none;
    }
  }
</style>
