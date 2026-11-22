<script lang="ts">
  import { tick } from 'svelte';
  import type {
    DashboardQuery,
    RangePreset,
    SourceHealthRow,
  } from '$lib/contracts/ipc';
  import { formatTokensFull } from '$lib/format/tokens';
  import { harnessColorVar } from '$lib/harnesses';
  import { theme } from '$lib/state/theme.svelte';
  import { SegmentedControl, type SegmentOption } from '$lib/ui';

  interface Props {
    query: DashboardQuery;
    sources: SourceHealthRow[];
    customRangeError: string | null;
    onRangeChange: (range: RangePreset) => void;
    onCustomDatesChange: (since: string, until: string) => void;
    onGroupByChange: (groupBy: string) => void;
    onSortChange: (sort: 'cost' | 'tokens' | 'date') => void;
    onToggleClient: (clientId: string) => void;
    onClearClients?: () => void;
  }

  let {
    query,
    sources,
    customRangeError,
    onRangeChange,
    onCustomDatesChange,
    onGroupByChange,
    onSortChange,
    onToggleClient,
    onClearClients,
  }: Props = $props();

  const RANGE_OPTIONS: SegmentOption[] = [
    { value: 'today', label: 'Today' },
    { value: '7d', label: '7D' },
    { value: '30d', label: '30D' },
    { value: 'all', label: 'All' },
    { value: 'custom', label: 'Custom' },
  ];

  const GROUP_BY_OPTIONS = [
    { value: 'model', label: 'Model' },
    { value: 'client,model', label: 'Harness › Model' },
    { value: 'workspace,model', label: 'Workspace › Model' },
    { value: 'client,session,model', label: 'Harness › Session › Model' },
  ];

  const SORT_OPTIONS: Array<{ value: 'cost' | 'tokens' | 'date'; label: string }> = [
    { value: 'cost', label: 'Cost' },
    { value: 'tokens', label: 'Tokens' },
    { value: 'date', label: 'Date' },
  ];

  let railEl = $state<HTMLDivElement | undefined>(undefined);
  let canScrollLeft = $state(false);
  let canScrollRight = $state(false);

  let dropdownOpen = $state(false);
  let harnessSearch = $state('');
  let triggerEl = $state<HTMLButtonElement | undefined>(undefined);
  let popoverEl = $state<HTMLDivElement | undefined>(undefined);
  let searchInputEl = $state<HTMLInputElement | undefined>(undefined);

  function isDetected(src: SourceHealthRow): boolean {
    return Boolean(
      src.sessionsPathExists || src.messageCount > 0 || src.headlessMessageCount > 0,
    );
  }

  let activeClientSet = $derived(new Set(query.activeClients));

  let primarySources = $derived(
    sources.filter((src) => isDetected(src) || activeClientSet.has(src.client)),
  );

  let secondarySources = $derived(
    sources.filter((src) => !isDetected(src) && !activeClientSet.has(src.client)),
  );

  let detectedTotal = $derived(sources.filter((src) => isDetected(src)).length);

  let normalizedHarnessSearch = $derived(harnessSearch.trim().toLowerCase());

  let filteredSources = $derived(
    normalizedHarnessSearch
      ? sources.filter(
          (src) =>
            src.label.toLowerCase().includes(normalizedHarnessSearch) ||
            src.client.toLowerCase().includes(normalizedHarnessSearch),
        )
      : sources,
  );

  let detectedFiltered = $derived(filteredSources.filter((src) => isDetected(src)));
  let otherFiltered = $derived(filteredSources.filter((src) => !isDetected(src)));

  function handleSinceInput(value: string): void {
    onCustomDatesChange(value, query.customUntil ?? '');
  }

  function handleUntilInput(value: string): void {
    onCustomDatesChange(query.customSince ?? '', value);
  }

  function handleClearAll(): void {
    if (query.activeClients.length === 0) {
      return;
    }
    if (onClearClients) {
      onClearClients();
      return;
    }
    for (const id of [...query.activeClients]) {
      onToggleClient(id);
    }
  }

  function updateScrollState(): void {
    if (!railEl) {
      canScrollLeft = false;
      canScrollRight = false;
      return;
    }
    const { scrollLeft, scrollWidth, clientWidth } = railEl;
    const maxScroll = scrollWidth - clientWidth;
    canScrollLeft = scrollLeft > 2;
    canScrollRight = maxScroll - scrollLeft > 2;
  }

  function nudgeScroll(delta: number): void {
    if (!railEl) {
      return;
    }
    if (typeof railEl.scrollBy === 'function') {
      const reduced =
        typeof window !== 'undefined' &&
        typeof window.matchMedia === 'function' &&
        window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      railEl.scrollBy({ left: delta, behavior: reduced ? 'auto' : 'smooth' });
    } else {
      railEl.scrollLeft += delta;
    }
    updateScrollState();
  }

  function handleRailWheel(event: WheelEvent): void {
    if (!railEl) {
      return;
    }
    if (railEl.scrollWidth <= railEl.clientWidth) {
      return;
    }
    if (Math.abs(event.deltaY) > Math.abs(event.deltaX) && event.deltaY !== 0) {
      event.preventDefault();
      railEl.scrollLeft += event.deltaY;
      updateScrollState();
    }
  }

  async function openDropdown(): Promise<void> {
    harnessSearch = '';
    dropdownOpen = true;
    await tick();
    searchInputEl?.focus();
  }

  function closeDropdown(restoreFocus: boolean): void {
    if (!dropdownOpen) {
      return;
    }
    dropdownOpen = false;
    harnessSearch = '';
    if (restoreFocus) {
      triggerEl?.focus();
    }
  }

  function toggleDropdown(): void {
    if (dropdownOpen) {
      closeDropdown(true);
    } else {
      void openDropdown();
    }
  }

  function clearHarnessSearch(): void {
    harnessSearch = '';
    searchInputEl?.focus();
  }

  $effect(() => {
    void sources.length;
    void query.activeClients.length;
    const el = railEl;
    if (!el) {
      canScrollLeft = false;
      canScrollRight = false;
      return;
    }
    updateScrollState();

    let ro: ResizeObserver | undefined;
    if (typeof ResizeObserver !== 'undefined') {
      ro = new ResizeObserver(() => {
        updateScrollState();
      });
      ro.observe(el);
    }

    const onWinResize = () => updateScrollState();
    if (typeof window !== 'undefined') {
      window.addEventListener('resize', onWinResize);
    }

    return () => {
      ro?.disconnect();
      if (typeof window !== 'undefined') {
        window.removeEventListener('resize', onWinResize);
      }
    };
  });

  $effect(() => {
    if (sources.length === 0 && dropdownOpen) {
      dropdownOpen = false;
      harnessSearch = '';
    }
  });

  $effect(() => {
    if (!dropdownOpen || typeof window === 'undefined' || typeof document === 'undefined') {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        closeDropdown(true);
      }
    };

    const onOutsidePointer = (event: Event) => {
      const target = event.target as Node | null;
      if (!target) {
        return;
      }
      if (popoverEl?.contains(target) || triggerEl?.contains(target)) {
        return;
      }
      closeDropdown(false);
    };

    window.addEventListener('keydown', onKeyDown, true);
    document.addEventListener('pointerdown', onOutsidePointer, true);
    document.addEventListener('mousedown', onOutsidePointer, true);

    return () => {
      window.removeEventListener('keydown', onKeyDown, true);
      document.removeEventListener('pointerdown', onOutsidePointer, true);
      document.removeEventListener('mousedown', onOutsidePointer, true);
    };
  });
</script>

{#snippet harnessChip(src: SourceHealthRow)}
  {@const pressed = activeClientSet.has(src.client)}
  {@const detected = isDetected(src)}
  <button
    type="button"
    class="harness-chip"
    class:is-pressed={pressed}
    class:is-detected={detected}
    aria-pressed={pressed}
    style:--chip-accent={harnessColorVar(src.client, theme.resolved)}
    onclick={() => onToggleClient(src.client)}
  >
    <span class="harness-chip-dot" aria-hidden="true"></span>
    <span class="harness-chip-label">{src.label}</span>
  </button>
{/snippet}

{#snippet dropdownItem(src: SourceHealthRow)}
  {@const pressed = activeClientSet.has(src.client)}
  {@const detected = isDetected(src)}
  {@const totalMessages = src.messageCount + src.headlessMessageCount}
  <button
    type="button"
    role="checkbox"
    class="harness-dropdown-item"
    class:is-checked={pressed}
    class:is-detected={detected}
    aria-checked={pressed}
    aria-label={src.label}
    style:--chip-accent={harnessColorVar(src.client, theme.resolved)}
    onclick={() => onToggleClient(src.client)}
  >
    <span class="harness-dropdown-check" aria-hidden="true">
      {#if pressed}
        <svg viewBox="0 0 12 12" width="10" height="10" fill="none">
          <path
            d="M2.5 6.2L4.9 8.6L9.5 3.6"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      {/if}
    </span>
    <span class="harness-chip-dot" aria-hidden="true"></span>
    <span class="harness-dropdown-label">{src.label}</span>
    <span class="harness-dropdown-count ui-mono">
      {totalMessages > 0 ? formatTokensFull(totalMessages) : '—'}
    </span>
  </button>
{/snippet}

<section class="toolbar" aria-label="Filters and grouping">
  <div class="toolbar-controls">
    <div class="toolbar-group">
      <SegmentedControl
        options={RANGE_OPTIONS}
        value={query.range}
        ariaLabel="Date range"
        size="sm"
        onchange={(val) => onRangeChange(val as RangePreset)}
      />

      {#if query.range === 'custom'}
        <div class="toolbar-custom-dates">
          <input
            type="date"
            class="toolbar-date-input"
            aria-label="From date"
            aria-invalid={Boolean(customRangeError)}
            value={query.customSince ?? ''}
            oninput={(event) => handleSinceInput(event.currentTarget.value)}
          />
          <span class="toolbar-date-sep" aria-hidden="true">→</span>
          <input
            type="date"
            class="toolbar-date-input"
            aria-label="To date"
            aria-invalid={Boolean(customRangeError)}
            value={query.customUntil ?? ''}
            oninput={(event) => handleUntilInput(event.currentTarget.value)}
          />
          {#if customRangeError}
            <span class="toolbar-date-error" role="alert">{customRangeError}</span>
          {/if}
        </div>
      {/if}
    </div>

    <div class="toolbar-selects">
      <label class="toolbar-field">
        <span class="toolbar-label">Group by</span>
        <select
          class="toolbar-select"
          aria-label="Group by"
          value={query.groupBy}
          onchange={(event) => onGroupByChange(event.currentTarget.value)}
        >
          {#each GROUP_BY_OPTIONS as opt (opt.value)}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </label>

      <label class="toolbar-field">
        <span class="toolbar-label">Sort</span>
        <select
          class="toolbar-select"
          aria-label="Sort by"
          value={query.sort ?? 'cost'}
          onchange={(event) =>
            onSortChange(event.currentTarget.value as 'cost' | 'tokens' | 'date')}
        >
          {#each SORT_OPTIONS as opt (opt.value)}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </label>
    </div>
  </div>

  {#if sources.length > 0}
    <div class="toolbar-harness-bar" role="group" aria-label="Filter by harness">
      <div
        class="toolbar-harnesses-rail"
        data-fade-left={canScrollLeft}
        data-fade-right={canScrollRight}
      >
        {#if canScrollLeft}
          <button
            type="button"
            class="toolbar-rail-nudge toolbar-rail-nudge-left"
            aria-label="Scroll harnesses left"
            tabindex="-1"
            onclick={() => nudgeScroll(-240)}
          >
            <svg viewBox="0 0 12 12" width="11" height="11" fill="none" aria-hidden="true">
              <path
                d="M7.5 2.5L4 6L7.5 9.5"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>
        {/if}

        <div
          bind:this={railEl}
          class="toolbar-harnesses-scroll toolbar-harnesses"
          data-fade-left={canScrollLeft}
          data-fade-right={canScrollRight}
          onscroll={updateScrollState}
          onwheel={handleRailWheel}
        >
          {#each primarySources as src (src.client)}
            {@render harnessChip(src)}
          {/each}

          {#if primarySources.length > 0 && secondarySources.length > 0}
            <span class="toolbar-harnesses-sep" aria-hidden="true"></span>
          {/if}

          {#each secondarySources as src (src.client)}
            {@render harnessChip(src)}
          {/each}
        </div>

        {#if canScrollRight}
          <button
            type="button"
            class="toolbar-rail-nudge toolbar-rail-nudge-right"
            aria-label="Scroll harnesses right"
            tabindex="-1"
            onclick={() => nudgeScroll(240)}
          >
            <svg viewBox="0 0 12 12" width="11" height="11" fill="none" aria-hidden="true">
              <path
                d="M4.5 2.5L8 6L4.5 9.5"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>
        {/if}
      </div>

      <div class="toolbar-harnesses-actions">
        {#if query.activeClients.length > 0}
          <button
            type="button"
            class="toolbar-harnesses-clear"
            aria-label="Clear harness filters"
            onclick={handleClearAll}
          >
            Clear
          </button>
        {/if}

        <button
          bind:this={triggerEl}
          type="button"
          class="toolbar-harnesses-trigger"
          class:has-selection={query.activeClients.length > 0}
          class:is-open={dropdownOpen}
          aria-expanded={dropdownOpen}
          aria-haspopup="dialog"
          onclick={toggleDropdown}
        >
          <span>
            {query.activeClients.length === 0
              ? `All ${sources.length}`
              : `${query.activeClients.length} selected`}
          </span>
          <svg
            class="toolbar-harnesses-chevron"
            viewBox="0 0 12 12"
            width="11"
            height="11"
            fill="none"
            aria-hidden="true"
          >
            <path
              d="M3 4.5L6 7.5L9 4.5"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </button>

        {#if dropdownOpen}
          <div
            bind:this={popoverEl}
            class="harness-dropdown"
            role="dialog"
            aria-label="Filter harnesses"
          >
            <div class="harness-dropdown-search-wrap">
              <svg
                class="harness-dropdown-search-icon"
                viewBox="0 0 16 16"
                width="13"
                height="13"
                fill="none"
                aria-hidden="true"
              >
                <circle cx="7" cy="7" r="4.75" stroke="currentColor" stroke-width="1.4" />
                <path
                  d="M10.5 10.5L13.75 13.75"
                  stroke="currentColor"
                  stroke-width="1.4"
                  stroke-linecap="round"
                />
              </svg>
              <input
                bind:this={searchInputEl}
                type="search"
                class="harness-dropdown-search"
                placeholder={`Search ${sources.length} harnesses…`}
                aria-label="Search harnesses"
                bind:value={harnessSearch}
              />
              {#if harnessSearch.length > 0}
                <button
                  type="button"
                  class="harness-dropdown-search-clear"
                  aria-label="Clear harness search"
                  onclick={clearHarnessSearch}
                >
                  ×
                </button>
              {/if}
            </div>

            <div class="harness-dropdown-list">
              {#if detectedFiltered.length === 0 && otherFiltered.length === 0}
                <div class="harness-dropdown-empty">No matching harnesses</div>
              {:else}
                {#if detectedFiltered.length > 0}
                  <div class="harness-dropdown-section">
                    <div class="harness-dropdown-heading">
                      Detected on this Mac ({detectedFiltered.length})
                    </div>
                    {#each detectedFiltered as src (src.client)}
                      {@render dropdownItem(src)}
                    {/each}
                  </div>
                {/if}

                {#if otherFiltered.length > 0}
                  <div class="harness-dropdown-section">
                    <div class="harness-dropdown-heading">
                      Other supported ({otherFiltered.length})
                    </div>
                    {#each otherFiltered as src (src.client)}
                      {@render dropdownItem(src)}
                    {/each}
                  </div>
                {/if}
              {/if}
            </div>

            <div class="harness-dropdown-footer">
              <span class="harness-dropdown-summary">
                {query.activeClients.length > 0
                  ? `${query.activeClients.length} selected`
                  : `${detectedTotal} detected`}
              </span>
              <button
                type="button"
                class="harness-dropdown-clear"
                disabled={query.activeClients.length === 0}
                onclick={handleClearAll}
              >
                Clear all
              </button>
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</section>
