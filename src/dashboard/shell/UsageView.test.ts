import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { DashboardQuery, DashboardSnapshot } from '$lib/contracts/ipc';
import { mockDashboardSnapshot } from '$lib/contracts/mocks';
import UsageView from '../views/UsageView.svelte';

describe('UsageView', () => {
  const query: DashboardQuery = {
    range: '30d',
    groupBy: 'model',
    activeClients: [],
    sort: 'cost',
  };

  it('shows Near limit pill when a quota metric is at or above 80%', () => {
    const base = mockDashboardSnapshot(query);
    const snapshot: DashboardSnapshot = {
      ...base,
      usage: [
        {
          provider: 'Anthropic',
          plan: 'Max 20x',
          email: 'dev@tokscale.local',
          metrics: [
            {
              label: '5-hour session window',
              remainingLabel: '20.0% remaining',
              remainingPercent: 20,
              usedPercent: 80,
              resetsAt: '2026-11-21T16:30:00Z',
            },
          ],
        },
        {
          provider: 'OpenAI',
          plan: 'ChatGPT Pro',
          metrics: [
            {
              label: 'Codex cloud tasks',
              remainingLabel: '65.0% remaining',
              remainingPercent: 65,
              usedPercent: 35,
            },
          ],
        },
      ],
    };

    const { container } = render(UsageView, {
      props: { snapshot, query, onnavigate: vi.fn() },
    });

    expect(screen.getByText('Anthropic')).toBeInTheDocument();
    expect(screen.getByText('OpenAI')).toBeInTheDocument();
    expect(screen.getAllByText('Near limit')).toHaveLength(1);
    const cards = container.querySelectorAll('.ui-quota-card');
    expect(cards[0]).toHaveAttribute('data-near-limit', 'true');
    expect(cards[1]).toHaveAttribute('data-near-limit', 'false');
  });

  it('renders EmptyState when snapshot.usage is empty', () => {
    const base = mockDashboardSnapshot(query);
    const snapshot: DashboardSnapshot = {
      ...base,
      usage: [],
    };

    render(UsageView, {
      props: { snapshot, query, onnavigate: vi.fn() },
    });

    expect(screen.getByText('No subscription quotas found')).toBeInTheDocument();
    expect(
      screen.getByText('Quotas appear once a subscription provider is signed in to tokscale.'),
    ).toBeInTheDocument();
  });
});
