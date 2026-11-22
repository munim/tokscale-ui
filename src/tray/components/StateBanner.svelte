<script lang="ts">
  import type { SourceWarning } from '$lib/contracts/ipc';

  interface Props {
    partial: boolean;
    warnings: SourceWarning[];
    error?: string | null;
    onretry: () => void;
  }

  let { partial, warnings, error = null, onretry }: Props = $props();
</script>

{#if error}
  <div class="tray-banner" data-tone="danger" role="alert">
    <div class="tray-banner-copy">
      <span class="tray-banner-title">Sync error</span>
      <span class="tray-banner-msg">{error}</span>
    </div>
    <button type="button" class="tray-banner-retry ui-focusable" onclick={onretry}>
      Retry
    </button>
  </div>
{:else if partial && warnings.length > 0}
  <div class="tray-banner" data-tone="warn" role="status">
    <div class="tray-banner-copy">
      {#each warnings as warning (`${warning.source}:${warning.message}`)}
        <div class="tray-banner-row">
          <span class="tray-banner-source">{warning.source}</span>
          <span class="tray-banner-msg">{warning.message}</span>
        </div>
      {/each}
    </div>
    <button type="button" class="tray-banner-retry ui-focusable" onclick={onretry}>
      Retry
    </button>
  </div>
{/if}

<style>
  .tray-banner {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
    padding: 7px 9px;
    border-radius: var(--radius);
    font-size: 11.5px;
    flex-shrink: 0;
  }

  .tray-banner[data-tone='warn'] {
    background: color-mix(in oklch, var(--warn) 16%, transparent);
    border: 1px solid color-mix(in oklch, var(--warn) 38%, transparent);
    color: var(--warn-ink);
  }

  .tray-banner[data-tone='danger'] {
    background: color-mix(in oklch, var(--danger) 14%, transparent);
    border: 1px solid color-mix(in oklch, var(--danger) 36%, transparent);
    color: var(--danger-ink);
  }

  .tray-banner-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1 1 auto;
  }

  .tray-banner-row {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
    line-height: 1.35;
  }

  .tray-banner-source,
  .tray-banner-title {
    font-weight: 600;
  }

  .tray-banner-msg {
    word-break: break-word;
  }

  .tray-banner-retry {
    appearance: none;
    flex-shrink: 0;
    background: transparent;
    border: none;
    padding: 0;
    font-family: var(--font-body);
    font-size: 11.5px;
    font-weight: 600;
    color: currentColor;
    text-decoration: underline;
    cursor: pointer;
  }
</style>
