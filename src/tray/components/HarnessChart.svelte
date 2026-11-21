<script lang="ts">
  import type { ChartBar, HarnessSeriesItem } from '$lib/contracts/ipc';
  import { formatPercent } from '$lib/format';
  import { HarnessBadge, StackedBarChart, type StackedBarItem } from '$lib/ui';

  interface Props {
    bars: ChartBar[];
    harnesses: HarnessSeriesItem[];
  }

  let { bars, harnesses }: Props = $props();

  let chartBars = $derived<StackedBarItem[]>(
    bars.map((bar) => ({
      label: bar.label,
      slot: bar.slot,
      segments: bar.segments.map((seg) => ({
        id: seg.harnessId,
        label: seg.label,
        color: seg.color,
        value: seg.cost,
      })),
    })),
  );

  let legendItems = $derived.by(() => {
    const sorted = [...harnesses].sort((a, b) => b.cost - a.cost);
    if (sorted.length <= 5) {
      return sorted;
    }
    const top5 = sorted.slice(0, 5);
    const rest = sorted.slice(5);
    const otherCost = rest.reduce((sum, item) => sum + item.cost, 0);
    const otherTokens = rest.reduce((sum, item) => sum + item.tokens, 0);
    const otherShare = rest.reduce((sum, item) => sum + item.sharePercent, 0);
    return [
      ...top5,
      {
        id: 'other',
        label: 'Other',
        color: 'var(--c-other)',
        monogram: 'OT',
        cost: otherCost,
        tokens: otherTokens,
        sharePercent: Math.round(otherShare * 10) / 10,
      },
    ];
  });
</script>

<section class="tray-chart-section" aria-label="Usage by harness">
  <StackedBarChart
    bars={chartBars}
    valueFormat="usd"
    height={96}
    compact={true}
    ariaLabel="Usage cost by harness"
  />

  {#if legendItems.length > 0}
    <ul class="tray-legend">
      {#each legendItems as item (item.id)}
        <li class="tray-legend-item">
          <HarnessBadge id={item.id} size={16} />
          <span class="tray-legend-label">{item.label}</span>
          <span class="tray-legend-pct ui-mono">{formatPercent(item.sharePercent, 1)}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .tray-chart-section {
    display: flex;
    flex-direction: column;
    gap: 5px;
    flex-shrink: 0;
  }

  .tray-chart-section :global(.ui-chart) {
    gap: 4px;
  }

  .tray-chart-section :global(.ui-chart-readout) {
    min-height: 16px;
    font-size: 11px;
    line-height: 1.2;
  }

  .tray-chart-section :global(.ui-chart-axis) {
    font-size: 10.5px;
    line-height: 1.2;
  }

  .tray-legend {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 2px 10px;
  }

  .tray-legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: 11.5px;
    line-height: 1.25;
  }

  .tray-legend-label {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg);
  }

  .tray-legend-pct {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--muted);
  }
</style>
