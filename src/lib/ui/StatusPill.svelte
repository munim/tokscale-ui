<script lang="ts">
  interface Props {
    tone: 'ok' | 'warn' | 'danger' | 'neutral';
    label: string;
  }

  let { tone, label }: Props = $props();
</script>

<span class="ui-status-pill" data-tone={tone}>
  <span class="ui-status-dot" aria-hidden="true"></span>
  <span class="ui-status-label">{label}</span>
</span>

<style>
  .ui-status-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding: 0 8px;
    border-radius: 999px;
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 600;
    line-height: 1;
    white-space: nowrap;
  }

  .ui-status-dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: currentColor;
    flex-shrink: 0;
  }

  .ui-status-pill[data-tone='ok'] {
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .ui-status-pill[data-tone='warn'] {
    background: color-mix(in oklch, var(--warn) 20%, transparent);
    color: var(--warn-ink);
  }

  .ui-status-pill[data-tone='danger'] {
    background: color-mix(in oklch, var(--danger) 16%, transparent);
    color: var(--danger-ink);
  }

  .ui-status-pill[data-tone='neutral'] {
    background: var(--fg-soft);
    color: var(--muted);
  }
</style>
