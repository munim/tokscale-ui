<script lang="ts">
  import '../styles/tokens.css';
  import { ipc, type TraySnapshot } from '$lib/contracts/ipc';

  let snapshot = $state<TraySnapshot | null>(null);

  $effect(() => {
    void ipc.getTraySnapshot('today').then((data) => {
      snapshot = data;
    });
  });
</script>

<main class="tray-stub" data-view="tray">
  <h1>tokscale</h1>
  {#if snapshot}
    <p>${snapshot.total.cost.toFixed(2)}</p>
  {:else}
    <p>Loading…</p>
  {/if}
</main>
