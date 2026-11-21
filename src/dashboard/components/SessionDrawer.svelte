<script lang="ts">
  import type { SessionRow } from '$lib/contracts/ipc';

  interface Props {
    session: SessionRow | null;
    onclose: () => void;
  }

  let { session, onclose }: Props = $props();
</script>

{#if session}
  <div role="dialog" aria-modal="true" aria-labelledby="drawer-title">
    <h2 id="drawer-title">{session.displayLabel}</h2>
    <button type="button" onclick={onclose}>Close</button>
  </div>
{/if}
