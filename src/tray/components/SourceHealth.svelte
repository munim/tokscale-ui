<script lang="ts">
  import type { SourceHealthRow } from '$lib/contracts/ipc';
  import { HarnessBadge, StatusPill } from '$lib/ui';

  interface Props {
    sources: SourceHealthRow[];
    onretry: (client: string) => void;
  }

  let { sources, onretry }: Props = $props();

  function pillLabel(row: SourceHealthRow): string {
    if (row.status === 'ok') {
      return 'Detected';
    }
    if (row.status === 'danger') {
      return 'Unreadable';
    }
    return row.sessionsPathExists ? 'Warning' : 'Not found';
  }
</script>

<div class="tray-source-list">
  {#each sources as row (row.client)}
    <div class="tray-source-row">
      <div class="tray-source-info">
        <HarnessBadge id={row.client} size={16} />
        <div class="tray-source-text">
          <span class="tray-source-name">{row.label}</span>
          {#if row.warning}
            <span class="tray-source-warning" title={row.warning}>{row.warning}</span>
          {/if}
        </div>
      </div>

      <div class="tray-source-actions">
        <StatusPill tone={row.status} label={pillLabel(row)} />
        {#if row.status === 'warn' || row.status === 'danger'}
          <button
            type="button"
            class="tray-retry-btn ui-focusable"
            onclick={() => onretry(row.client)}
          >
            Retry
          </button>
        {/if}
      </div>
    </div>
  {/each}
</div>

<style>
  .tray-source-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .tray-source-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 5px 8px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg);
  }

  .tray-source-info {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    flex: 1 1 auto;
  }

  .tray-source-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .tray-source-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tray-source-warning {
    font-size: 10.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tray-source-actions {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .tray-retry-btn {
    appearance: none;
    height: 20px;
    padding: 0 8px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }

  .tray-retry-btn:hover {
    background: var(--fg-soft);
  }
</style>
