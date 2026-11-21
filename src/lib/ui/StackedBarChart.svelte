<script lang="ts">
  import { formatUsd } from '$lib/format/money';
  import { formatTokens } from '$lib/format/tokens';
  import type { StackedBarItem, StackedBarSegment } from './types';

  interface Props {
    bars: StackedBarItem[];
    height?: number;
    ariaLabel: string;
    valueFormat?: 'usd' | 'tokens';
    compact?: boolean;
  }

  let {
    bars,
    height = 180,
    ariaLabel,
    valueFormat = 'usd',
    compact = true,
  }: Props = $props();

  interface PreparedBar {
    label: string;
    total: number;
    segments: StackedBarSegment[];
  }

  let hoveredIndex = $state<number | null>(null);
  let focusedIndex = $state<number | null>(null);
  let barEls = $state<Array<SVGGElement | undefined>>([]);

  function formatValue(val: number): string {
    return valueFormat === 'tokens' ? formatTokens(val) : formatUsd(val);
  }

  let preparedBars = $derived.by((): PreparedBar[] => {
    if (bars.length === 0) {
      return [];
    }

    let topIds: Set<string> | null = null;
    if (compact) {
      const totals = new Map<string, { label: string; total: number }>();
      for (const bar of bars) {
        for (const seg of bar.segments) {
          if (seg.id === 'other' || seg.value <= 0) {
            continue;
          }
          const prev = totals.get(seg.id);
          if (prev) {
            prev.total += seg.value;
          } else {
            totals.set(seg.id, { label: seg.label, total: seg.value });
          }
        }
      }
      if (totals.size > 5) {
        const ranked = Array.from(totals.entries()).sort((a, b) => {
          if (b[1].total !== a[1].total) {
            return b[1].total - a[1].total;
          }
          return a[1].label.localeCompare(b[1].label);
        });
        topIds = new Set(ranked.slice(0, 5).map(([id]) => id));
      }
    }

    return bars.map((bar) => {
      const merged = new Map<string, StackedBarSegment>();
      let otherSum = 0;

      for (const seg of bar.segments) {
        if (seg.value <= 0) {
          continue;
        }
        if (topIds && !topIds.has(seg.id)) {
          otherSum += seg.value;
          continue;
        }
        const existing = merged.get(seg.id);
        if (existing) {
          existing.value += seg.value;
        } else {
          merged.set(seg.id, { ...seg });
        }
      }

      const segments = Array.from(merged.values());
      if (otherSum > 0) {
        const existingOther = segments.find((s) => s.id === 'other');
        if (existingOther) {
          existingOther.value += otherSum;
        } else {
          segments.push({
            id: 'other',
            label: 'Other',
            color: 'var(--c-other)',
            value: otherSum,
          });
        }
      }

      const total = segments.reduce((sum, s) => sum + s.value, 0);
      return {
        label: bar.label,
        total,
        segments,
      };
    });
  });

  let maxTotal = $derived.by(() => {
    let max = 0;
    for (const bar of preparedBars) {
      if (bar.total > max) {
        max = bar.total;
      }
    }
    return max > 0 ? max : 1;
  });

  let svgWidth = $derived(Math.max(320, preparedBars.length * 20));
  let barStep = $derived(svgWidth / Math.max(1, preparedBars.length));
  let gap = $derived(Math.min(4, Math.max(2, barStep * 0.16)));
  let barWidth = $derived(Math.max(4, barStep - gap));

  let activeIndex = $derived.by(() => {
    const idx = hoveredIndex ?? focusedIndex;
    if (idx === null || idx < 0 || idx >= preparedBars.length) {
      return null;
    }
    return idx;
  });

  let activeBar = $derived(activeIndex !== null ? preparedBars[activeIndex] ?? null : null);

  function focusBar(nextIndex: number): void {
    if (preparedBars.length === 0) return;
    const clamped = Math.max(0, Math.min(preparedBars.length - 1, nextIndex));
    focusedIndex = clamped;
    barEls[clamped]?.focus();
  }

  function handleBarKeydown(event: KeyboardEvent, index: number): void {
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
      event.preventDefault();
      focusBar(index + 1);
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
      event.preventDefault();
      focusBar(index - 1);
    } else if (event.key === 'Home') {
      event.preventDefault();
      focusBar(0);
    } else if (event.key === 'End') {
      event.preventDefault();
      focusBar(preparedBars.length - 1);
    }
  }

  let xLabels = $derived.by(() => {
    const count = preparedBars.length;
    if (count === 0) return null;
    const start = preparedBars[0].label;
    const end = count > 1 ? preparedBars[count - 1].label : '';
    const mid = count >= 3 ? preparedBars[Math.floor(count / 2)].label : '';
    return { start, mid, end };
  });
</script>

<div class="ui-chart">
  <div class="ui-chart-readout" aria-live="polite">
    {#if activeBar}
      <span class="ui-chart-readout-total">
        {activeBar.label} · {formatValue(activeBar.total)}
      </span>
      <span class="ui-chart-readout-segs">
        {#each activeBar.segments as seg (seg.id)}
          <span class="ui-chart-readout-seg">
            <span class="ui-chart-dot" style:background={seg.color} aria-hidden="true"></span>
            <span class="ui-chart-seg-name">{seg.label}</span>
            <span class="ui-chart-seg-val">{formatValue(seg.value)}</span>
          </span>
        {/each}
      </span>
    {:else}
      <span class="ui-chart-readout-idle">Hover a bar</span>
    {/if}
  </div>

  <div class="ui-chart-stage">
    <svg
      class="ui-chart-svg"
      viewBox={`0 0 ${svgWidth} ${height}`}
      style:height={`${height}px`}
      role="img"
      aria-label={ariaLabel}
      onmouseleave={() => (hoveredIndex = null)}
    >
      {#each preparedBars as bar, i (`${bar.label}-${i}`)}
        {@const x = i * barStep + gap / 2}
        {@const isActive = activeIndex === i}
        {@const isDimmed = activeIndex !== null && activeIndex !== i}
        <g
          bind:this={barEls[i]}
          class="ui-chart-bar"
          class:is-active={isActive}
          class:is-dimmed={isDimmed}
          data-bar-index={i}
          role="button"
          tabindex="0"
          aria-label={`${bar.label}: ${formatValue(bar.total)}`}
          onmouseenter={() => (hoveredIndex = i)}
          onfocus={() => (focusedIndex = i)}
          onblur={() => {
            if (focusedIndex === i) {
              focusedIndex = null;
            }
          }}
          onkeydown={(event) => handleBarKeydown(event, i)}
        >
          <rect
            class="ui-chart-hit"
            {x}
            y={0}
            width={barWidth}
            {height}
            fill="transparent"
          />
          {#each bar.segments as seg, segIdx (seg.id)}
            {@const priorSum = bar.segments
              .slice(0, segIdx)
              .reduce((acc, prev) => acc + prev.value, 0)}
            {@const segHeight = Math.max(1, (seg.value / maxTotal) * (height - 4))}
            {@const priorHeight = (priorSum / maxTotal) * (height - 4)}
            {@const y = Math.max(0, height - priorHeight - segHeight)}
            <rect
              class="ui-chart-seg"
              data-segment-id={seg.id}
              data-segment-label={seg.label}
              {x}
              {y}
              width={barWidth}
              height={segHeight}
              rx={segIdx === bar.segments.length - 1 ? 2 : 0}
              fill={seg.color}
            />
          {/each}
        </g>
      {/each}
    </svg>
  </div>

  {#if xLabels}
    <div class="ui-chart-axis" aria-hidden="true">
      <span>{xLabels.start}</span>
      {#if xLabels.mid}
        <span>{xLabels.mid}</span>
      {/if}
      {#if xLabels.end}
        <span>{xLabels.end}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .ui-chart {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }

  .ui-chart-readout {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 6px 10px;
    min-height: 20px;
    font-size: 11.5px;
    color: var(--muted);
  }

  .ui-chart-readout-idle {
    color: var(--muted);
  }

  .ui-chart-readout-total {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--fg);
  }

  .ui-chart-readout-segs {
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .ui-chart-readout-seg {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .ui-chart-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    flex-shrink: 0;
  }

  .ui-chart-seg-name {
    color: var(--muted);
  }

  .ui-chart-seg-val {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    color: var(--fg);
  }

  .ui-chart-stage {
    width: 100%;
    border-bottom: 1px solid var(--border);
  }

  .ui-chart-svg {
    display: block;
    width: 100%;
    overflow: visible;
  }

  .ui-chart-bar {
    cursor: pointer;
    transition: opacity 120ms ease;
  }

  .ui-chart-bar.is-dimmed {
    opacity: 0.55;
  }

  .ui-chart-bar:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .ui-chart-axis {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 11px;
    color: var(--muted);
  }
</style>
