<script lang="ts">
  import type { UpdateStatusPayload } from '$lib/contracts/ipc';
  import { formatRelative } from '$lib/format/date';

  interface Props {
    lastSyncedAt?: number;
    syncing: boolean;
    syncMessage?: string;
    updateStatus?: UpdateStatusPayload | null;
    skippedUpdateVersion?: string;
    onOpenUpdates?: () => void;
  }

  let {
    lastSyncedAt,
    syncing,
    syncMessage,
    updateStatus = null,
    skippedUpdateVersion,
    onOpenUpdates,
  }: Props = $props();

  let now = $state(new Date());

  $effect(() => {
    const timer = setInterval(() => {
      now = new Date();
    }, 30_000);
    return () => clearInterval(timer);
  });

  let syncText = $derived(
    lastSyncedAt !== undefined && lastSyncedAt > 0
      ? `Last sync ${formatRelative(lastSyncedAt, now)}`
      : 'Not synced yet',
  );

  let updateLabel = $derived.by(() => {
    if (!updateStatus) return null;
    if (updateStatus.phase === 'ready') {
      return `Restart to apply v${updateStatus.availableVersion ?? ''}`;
    }
    if (updateStatus.phase === 'downloading') {
      return `Downloading v${updateStatus.availableVersion ?? ''}…`;
    }
    if (
      updateStatus.phase === 'available' &&
      updateStatus.availableVersion &&
      updateStatus.availableVersion !== skippedUpdateVersion
    ) {
      return `Update v${updateStatus.availableVersion} available`;
    }
    return null;
  });
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
    {#if updateLabel}
      <button
        type="button"
        class="status-update-pill"
        class:is-ready={updateStatus?.phase === 'ready'}
        onclick={() => onOpenUpdates?.()}
      >
        {updateLabel}
      </button>
    {/if}
  </div>
  <div class="status-right" aria-label="Keyboard shortcuts">
    <span>← → tabs · d t c sort · r refresh · / search</span>
  </div>
</footer>
