import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import QuotaMeterCard from './QuotaMeterCard.svelte';

describe('QuotaMeterCard', () => {
  const now = new Date('2026-11-21T12:00:00Z');

  it('uses accent below 80% and omits Near limit pill', () => {
    const { container } = render(QuotaMeterCard, {
      props: {
        provider: 'OpenAI',
        plan: 'ChatGPT Pro',
        metrics: [
          {
            label: 'Codex cloud tasks',
            usedPercent: 32,
            remainingLabel: '68.0% remaining',
            resetsAt: '2026-11-22T00:00:00Z',
          },
        ],
        now,
      },
    });

    expect(screen.queryByText('Near limit')).not.toBeInTheDocument();
    const fill = container.querySelector('.ui-quota-fill');
    expect(fill).toHaveAttribute('data-tone', 'accent');
    expect(screen.getByText('resets in 12h')).toBeInTheDocument();
  });

  it('shows Near limit pill and warn/danger tones at >=80% and >=95%', () => {
    const { container } = render(QuotaMeterCard, {
      props: {
        provider: 'Anthropic',
        plan: 'Max 20x',
        metrics: [
          {
            label: '5-hour window',
            usedPercent: 84.6,
            resetsAt: '2026-11-21T16:30:00Z',
          },
          {
            label: 'Burst quota',
            usedPercent: 97,
          },
        ],
        now,
      },
    });

    expect(screen.getByText('Near limit')).toBeInTheDocument();
    const fills = container.querySelectorAll('.ui-quota-fill');
    expect(fills[0]).toHaveAttribute('data-tone', 'warn');
    expect(fills[1]).toHaveAttribute('data-tone', 'danger');
  });
});
