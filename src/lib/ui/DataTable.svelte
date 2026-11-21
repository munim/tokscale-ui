<script lang="ts" generics="Row">
  import type { Snippet } from 'svelte';
  import type { ColumnDef } from './types';

  interface Props {
    columns: ColumnDef<Row>[];
    rows: Row[];
    sortKey?: string;
    sortDir?: 'asc' | 'desc';
    onsort?: (key: string) => void;
    groupBy?: (row: Row) => string | null;
    rowKey: (row: Row) => string;
    onrowclick?: (row: Row) => void;
    expandedRowKey?: string | null;
    expandedRow?: Snippet<[Row]>;
    groupHeader?: Snippet<[string, Row[]]>;
    cell?: Snippet<[Row, ColumnDef<Row>]>;
    footer?: Snippet;
    rowHeight?: number;
    overscan?: number;
    maxHeight?: number;
    ariaLabel?: string;
  }

  let {
    columns,
    rows,
    sortKey,
    sortDir = 'desc',
    onsort,
    groupBy,
    rowKey,
    onrowclick,
    expandedRowKey = null,
    expandedRow,
    groupHeader,
    cell,
    footer,
    rowHeight = 32,
    overscan = 10,
    maxHeight = 560,
    ariaLabel,
  }: Props = $props();

  type VirtualItem =
    | { kind: 'group'; key: string; label: string; rows: Row[] }
    | { kind: 'row'; key: string; row: Row };

  let scrollEl = $state<HTMLDivElement | undefined>(undefined);
  let scrollTop = $state(0);
  let measuredHeight = $state(0);

  $effect(() => {
    const el = scrollEl;
    if (!el) return;
    if (el.clientHeight > 0) {
      measuredHeight = el.clientHeight;
    }
    if (typeof ResizeObserver === 'undefined') {
      return;
    }
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        if (entry.contentRect.height > 0) {
          measuredHeight = entry.contentRect.height;
        }
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  });

  let flatItems = $derived.by((): VirtualItem[] => {
    if (!groupBy) {
      return rows.map((row) => ({
        kind: 'row',
        key: rowKey(row),
        row,
      }));
    }

    const items: VirtualItem[] = [];
    let currentLabel: string | null = null;
    let currentGroupRows: Row[] | null = null;

    for (let i = 0; i < rows.length; i++) {
      const row = rows[i];
      const groupLabel = groupBy(row);
      if (groupLabel !== null && groupLabel !== currentLabel) {
        currentLabel = groupLabel;
        currentGroupRows = [row];
        items.push({
          kind: 'group',
          key: `group:${groupLabel}:${i}`,
          label: groupLabel,
          rows: currentGroupRows,
        });
      } else if (groupLabel !== null && currentGroupRows) {
        currentGroupRows.push(row);
      }
      items.push({
        kind: 'row',
        key: rowKey(row),
        row,
      });
    }
    return items;
  });

  let viewportHeight = $derived(measuredHeight > 0 ? measuredHeight : maxHeight);
  let visibleCount = $derived(Math.max(1, Math.ceil(viewportHeight / rowHeight)));

  let windowBounds = $derived.by(() => {
    const total = flatItems.length;
    if (total === 0) {
      return { start: 0, end: 0 };
    }
    const firstVisible = Math.floor(scrollTop / rowHeight);
    const start = Math.max(0, firstVisible - overscan);
    const end = Math.min(total, firstVisible + visibleCount + overscan);
    return { start, end };
  });

  let visibleItems = $derived(flatItems.slice(windowBounds.start, windowBounds.end));
  let topSpacerHeight = $derived(windowBounds.start * rowHeight);
  let bottomSpacerHeight = $derived(
    Math.max(0, (flatItems.length - windowBounds.end) * rowHeight),
  );

  function handleScroll(event: Event): void {
    const target = event.currentTarget as HTMLDivElement;
    scrollTop = target.scrollTop;
  }

  function handleRowKeydown(event: KeyboardEvent, row: Row): void {
    if (!onrowclick) return;
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onrowclick(row);
    }
  }
</script>

<div
  bind:this={scrollEl}
  class="ui-table-wrap"
  style:--ui-table-max-h={`${maxHeight}px`}
  style:--ui-table-row-h={`${rowHeight}px`}
  onscroll={handleScroll}
>
  <table class="ui-table" aria-label={ariaLabel}>
    <thead class="ui-table-head">
      <tr>
        {#each columns as col (col.key)}
          {@const isEnd = col.align === 'end' || col.numeric}
          {@const isSorted = sortKey === col.key}
          <th
            scope="col"
            class:is-end={isEnd}
            style:width={col.width}
            aria-sort={col.sortable
              ? isSorted
                ? sortDir === 'asc'
                  ? 'ascending'
                  : 'descending'
                : 'none'
              : undefined}
          >
            {#if col.sortable && onsort}
              <button
                type="button"
                class="ui-table-sort"
                class:is-end={isEnd}
                class:is-active={isSorted}
                onclick={() => onsort(col.key)}
              >
                <span>{col.label}</span>
                <span class="ui-sort-indicator" aria-hidden="true">
                  {#if isSorted}
                    {sortDir === 'asc' ? '▲' : '▼'}
                  {:else}
                    ↕
                  {/if}
                </span>
              </button>
            {:else}
              <span>{col.label}</span>
            {/if}
          </th>
        {/each}
      </tr>
    </thead>

    <tbody>
      {#if topSpacerHeight > 0}
        <tr class="ui-table-spacer" aria-hidden="true">
          <td colspan={columns.length} style:height={`${topSpacerHeight}px`}></td>
        </tr>
      {/if}

      {#each visibleItems as item (item.key)}
        {#if item.kind === 'group'}
          <tr class="ui-table-group" style:height={`${rowHeight}px`}>
            <td colspan={columns.length}>
              {#if groupHeader}
                {@render groupHeader(item.label, item.rows)}
              {:else}
                <span class="ui-table-group-label">{item.label}</span>
              {/if}
            </td>
          </tr>
        {:else}
          {@const key = item.key}
          {@const isExpanded = expandedRowKey !== null && expandedRowKey === key}
          <tr
            class="ui-table-row"
            class:is-clickable={Boolean(onrowclick)}
            class:is-expanded={isExpanded}
            data-row-key={key}
            style:height={`${rowHeight}px`}
            tabindex={onrowclick ? 0 : undefined}
            aria-expanded={expandedRow ? isExpanded : undefined}
            onclick={onrowclick ? () => onrowclick(item.row) : undefined}
            onkeydown={onrowclick ? (event) => handleRowKeydown(event, item.row) : undefined}
          >
            {#each columns as col (col.key)}
              {@const isEnd = col.align === 'end' || col.numeric}
              {@const cellTitle = col.title ? col.title(item.row) : undefined}
              <td class:is-end={isEnd} class:ui-table-num={isEnd} title={cellTitle}>
                {#if col.cell}
                  {@render col.cell(item.row)}
                {:else if cell}
                  {@render cell(item.row, col)}
                {:else if col.render}
                  {col.render(item.row)}
                {/if}
              </td>
            {/each}
          </tr>
          {#if isExpanded && expandedRow}
            <tr class="ui-table-expanded-row">
              <td colspan={columns.length}>
                {@render expandedRow(item.row)}
              </td>
            </tr>
          {/if}
        {/if}
      {/each}

      {#if bottomSpacerHeight > 0}
        <tr class="ui-table-spacer" aria-hidden="true">
          <td colspan={columns.length} style:height={`${bottomSpacerHeight}px`}></td>
        </tr>
      {/if}
    </tbody>

    {#if footer}
      <tfoot class="ui-table-foot">
        {@render footer()}
      </tfoot>
    {/if}
  </table>
</div>

<style>
  .ui-table-wrap {
    width: 100%;
    max-height: var(--ui-table-max-h, 560px);
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .ui-table {
    width: 100%;
    border-collapse: collapse;
    table-layout: auto;
    font-size: 12.5px;
  }

  .ui-table-head th {
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--surface);
    color: var(--muted);
    font-weight: 600;
    font-size: 11.5px;
    text-align: start;
    padding: 0 10px;
    height: var(--ui-table-row-h, 32px);
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
  }

  .ui-table-head th.is-end {
    text-align: end;
  }

  .ui-table-sort {
    appearance: none;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 0;
    cursor: pointer;
    width: 100%;
  }

  .ui-table-sort.is-end {
    justify-content: flex-end;
  }

  .ui-table-sort:hover,
  .ui-table-sort.is-active {
    color: var(--fg);
  }

  .ui-table-sort:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .ui-sort-indicator {
    font-size: 10px;
    opacity: 0.7;
  }

  .ui-table-row td {
    padding: 0 10px;
    height: var(--ui-table-row-h, 32px);
    border-bottom: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    color: var(--fg);
    vertical-align: middle;
    white-space: nowrap;
  }

  .ui-table-row.is-clickable {
    cursor: pointer;
    transition: background-color 100ms ease;
  }

  .ui-table-row.is-clickable:hover {
    background: var(--fg-soft);
  }

  .ui-table-row.is-clickable:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .ui-table-num {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    text-align: end;
  }

  .ui-table-group td {
    padding: 0 10px;
    height: var(--ui-table-row-h, 32px);
    background: var(--fg-soft);
    border-bottom: 1px solid var(--border);
    font-weight: 600;
    font-size: 12px;
    color: var(--fg);
  }

  .ui-table-expanded-row td {
    padding: 10px 14px;
    background: color-mix(in oklch, var(--fg-soft) 65%, var(--surface));
    border-bottom: 1px solid var(--border);
  }

  .ui-table-spacer td {
    padding: 0;
    border: none;
  }

  .ui-table-foot :global(tr td) {
    position: sticky;
    bottom: 0;
    background: var(--surface);
    border-top: 1px solid var(--border);
    font-weight: 600;
    padding: 0 10px;
    height: var(--ui-table-row-h, 32px);
  }
</style>
