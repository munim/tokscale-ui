<script lang="ts">
  import { SegmentedControl, type SegmentOption } from '$lib/ui';
  import type { TrayRange } from '../store.svelte';

  interface Props {
    value: TrayRange;
    onchange: (range: TrayRange) => void;
  }

  let { value, onchange }: Props = $props();

  const options: SegmentOption[] = [
    { value: 'today', label: 'Today' },
    { value: '7d', label: '7D' },
    { value: '30d', label: '30D' },
    { value: 'all', label: 'All' },
  ];
</script>

<div class="tray-range">
  <SegmentedControl
    {options}
    {value}
    ariaLabel="Usage range"
    onchange={(next) => onchange(next as TrayRange)}
  />
</div>

<style>
  .tray-range {
    display: flex;
    flex-shrink: 0;
  }

  .tray-range :global(.ui-seg) {
    width: 100%;
    padding: 1px;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .tray-range :global(.ui-seg button) {
    height: 30px;
    padding: 0 10px;
    text-align: center;
  }
</style>
