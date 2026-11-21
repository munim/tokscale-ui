<script lang="ts">
  interface Props {
    size?: number;
    label?: string;
  }

  let { size = 20, label }: Props = $props();
</script>

<svg
  class="ui-brand-mark"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
>
  <rect
    x="2"
    y="2"
    width="20"
    height="20"
    rx="6"
    stroke="currentColor"
    stroke-width="1.75"
  />
  <rect x="6" y="13" width="3" height="5" rx="1" fill="var(--accent)" />
  <rect x="10.5" y="9" width="3" height="9" rx="1" fill="var(--accent)" />
  <rect x="15" y="6" width="3" height="12" rx="1" fill="currentColor" />
</svg>

<style>
  .ui-brand-mark {
    display: inline-block;
    flex-shrink: 0;
    color: var(--fg);
    vertical-align: middle;
  }
</style>
