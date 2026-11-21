<script lang="ts">
  import type { TopModelSummary } from '$lib/contracts/ipc';
  import { formatUsd } from '$lib/format';
  import { HarnessBadge } from '$lib/ui';

  interface Props {
    models: TopModelSummary[];
    onselect: (modelName: string) => void;
  }

  let { models, onselect }: Props = $props();

  let topThree = $derived(models.slice(0, 3));
</script>

{#if topThree.length > 0}
  <section class="tray-models" aria-label="Top models">
    <h2 class="tray-section-title">Top models</h2>
    <div class="tray-models-list">
      {#each topThree as item (`${item.client}::${item.model}`)}
        {@const barWidth = Math.max(4, Math.min(100, item.sharePercent))}
        <button
          type="button"
          class="tray-model-row ui-focusable"
          onclick={() => onselect(item.model)}
        >
          <div class="tray-model-main">
            <HarnessBadge id={item.client} size={16} />
            <span class="tray-model-name" title={item.model}>{item.model}</span>
            <span class="tray-model-cost ui-mono">{formatUsd(item.cost)}</span>
          </div>
          <div class="tray-model-bar" aria-hidden="true">
            <div class="tray-model-bar-fill" style:width={`${barWidth}%`}></div>
          </div>
        </button>
      {/each}
    </div>
  </section>
{/if}

<style>
  .tray-models {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex-shrink: 0;
  }

  .tray-section-title {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    line-height: 1.15;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .tray-models-list {
    display: flex;
    flex-direction: column;
    gap: 3px;
    user-select: text;
  }

  .tray-model-row {
    appearance: none;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 4px 8px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    text-align: left;
    cursor: pointer;
    user-select: text;
    transition:
      background-color 120ms ease,
      border-color 120ms ease;
  }

  .tray-model-row:hover {
    background: var(--fg-soft);
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border));
  }

  .tray-model-main {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 100%;
    min-width: 0;
  }

  .tray-model-name {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    line-height: 1.25;
    font-weight: 500;
    color: var(--fg);
  }

  .tray-model-cost {
    flex-shrink: 0;
    font-size: 12px;
    line-height: 1.25;
    font-weight: 600;
    color: var(--fg);
  }

  .tray-model-bar {
    width: 100%;
    height: 2px;
    border-radius: 999px;
    background: var(--fg-soft);
    overflow: hidden;
  }

  .tray-model-bar-fill {
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
  }
</style>
