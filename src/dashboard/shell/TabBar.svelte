<script lang="ts">
  import type { DashboardTab } from '$lib/contracts/ipc';
  import { DASHBOARD_TABS } from './router.svelte';

  interface Props {
    activeTab: DashboardTab;
    onchange: (tab: DashboardTab) => void;
  }

  let { activeTab, onchange }: Props = $props();

  let tabEls = $state<Array<HTMLButtonElement | undefined>>([]);
  let focusedIndex = $state(0);

  $effect(() => {
    const idx = DASHBOARD_TABS.findIndex((t) => t.id === activeTab);
    if (idx >= 0) {
      focusedIndex = idx;
    }
  });

  function moveFocus(nextIndex: number): void {
    const count = DASHBOARD_TABS.length;
    const wrapped = (nextIndex + count) % count;
    focusedIndex = wrapped;
    tabEls[wrapped]?.focus();
  }

  function handleKeydown(event: KeyboardEvent, index: number): void {
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
      event.preventDefault();
      moveFocus(index + 1);
      return;
    }
    if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
      event.preventDefault();
      moveFocus(index - 1);
      return;
    }
    if (event.key === 'Home') {
      event.preventDefault();
      moveFocus(0);
      return;
    }
    if (event.key === 'End') {
      event.preventDefault();
      moveFocus(DASHBOARD_TABS.length - 1);
      return;
    }
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      const target = DASHBOARD_TABS[index];
      if (target) {
        onchange(target.id);
      }
    }
  }
</script>

<div class="tab-bar" role="tablist" aria-label="Dashboard sections">
  {#each DASHBOARD_TABS as tab, i (tab.id)}
    {@const isActive = tab.id === activeTab}
    <button
      bind:this={tabEls[i]}
      type="button"
      role="tab"
      class="tab-btn"
      class:is-active={isActive}
      aria-selected={isActive}
      tabindex={i === focusedIndex ? 0 : -1}
      onclick={() => {
        focusedIndex = i;
        onchange(tab.id);
      }}
      onfocus={() => (focusedIndex = i)}
      onkeydown={(event) => handleKeydown(event, i)}
    >
      {tab.label}
    </button>
  {/each}
</div>
