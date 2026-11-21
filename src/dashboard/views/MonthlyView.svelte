<script lang="ts">
  import type {
    DashboardNavigateEvent,
    DashboardQuery,
    DashboardSnapshot,
  } from '$lib/contracts/ipc';

  interface Props {
    snapshot: DashboardSnapshot;
    query: DashboardQuery;
    onnavigate: (handoff: DashboardNavigateEvent) => void;
  }

  let { snapshot, query, onnavigate }: Props = $props();
  void (() => [snapshot, query, onnavigate]);
</script>

<section data-view="monthly">TODO: Monthly</section>
