<script lang="ts">
  interface Props {
    title: string;
    body: string;
    actionLabel?: string;
    onaction?: () => void;
  }

  let { title, body, actionLabel, onaction }: Props = $props();
</script>

<section class="ui-empty-state">
  <h3 class="ui-empty-title">{title}</h3>
  <p class="ui-empty-body">{body}</p>
  {#if actionLabel && onaction}
    <button type="button" class="ui-empty-btn" onclick={onaction}>
      {actionLabel}
    </button>
  {/if}
</section>

<style>
  .ui-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 8px;
    padding: 28px 20px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    border: 1px dashed var(--border);
  }

  .ui-empty-title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--fg);
  }

  .ui-empty-body {
    margin: 0;
    max-width: 36ch;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--muted);
  }

  .ui-empty-btn {
    margin-top: 6px;
    height: 32px;
    padding: 0 14px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--fg-soft);
    color: var(--fg);
    font-family: var(--font-body);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 120ms ease;
  }

  .ui-empty-btn:hover {
    background: var(--accent-soft);
    border-color: color-mix(in oklch, var(--accent) 45%, var(--border));
  }

  .ui-empty-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
