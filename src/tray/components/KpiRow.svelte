<script lang="ts">
  import { formatTokens, formatTokensFull, formatUsd } from '$lib/format';
  import { KpiCard } from '$lib/ui';

  interface Props {
    cost: number;
    tokens: number;
    sessions: number;
  }

  let { cost, tokens, sessions }: Props = $props();

  let costDisplay = $derived(formatUsd(cost));
  let tokensDisplay = $derived(formatTokens(tokens));
  let tokensFull = $derived(formatTokensFull(tokens));
  let sessionsDisplay = $derived(formatTokensFull(Math.max(0, Math.round(sessions))));
</script>

<section class="tray-kpis" aria-label="Usage summary">
  <KpiCard label="Cost" value={costDisplay} />
  <KpiCard label="Tokens" value={tokensDisplay} title={tokensFull} />
  <KpiCard label="Sessions" value={sessionsDisplay} />
</section>

<style>
  .tray-kpis {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    flex-shrink: 0;
  }

  .tray-kpis :global(.ui-kpi-card) {
    padding: 6px 9px;
    gap: 2px;
  }

  .tray-kpis :global(.ui-kpi-label) {
    font-size: 11px;
    line-height: 1.2;
  }

  .tray-kpis :global(.ui-kpi-value) {
    font-size: 16px;
    line-height: 1.15;
  }
</style>
