<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    disabled?: boolean;
    onchange: (checked: boolean) => void;
  }

  let { checked, label, disabled = false, onchange }: Props = $props();
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  class="s-switch"
  class:is-on={checked}
  onclick={() => {
    if (!disabled) {
      onchange(!checked);
    }
  }}
>
  <span class="s-switch-thumb" aria-hidden="true"></span>
</button>

<style>
  .s-switch {
    position: relative;
    appearance: none;
    width: 36px;
    height: 20px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--fg-soft);
    cursor: pointer;
    flex-shrink: 0;
    transition: background-color 140ms ease, border-color 140ms ease;
  }

  .s-switch.is-on {
    background: var(--accent);
    border-color: transparent;
  }

  .s-switch:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .s-switch:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .s-switch-thumb {
    display: block;
    width: 14px;
    height: 14px;
    border-radius: 999px;
    background: var(--surface);
    box-shadow: 0 1px 2px color-mix(in oklch, var(--shadow-ink) 24%, transparent);
    transform: translateX(0);
    transition: transform 140ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .s-switch.is-on .s-switch-thumb {
    transform: translateX(16px);
  }
</style>
