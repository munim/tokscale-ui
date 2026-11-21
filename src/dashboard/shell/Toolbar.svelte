<script lang="ts">
  import type {
    DashboardQuery,
    RangePreset,
    SourceHealthRow,
  } from '$lib/contracts/ipc';
  import { HarnessBadge, SegmentedControl, type SegmentOption } from '$lib/ui';

  interface Props {
    query: DashboardQuery;
    sources: SourceHealthRow[];
    customRangeError: string | null;
    onRangeChange: (range: RangePreset) => void;
    onCustomDatesChange: (since: string, until: string) => void;
    onGroupByChange: (groupBy: string) => void;
    onSortChange: (sort: 'cost' | 'tokens' | 'date') => void;
    onToggleClient: (clientId: string) => void;
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

  function handleSinceInput(value: string): void {
    onCustomDatesChange(value, query.customUntil ?? '');
  }

  function handleUntilInput(value: string): void {
    onCustomDatesChange(query.customSince ?? '', value);
  }
</script>

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
    <div class="toolbar-harnesses" role="group" aria-label="Filter by harness">
      {#each sources as src (src.client)}
        {@const pressed = query.activeClients.includes(src.client)}
        <button
          type="button"
          class="harness-chip"
          class:is-pressed={pressed}
          aria-pressed={pressed}
          onclick={() => onToggleClient(src.client)}
        >
          <span class="harness-chip-badge" aria-hidden="true">
            <HarnessBadge id={src.client} size={16} />
          </span>
          <span>{src.label}</span>
        </button>
      {/each}
    </div>
  {/if}
</section>
