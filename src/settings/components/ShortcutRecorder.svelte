<script lang="ts">
  interface Props {
    value: string | undefined;
    onchange: (next: string | undefined) => void;
  }

  let { value, onchange }: Props = $props();

  let recording = $state(false);
  let recorderEl = $state<HTMLButtonElement | null>(null);

  const IGNORED_KEYS = new Set(['Control', 'Shift', 'Alt', 'Meta', 'CapsLock', 'Tab']);

  function normalizeKeyName(key: string): string | null {
    if (IGNORED_KEYS.has(key)) return null;
    if (key === ' ') return 'Space';
    if (key.length === 1) return key.toUpperCase();
    if (/^F\d{1,2}$/i.test(key)) return key.toUpperCase();
    const map: Record<string, string> = {
      ArrowUp: 'Up',
      ArrowDown: 'Down',
      ArrowLeft: 'Left',
      ArrowRight: 'Right',
      Enter: 'Return',
      Backspace: 'Backspace',
      Delete: 'Delete',
    };
    return map[key] ?? null;
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (!recording) return;
    event.preventDefault();
    event.stopPropagation();

    if (event.key === 'Escape') {
      recording = false;
      return;
    }

    const key = normalizeKeyName(event.key);
    if (!key) return;

    const parts: string[] = [];
    if (event.metaKey || event.ctrlKey) parts.push('CmdOrCtrl');
    if (event.altKey) parts.push('Alt');
    if (event.shiftKey) parts.push('Shift');

    // Require at least one modifier so we don't hijack single letters globally.
    if (parts.length === 0) {
      return;
    }

    parts.push(key);
    const accelerator = parts.join('+');
    recording = false;
    onchange(accelerator);
  }

  function startRecording(): void {
    recording = true;
    recorderEl?.focus();
  }
</script>

<div class="shortcut-recorder">
  <button
    bind:this={recorderEl}
    type="button"
    class="shortcut-pill ui-mono"
    class:is-recording={recording}
    aria-label="Global shortcut"
    onclick={startRecording}
    onblur={() => (recording = false)}
    onkeydown={handleKeydown}
  >
    {#if recording}
      Press shortcut…
    {:else if value}
      {value}
    {:else}
      Click to record
    {/if}
  </button>

  {#if value}
    <button
      type="button"
      class="s-btn s-btn-sm s-btn-ghost"
      aria-label="Clear shortcut"
      onclick={() => {
        recording = false;
        onchange(undefined);
      }}
    >
      Clear
    </button>
  {/if}
</div>

<style>
  .shortcut-recorder {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .shortcut-pill {
    appearance: none;
    height: 30px;
    min-width: 132px;
    padding: 0 10px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--fg);
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 120ms ease, background-color 120ms ease, box-shadow 120ms ease;
  }

  .shortcut-pill:hover {
    background: var(--fg-soft);
  }

  .shortcut-pill.is-recording {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-strong);
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 20%, transparent);
  }

  .shortcut-pill:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
