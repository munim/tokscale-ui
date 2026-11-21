<script lang="ts">
  import type { DashboardSnapshot } from '$lib/contracts/ipc';
  import { formatUsd } from '$lib/format/money';
  import { formatTokens, formatTokensFull } from '$lib/format/tokens';
  import { KpiCard } from '$lib/ui';

  interface Props {
    kpis: DashboardSnapshot['kpis'];
  }

  let { kpis }: Props = $props();
</script>

<section class="kpi-strip" aria-label="Key performance indicators">
  <KpiCard label="Total cost" value={formatUsd(kpis.totalCost)} />
  <KpiCard
    label="Total tokens"
    value={formatTokens(kpis.totalTokens)}
    title={formatTokensFull(kpis.totalTokens)}
  />
  <KpiCard label="Sessions" value={formatTokensFull(kpis.sessions)} />
  <KpiCard
    label="Cost / 1M tokens"
    value={formatUsd(kpis.costPerMillion)}
    hint="blended, all models"
  />
</section>
