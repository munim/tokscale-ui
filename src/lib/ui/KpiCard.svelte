<script lang="ts">
  interface Props {
    label: string;
    value: string;
    delta?: string;
    deltaTone?: 'up' | 'down' | 'neutral';
    hint?: string;
    title?: string;
  }

  let { label, value, delta, deltaTone = 'neutral', hint, title }: Props = $props();
</script>

<article class="ui-kpi-card">
  <div class="ui-kpi-top">
    <span class="ui-kpi-label">{label}</span>
    {#if delta}
      <span class="ui-kpi-delta" data-tone={deltaTone}>{delta}</span>
    {/if}
  </div>
  <div class="ui-kpi-value" {title}>{value}</div>
  {#if hint}
    <div class="ui-kpi-hint">{hint}</div>
  {/if}
</article>

<style>
  .ui-kpi-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    min-width: 0;
  }

  .ui-kpi-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .ui-kpi-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ui-kpi-delta {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 11px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 999px;
    white-space: nowrap;
  }

  .ui-kpi-delta[data-tone='up'] {
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .ui-kpi-delta[data-tone='down'] {
    background: color-mix(in oklch, var(--danger) 14%, transparent);
    color: var(--danger-ink);
  }

  .ui-kpi-delta[data-tone='neutral'] {
    background: var(--fg-soft);
    color: var(--muted);
  }

  .ui-kpi-value {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 20px;
    font-weight: 600;
    line-height: 1.2;
    color: var(--fg);
    letter-spacing: -0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ui-kpi-hint {
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
