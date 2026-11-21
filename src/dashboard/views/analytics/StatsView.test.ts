import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { DashboardQuery, DashboardSnapshot } from '$lib/contracts/ipc';
import { mockDashboardSnapshot } from '$lib/contracts/mocks';
import StatsView from '../StatsView.svelte';

function makeQuery(overrides: Partial<DashboardQuery> = {}): DashboardQuery {
  return {
    range: 'all',
    groupBy: 'model',
    activeClients: [],
    sort: 'cost',
    ...overrides,
  };
}

describe('StatsView', () => {
  it('defaults to 52 weeks, shows 5 KpiCards with mock streak values, and toggles to 26 weeks', async () => {
    const query = makeQuery();
    const snapshot = mockDashboardSnapshot(query);
    const { container } = render(StatsView, {
      props: { snapshot, query, onnavigate: vi.fn() },
    });

    const toggle52 = screen.getByRole('radio', { name: '52 weeks' });
    const toggle26 = screen.getByRole('radio', { name: '26 weeks' });
    expect(toggle52).toHaveAttribute('aria-checked', 'true');
    expect(container.querySelectorAll('.heatmap-week')).toHaveLength(52);

    expect(screen.getByText('Active days')).toBeInTheDocument();
    expect(screen.getByText('Longest streak')).toBeInTheDocument();
    expect(screen.getByText('42 days')).toBeInTheDocument();
    expect(screen.getByText('Current streak')).toBeInTheDocument();
    expect(screen.getByText('26 days')).toBeInTheDocument();
    expect(screen.getByText('Best day')).toBeInTheDocument();
    expect(screen.getByText('Avg / active day')).toBeInTheDocument();

    await fireEvent.click(toggle26);
    expect(toggle26).toHaveAttribute('aria-checked', 'true');
    expect(container.querySelectorAll('.heatmap-week')).toHaveLength(26);
  });

  it('moves focus across cells with arrow keys and updates the readout on focus/Enter', async () => {
    const query = makeQuery();
    const snapshot = mockDashboardSnapshot(query);
    const { container } = render(StatsView, {
      props: { snapshot, query, onnavigate: vi.fn() },
    });

    const firstCell = container.querySelector('.heatmap-cell[data-coord="0:0"]') as HTMLButtonElement;
    const rightCell = container.querySelector('.heatmap-cell[data-coord="1:0"]') as HTMLButtonElement;
    const downCell = container.querySelector('.heatmap-cell[data-coord="1:1"]') as HTMLButtonElement;

    firstCell.focus();
    await fireEvent.focus(firstCell);
    const readout = screen.getByTestId('heatmap-readout');
    expect(readout).toHaveTextContent(firstCell.dataset.date ?? '');

    await fireEvent.keyDown(firstCell, { key: 'ArrowRight' });
    expect(document.activeElement).toBe(rightCell);
    expect(readout).toHaveTextContent(rightCell.dataset.date ?? '');

    await fireEvent.keyDown(rightCell, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(downCell);

    await fireEvent.keyDown(downCell, { key: 'Enter' });
    expect(readout).toHaveTextContent(downCell.dataset.date ?? '');
  });

  it('renders EmptyState when graph has no cells', () => {
    const query = makeQuery();
    const snapshot: DashboardSnapshot = {
      ...mockDashboardSnapshot(query),
      graph: { weeks: [] },
    };
    render(StatsView, { props: { snapshot, query, onnavigate: vi.fn() } });
    expect(screen.getByText('No activity history yet')).toBeInTheDocument();
  });
});
