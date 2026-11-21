<script lang="ts">
  import { formatRelative } from '$lib/format/date';

  interface Props {
    lastSyncedAt?: number;
    syncing: boolean;
    syncMessage?: string;
  }

  let { lastSyncedAt, syncing, syncMessage }: Props = $props();

  let now = $state(new Date());

  $effect(() => {
    const timer = setInterval(() => {
      now = new Date();
    }, 30_000);
    return () => clearInterval(timer);
  });

  let syncText = $derived(
    lastSyncedAt !== undefined
      ? `Last sync ${formatRelative(lastSyncedAt, now)}`
      : 'Not synced yet',
  );
</script>

<footer class="status-bar">
  <div class="status-left">
    <span class="status-sync-text">{syncText}</span>
    {#if syncing}
      <span class="status-syncing" role="status" aria-live="polite">
        <span class="status-syncing-dot" aria-hidden="true"></span>
        <span>{syncMessage && syncMessage.trim() ? syncMessage : 'Syncing…'}</span>
      </span>
    {/if}
  </div>
  <div class="status-right" aria-label="Keyboard shortcuts">
    <span>← → tabs · d t c sort · r refresh · / search</span>
  </div>
</footer>
