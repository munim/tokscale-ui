<script lang="ts">
  import type {
    DashboardNavigateEvent,
    DashboardQuery,
    DashboardSnapshot,
  } from '$lib/contracts/ipc';
  import { EmptyState, QuotaMeterCard } from '$lib/ui';

  interface Props {
    snapshot: DashboardSnapshot;
    query: DashboardQuery;
    onnavigate: (handoff: DashboardNavigateEvent) => void;
  }

  let { snapshot, query, onnavigate }: Props = $props();
  void (() => [query, onnavigate]);

  let now = $state(new Date());

  $effect(() => {
    const timer = setInterval(() => {
      now = new Date();
    }, 60_000);
    return () => clearInterval(timer);
  });
</script>

<section class="usage-view" data-view="usage">
  {#if snapshot.usage.length === 0}
    <EmptyState
      title="No subscription quotas found"
      body="Quotas appear once a subscription provider is signed in to tokscale."
    />
  {:else}
    <div class="usage-grid">
      {#each snapshot.usage as item (`${item.provider}:${item.email ?? ''}:${item.plan ?? ''}`)}
        <QuotaMeterCard
          provider={item.provider}
          plan={item.plan}
          email={item.email}
          metrics={item.metrics}
          {now}
        />
      {/each}
    </div>
  {/if}
</section>
