<script lang="ts">
  import { formatCountdown } from '$lib/format/countdown';
  import { formatPercent } from '$lib/format/money';
  import StatusPill from './StatusPill.svelte';
  import type { QuotaMetricItem } from './types';

  interface Props {
    provider: string;
    plan?: string;
    email?: string;
    metrics: QuotaMetricItem[];
    now?: Date;
  }

  let { provider, plan, email, metrics, now = new Date() }: Props = $props();

  function resolveUsedPercent(metric: QuotaMetricItem): number {
    if (typeof metric.usedPercent === 'number' && Number.isFinite(metric.usedPercent)) {
      return Math.max(0, Math.min(100, metric.usedPercent));
    }
    if (typeof metric.remainingPercent === 'number' && Number.isFinite(metric.remainingPercent)) {
      return Math.max(0, Math.min(100, 100 - metric.remainingPercent));
    }
    return 0;
  }

  function resolveTone(used: number): 'accent' | 'warn' | 'danger' {
    if (used >= 95) return 'danger';
    if (used >= 80) return 'warn';
    return 'accent';
  }

  let hasNearLimit = $derived(metrics.some((metric) => resolveUsedPercent(metric) >= 80));
</script>

<article class="ui-quota-card" class:is-near-limit={hasNearLimit} data-near-limit={hasNearLimit}>
  <header class="ui-quota-head">
    <div class="ui-quota-identity">
      <div class="ui-quota-title-row">
        <h3 class="ui-quota-provider">{provider}</h3>
        {#if plan}
          <span class="ui-quota-plan">{plan}</span>
        {/if}
      </div>
      {#if email}
        <div class="ui-quota-email">{email}</div>
      {/if}
    </div>
    {#if hasNearLimit}
      <StatusPill tone="warn" label="Near limit" />
    {/if}
  </header>

  <div class="ui-quota-metrics">
    {#each metrics as metric (metric.label)}
      {@const used = resolveUsedPercent(metric)}
      {@const tone = resolveTone(used)}
      {@const remainingText =
        metric.remainingLabel ?? `${formatPercent(Math.max(0, 100 - used))} remaining`}
      <div class="ui-quota-metric">
        <div class="ui-quota-meta">
          <span class="ui-quota-metric-label">{metric.label}</span>
          <span class="ui-quota-metric-value">{remainingText}</span>
        </div>
        <div
          class="ui-quota-track"
          role="progressbar"
          aria-label={metric.label}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(used * 10) / 10}
        >
          <div
            class="ui-quota-fill"
            data-tone={tone}
            style:width={`${used}%`}
          ></div>
        </div>
        <div class="ui-quota-foot">
          <span class="ui-quota-used">{formatPercent(used)} used</span>
          {#if metric.resetsAt}
            <span class="ui-quota-reset">{formatCountdown(metric.resetsAt, now)}</span>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</article>

<style>
  .ui-quota-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .ui-quota-card.is-near-limit {
    border-color: color-mix(in oklch, var(--warn) 55%, var(--border));
  }

  .ui-quota-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .ui-quota-identity {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .ui-quota-title-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }

  .ui-quota-provider {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--fg);
  }

  .ui-quota-plan {
    font-size: 11px;
    font-weight: 500;
    color: var(--muted);
    background: var(--fg-soft);
    padding: 1px 6px;
    border-radius: 999px;
  }

  .ui-quota-email {
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ui-quota-metrics {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .ui-quota-metric {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .ui-quota-meta {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    font-size: 12px;
  }

  .ui-quota-metric-label {
    color: var(--fg);
    font-weight: 500;
  }

  .ui-quota-metric-value {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    color: var(--muted);
    font-size: 11.5px;
  }

  .ui-quota-track {
    width: 100%;
    height: 7px;
    border-radius: 999px;
    background: var(--fg-soft);
    overflow: hidden;
  }

  .ui-quota-fill {
    height: 100%;
    border-radius: 999px;
    transition: width 160ms ease;
  }

  .ui-quota-fill[data-tone='accent'] {
    background: var(--accent);
  }

  .ui-quota-fill[data-tone='warn'] {
    background: var(--warn);
  }

  .ui-quota-fill[data-tone='danger'] {
    background: var(--danger);
  }

  .ui-quota-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    font-size: 11px;
    color: var(--muted);
  }

  .ui-quota-used {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }
</style>
