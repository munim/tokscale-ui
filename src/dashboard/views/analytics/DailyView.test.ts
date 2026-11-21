import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { DashboardQuery, DashboardSnapshot } from '$lib/contracts/ipc';
import { mockDashboardSnapshot } from '$lib/contracts/mocks';
import DailyView from '../DailyView.svelte';

function makeQuery(overrides: Partial<DashboardQuery> = {}): DashboardQuery {
  return {
    range: '7d',
    groupBy: 'model',
    activeClients: [],
    sort: 'cost',
    ...overrides,
  };
}

describe('DailyView', () => {
  it('renders 11 columns newest first by default', () => {
    const query = makeQuery();
    const snapshot = mockDashboardSnapshot(query);
    const { container } = render(DailyView, {
      props: { snapshot, query, onnavigate: vi.fn() },
    });

    const headers = Array.from(container.querySelectorAll('thead th')).map((th) =>
      th.textContent?.replace(/[▲▼↕]/g, '').trim(),
    );
    expect(headers).toEqual([
      'Date',
      'Harnesses',
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

    const firstRow = container.querySelector('tbody tr[data-row-key]');
    expect(firstRow).toHaveAttribute('data-row-key', snapshot.daily[0].date);
  });

  it('expands one row at a time to show per-harness sourceBreakdown', async () => {
    const query = makeQuery();
    const snapshot = mockDashboardSnapshot(query);
    const { container } = render(DailyView, {
      props: { snapshot, query, onnavigate: vi.fn() },
    });

    const day0 = snapshot.daily[0].date;
    const day1 = snapshot.daily[1].date;

    const row0 = container.querySelector(`tr[data-row-key="${day0}"]`) as HTMLTableRowElement;
    const row1 = container.querySelector(`tr[data-row-key="${day1}"]`) as HTMLTableRowElement;

    await fireEvent.click(row0);
    expect(container.querySelectorAll('tr.ui-table-expanded-row')).toHaveLength(1);
    expect(container.querySelector(`[data-expanded-date="${day0}"]`)).toBeInTheDocument();

    await fireEvent.click(row1);
    expect(container.querySelectorAll('tr.ui-table-expanded-row')).toHaveLength(1);
    expect(container.querySelector(`[data-expanded-date="${day0}"]`)).not.toBeInTheDocument();
    expect(container.querySelector(`[data-expanded-date="${day1}"]`)).toBeInTheDocument();

    await fireEvent.click(row1);
    expect(container.querySelectorAll('tr.ui-table-expanded-row')).toHaveLength(0);
  });

  it('renders EmptyState when daily list is empty', () => {
    const query = makeQuery();
    const snapshot: DashboardSnapshot = { ...mockDashboardSnapshot(query), daily: [] };
    render(DailyView, { props: { snapshot, query, onnavigate: vi.fn() } });
    expect(screen.getByText('No daily activity in this range')).toBeInTheDocument();
  });
});
