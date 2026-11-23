<script lang="ts">
  import { BrandMark } from '$lib/ui';

  interface Props {
    syncing: boolean;
    onsync: () => void;
    onopensettings: () => void;
    firstActionEl?: HTMLButtonElement | null;
  }

  let {
    syncing,
    onsync,
    onopensettings,
    firstActionEl = $bindable(null),
  }: Props = $props();
</script>

<header class="tray-head">
  <div class="tray-brand">
    <BrandMark size={18} />
    <span class="tray-wordmark">tokscale</span>
  </div>

  <div class="tray-head-actions">
    <button
      bind:this={firstActionEl}
      type="button"
      class="tray-icon-btn ui-focusable"
      class:is-syncing={syncing}
      aria-label="Sync now"
      title="Sync now"
      onclick={onsync}
    >
      <svg
        class="tray-sync-icon"
        width="16"
        height="16"
        viewBox="0 0 16 16"
        fill="none"
        aria-hidden="true"
      >
        <path
          d="M13.5 8a5.5 5.5 0 1 1-1.35-3.61"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
        />
        <path
          d="M13.75 2.5v2.75H11"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>

    <button
      type="button"
      class="tray-icon-btn ui-focusable"
      aria-label="Settings"
      title="Settings"
      onclick={onopensettings}
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 16 16"
        fill="none"
        aria-hidden="true"
      >
        <circle cx="8" cy="8" r="2.25" stroke="currentColor" stroke-width="1.4" />
        <path
          d="M8 1.75v1.4M8 12.85v1.4M2.59 4.88l1.21.7M12.2 10.42l1.21.7M2.59 11.12l1.21-.7M12.2 5.58l1.21-.7"
          stroke="currentColor"
          stroke-width="1.4"
          stroke-linecap="round"
        />
      </svg>
    </button>
  </div>
</header>

<style>
  .tray-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    flex-shrink: 0;
  }

  .tray-brand {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }

  .tray-wordmark {
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 700;
    letter-spacing: -0.015em;
    color: var(--fg);
  }

  .tray-head-actions {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .tray-icon-btn {
    width: 32px;
    height: 32px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    cursor: pointer;
    transition:
      background-color 120ms ease,
      color 120ms ease,
      border-color 120ms ease;
  }

  .tray-icon-btn:hover {
    color: var(--fg);
    background: var(--fg-soft);
  }

  .tray-icon-btn:focus-visible {
    outline: none;
    border-color: color-mix(in oklch, var(--accent) 52%, var(--border));
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 18%, transparent);
    color: var(--fg);
  }

  .tray-icon-btn.is-syncing {
    color: var(--accent-strong);
    background: color-mix(in oklch, var(--accent) 10%, var(--surface));
    border-color: color-mix(in oklch, var(--accent) 36%, var(--border));
  }

  .tray-icon-btn.is-syncing .tray-sync-icon {
    animation: tray-spin 0.95s linear infinite;
  }

  @keyframes tray-spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tray-icon-btn.is-syncing .tray-sync-icon {
      animation: none;
    }
  }
</style>
