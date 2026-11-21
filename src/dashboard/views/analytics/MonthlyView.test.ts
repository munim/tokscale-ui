import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { DashboardQuery, DashboardSnapshot } from '$lib/contracts/ipc';
import { mockDashboardSnapshot } from '$lib/contracts/mocks';
import { formatUsd } from '$lib/format';
import MonthlyView from '../MonthlyView.svelte';

function makeQuery(overrides: Partial<DashboardQuery> = {}): DashboardQuery {
  return {
    range: 'all',
    groupBy: 'model',
    activeClients: [],
    sort: 'cost',
    ...overrides,
  };
}

describe('MonthlyView', () => {
  it('renders 8 columns newest first and includes a totals footer row', () => {
    const query = makeQuery();
    const snapshot = mockDashboardSnapshot(query);
    const { container } = render(MonthlyView, {
      props: { snapshot, query, onnavigate: vi.fn() },
    });

    const headers = Array.from(container.querySelectorAll('thead th')).map((th) =>
      th.textContent?.replace(/[▲▼↕]/g, '').trim(),
    );
    expect(headers).toEqual([
      'Month',
      'Models',
      'Input',
      'Output',
      'Cache read',
      'Cache write',
      'Reasoning',
      'Cost',
    ]);

    const firstRow = container.querySelector('tbody tr[data-row-key]');
    expect(firstRow).toHaveAttribute('data-row-key', snapshot.monthly[0].month);

    const footerRow = container.querySelector('tfoot tr.monthly-totals-row');
    expect(footerRow).toBeInTheDocument();
    expect(footerRow).toHaveTextContent('Total');
    const totalCost = snapshot.monthly.reduce((sum, row) => sum + row.cost, 0);
    expect(footerRow).toHaveTextContent(formatUsd(totalCost));
  });

  it('renders EmptyState when monthly list is empty', () => {
    const query = makeQuery();
    const snapshot: DashboardSnapshot = { ...mockDashboardSnapshot(query), monthly: [] };
    render(MonthlyView, { props: { snapshot, query, onnavigate: vi.fn() } });
    expect(screen.getByText('No monthly data in this range')).toBeInTheDocument();
  });
});
