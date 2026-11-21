import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import StackedBarChart from './StackedBarChart.svelte';
import type { StackedBarItem } from './types';

describe('StackedBarChart', () => {
  const manySegmentBars: StackedBarItem[] = [
    {
      label: 'Nov 20',
      segments: [
        { id: 'claude', label: 'Claude Code', color: 'var(--c-claude)', value: 20 },
        { id: 'codex', label: 'Codex', color: 'var(--c-codex)', value: 15 },
        { id: 'opencode', label: 'OpenCode', color: 'var(--c-opencode)', value: 12 },
        { id: 'gemini', label: 'Gemini CLI', color: 'var(--c-gemini)', value: 10 },
        { id: 'cursor', label: 'Cursor', color: 'oklch(62% 0.12 10)', value: 8 },
        { id: 'trae', label: 'Trae', color: 'oklch(62% 0.12 90)', value: 3 },
        { id: 'warp', label: 'Warp', color: 'oklch(62% 0.12 180)', value: 2 },
      ],
    },
    {
      label: 'Nov 21',
      segments: [
        { id: 'claude', label: 'Claude Code', color: 'var(--c-claude)', value: 18 },
        { id: 'codex', label: 'Codex', color: 'var(--c-codex)', value: 11 },
      ],
    },
  ];

  it('rolls segments beyond the top 5 into Other in compact mode', async () => {
    const { container } = render(StackedBarChart, {
      props: {
        bars: manySegmentBars,
        valueFormat: 'usd',
        compact: true,
        ariaLabel: 'Cost by harness',
      },
    });

    const firstBar = container.querySelector('[data-bar-index="0"]') as SVGGElement;
    const segRects = firstBar.querySelectorAll('rect.ui-chart-seg');
    expect(segRects).toHaveLength(6);

    const segmentIds = Array.from(segRects).map((el) => el.getAttribute('data-segment-id'));
    expect(segmentIds).toContain('other');
    expect(segmentIds).not.toContain('trae');
    expect(segmentIds).not.toContain('warp');

    await fireEvent.focus(firstBar);
    expect(screen.getByText('Nov 20 · $70.00')).toBeInTheDocument();
    expect(screen.getByText('Other')).toBeInTheDocument();
    expect(screen.getByText('$5.00')).toBeInTheDocument();

    const secondBar = container.querySelector('[data-bar-index="1"]') as SVGGElement;
    expect(secondBar).toHaveClass('is-dimmed');
  });

  it('supports arrow-key navigation across bars and token formatting', async () => {
    const { container } = render(StackedBarChart, {
      props: {
        bars: manySegmentBars,
        valueFormat: 'tokens',
        compact: false,
        ariaLabel: 'Tokens by harness',
      },
    });

    const bars = container.querySelectorAll('[data-bar-index]');
    expect(bars[0].querySelectorAll('rect.ui-chart-seg')).toHaveLength(7);

    (bars[0] as unknown as HTMLElement).focus();
    await fireEvent.focus(bars[0]);
    expect(screen.getByText('Nov 20 · 70')).toBeInTheDocument();

    await fireEvent.keyDown(bars[0], { key: 'ArrowRight' });
    expect(document.activeElement).toBe(bars[1]);
    expect(screen.getByText('Nov 21 · 29')).toBeInTheDocument();
  });
});
