<script lang="ts">
  import type { SegmentOption } from './types';

  interface Props {
    options: SegmentOption[];
    value: string;
    onchange: (value: string) => void;
    ariaLabel: string;
    size?: 'sm' | 'md';
  }

  let { options, value, onchange, ariaLabel, size = 'md' }: Props = $props();

  let buttonEls = $state<Array<HTMLButtonElement | undefined>>([]);

  let activeIndex = $derived.by(() => {
    const idx = options.findIndex((o) => o.value === value && !o.disabled);
    if (idx >= 0) {
      return idx;
    }
    return options.findIndex((o) => !o.disabled);
  });

  function findNextEnabledIndex(startIndex: number, step: 1 | -1): number {
    if (options.length === 0) {
      return -1;
    }
    for (let offset = 1; offset <= options.length; offset++) {
      const idx = (startIndex + step * offset + options.length) % options.length;
      if (!options[idx]?.disabled) {
        return idx;
      }
    }
    return -1;
  }

  function selectByIndex(idx: number): void {
    const target = options[idx];
    if (!target || target.disabled) {
      return;
    }
    buttonEls[idx]?.focus();
    onchange(target.value);
  }

  function handleKeydown(event: KeyboardEvent, index: number): void {
    let nextIndex = -1;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
      nextIndex = findNextEnabledIndex(index, 1);
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
      nextIndex = findNextEnabledIndex(index, -1);
    } else if (event.key === 'Home') {
      nextIndex = options.findIndex((o) => !o.disabled);
    } else if (event.key === 'End') {
      for (let i = options.length - 1; i >= 0; i--) {
        if (!options[i]?.disabled) {
          nextIndex = i;
          break;
        }
      }
    } else {
      return;
    }

    if (nextIndex >= 0) {
      event.preventDefault();
      selectByIndex(nextIndex);
    }
  }
</script>

<div class="ui-seg" class:is-sm={size === 'sm'} role="radiogroup" aria-label={ariaLabel}>
  {#each options as option, i (option.value)}
    {@const isChecked = option.value === value}
    <button
      bind:this={buttonEls[i]}
      type="button"
      role="radio"
      aria-checked={isChecked}
      disabled={option.disabled}
      tabindex={i === activeIndex ? 0 : -1}
      class:is-active={isChecked}
      onclick={() => {
        if (!option.disabled && option.value !== value) {
          onchange(option.value);
        }
      }}
      onkeydown={(event) => handleKeydown(event, i)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .ui-seg {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius);
    background: var(--fg-soft);
    border: 1px solid var(--border);
    max-width: 100%;
  }

  .ui-seg button {
    appearance: none;
    border: none;
    background: transparent;
    color: var(--muted);
    font-family: var(--font-body);
    font-size: 12px;
    font-weight: 500;
    height: 26px;
    padding: 0 11px;
    border-radius: calc(var(--radius) - 2px);
    cursor: pointer;
    white-space: nowrap;
    transition:
      background-color 120ms ease,
      color 120ms ease,
      box-shadow 120ms ease;
  }

  .ui-seg.is-sm button {
    height: 24px;
    padding: 0 9px;
  }

  .ui-seg button:hover:not(:disabled):not(.is-active) {
    color: var(--fg);
  }

  .ui-seg button.is-active {
    background: var(--surface);
    color: var(--fg);
    font-weight: 600;
    box-shadow: 0 1px 2px color-mix(in oklch, var(--shadow-ink) 10%, transparent);
  }

  .ui-seg button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .ui-seg button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    position: relative;
    z-index: 1;
  }
</style>
