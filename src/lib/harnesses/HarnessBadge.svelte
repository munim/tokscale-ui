<script lang="ts">
  import { HARNESS_ICONS } from './icons';
  import { resolveHarness } from './registry';

  interface Props {
    id: string;
    size?: 16 | 24;
  }

  let { id, size = 16 }: Props = $props();

  let meta = $derived(resolveHarness(id));
  let iconShape = $derived(HARNESS_ICONS[meta.id] ?? null);
  let badgeVar = $derived(meta.cssVar ? `var(${meta.cssVar})` : null);
</script>

<span
  class="harness-badge"
  class:is-icon={Boolean(iconShape)}
  class:is-monogram={!iconShape}
  role="img"
  aria-label={meta.label}
  title={meta.label}
  data-harness={meta.id}
  data-size={size}
  style:--badge-size={`${size}px`}
  style:--badge-light={badgeVar ?? meta.colorLight}
  style:--badge-dark={badgeVar ?? meta.colorDark}
>
  {#if iconShape}
    <svg
      class="harness-icon"
      viewBox="0 0 16 16"
      width={Math.round(size * 0.68)}
      height={Math.round(size * 0.68)}
      aria-hidden="true"
    >
      {#if iconShape.rects}
        {#each iconShape.rects as rect}
          <rect
            x={rect.x}
            y={rect.y}
            width={rect.width}
            height={rect.height}
            rx={rect.rx}
            fill={rect.fill ?? 'none'}
            stroke={rect.stroke ?? 'none'}
            stroke-width={rect.strokeWidth}
          />
        {/each}
      {/if}
      {#each iconShape.paths as path}
        <path
          d={path.d}
          fill={path.fill ?? 'none'}
          stroke={path.stroke ?? 'none'}
          stroke-width={path.strokeWidth}
          stroke-linecap={path.strokeLinecap}
          stroke-linejoin={path.strokeLinejoin}
        />
      {/each}
      {#if iconShape.circles}
        {#each iconShape.circles as circle}
          <circle
            cx={circle.cx}
            cy={circle.cy}
            r={circle.r}
            fill={circle.fill ?? 'currentColor'}
          />
        {/each}
      {/if}
    </svg>
  {:else}
    <span class="harness-monogram" aria-hidden="true">{meta.monogram}</span>
  {/if}
</span>

<style>
  .harness-badge {
    --badge-color: var(--badge-light);
    --badge-ink: oklch(99% 0 0);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: var(--badge-size, 16px);
    height: var(--badge-size, 16px);
    border-radius: 999px;
    box-sizing: border-box;
    vertical-align: middle;
    user-select: none;
  }

  :global(:root[data-theme='dark']) .harness-badge {
    --badge-color: var(--badge-dark);
    --badge-ink: oklch(16% 0.015 240);
  }

  .harness-badge.is-icon {
    color: var(--badge-color);
    background: color-mix(in oklch, var(--badge-color) 15%, var(--surface));
    border: 1px solid color-mix(in oklch, var(--badge-color) 35%, transparent);
  }

  .harness-badge.is-monogram {
    background: var(--badge-color);
    color: var(--badge-ink);
    font-family: var(--font-mono);
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1;
  }

  .harness-badge[data-size='16'] .harness-monogram {
    font-size: 7.5px;
  }

  .harness-badge[data-size='24'] .harness-monogram {
    font-size: 10px;
  }

  .harness-icon {
    display: block;
    flex-shrink: 0;
  }
</style>
