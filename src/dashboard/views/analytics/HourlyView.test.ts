import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { DashboardQuery, DashboardSnapshot } from '$lib/contracts/ipc';
import { mockDashboardSnapshot } from '$lib/contracts/mocks';
import HourlyView from '../HourlyView.svelte';

function makeQuery(overrides: Partial<DashboardQuery> = {}): DashboardQuery {
  return {
    range: 'all',
    groupBy: 'model',
    activeClients: [],
    sort: 'cost',
    ...overrides,
  };
}

describe('HourlyView', () => {
  it('renders 12 columns and virtualizes 1,400+ rows to fewer than 80 DOM rows at once', async () => {
    const query = makeQuery({ range: 'all' });
    const snapshot = mockDashboardSnapshot(query);
    expect(snapshot.hourly.length).toBeGreaterThanOrEqual(1400);

    const { container } = render(HourlyView, {
      props: { snapshot, query, onnavigate: vi.fn() },
    });

    const headers = Array.from(container.querySelectorAll('thead th')).map((th) =>
      th.textContent?.replace(/[▲▼↕]/g, '').trim(),
    );
    expect(headers).toEqual([
      'Hour',
      'Harnesses',
      'Top model',
      'Input',
      'Output',
      'Cache read',
      'Cache write',
      'Reasoning',
      'Total tokens',
      'Messages',
      'Turns',
      'Cost',
    ]);

    const initialTrs = container.querySelectorAll('tbody tr');
    expect(initialTrs.length).toBeLessThan(80);
    expect(container.querySelectorAll('tr.ui-table-group').length).toBeGreaterThan(0);

    const wrap = container.querySelector('.ui-table-wrap') as HTMLDivElement;
    wrap.scrollTop = 32 * 500;
    await fireEvent.scroll(wrap);

    const scrolledTrs = container.querySelectorAll('tbody tr');
    expect(scrolledTrs.length).toBeLessThan(80);
  });

  it('caps visible harness badges at 4 and shows +N overflow', () => {
    const query = makeQuery({ range: 'today' });
    const base = mockDashboardSnapshot(query);
    const snapshot: DashboardSnapshot = {
      ...base,
      hourly: [
        {
          datetime: '2026-11-21 14:00',
          tokens: { input: 5000, output: 1200, cacheRead: 10000, cacheWrite: 500, reasoning: 200 },
          cost: 1.25,
          clients: ['claude', 'codex', 'opencode', 'gemini', 'cursor', 'capy'],
          topModel: 'claude-sonnet-4-5',
          messageCount: 14,
          turnCount: 7,
        },
      ],
    };

    const { container } = render(HourlyView, {
      props: { snapshot, query, onnavigate: vi.fn() },
    });

    expect(container.querySelectorAll('.harnesses-list .harness-badge')).toHaveLength(4);
    expect(screen.getByText('+2')).toBeInTheDocument();
  });

  it('renders EmptyState when hourly list is empty', () => {
    const query = makeQuery();
    const snapshot: DashboardSnapshot = { ...mockDashboardSnapshot(query), hourly: [] };
    render(HourlyView, { props: { snapshot, query, onnavigate: vi.fn() } });
    expect(screen.getByText('No hourly activity in this range')).toBeInTheDocument();
  });
});
