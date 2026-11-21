import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { DashboardQuery } from '$lib/contracts/ipc';
import { mockDashboardSnapshot } from '$lib/contracts/mocks';
import OverviewView from '../views/OverviewView.svelte';

describe('OverviewView', () => {
  const query: DashboardQuery = {
    range: '30d',
    groupBy: 'model',
    activeClients: [],
    sort: 'cost',
  };

  it('renders stacked bar chart, top 5 models, and 8-column harness split table', () => {
    const snapshot = mockDashboardSnapshot(query);
    const onnavigate = vi.fn();

    render(OverviewView, {
      props: { snapshot, query, onnavigate },
    });

    expect(screen.getByRole('heading', { name: 'Daily cost by harness' })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Daily cost by harness' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Top 5 models' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Harness split' })).toBeInTheDocument();

    const table = screen.getByRole('table', { name: 'Harness split' });
    const headers = table.querySelectorAll('thead th');
    expect(headers).toHaveLength(8);
  });

  it('emits onnavigate to models tab when a Top 5 model row is clicked', async () => {
    const snapshot = mockDashboardSnapshot(query);
    const expectedTopModel = [...snapshot.models].sort((a, b) => b.cost - a.cost)[0];
    const onnavigate = vi.fn();

    const { container } = render(OverviewView, {
      props: { snapshot, query, onnavigate },
    });

    const topModelButtons = container.querySelectorAll('.top-model-row');
    expect(topModelButtons.length).toBeGreaterThan(0);
    expect(topModelButtons.length).toBeLessThanOrEqual(5);

    await fireEvent.click(topModelButtons[0]);

    expect(onnavigate).toHaveBeenCalledWith({
      tab: 'models',
      query: expectedTopModel.model,
    });
  });

  it('emits onnavigate with harness id when a Harness split row is clicked', async () => {
    const snapshot = mockDashboardSnapshot(query);
    const onnavigate = vi.fn();

    render(OverviewView, {
      props: { snapshot, query, onnavigate },
    });

    const claudeCell = screen.getByText('Claude Code').closest('tr');
    expect(claudeCell).not.toBeNull();
    await fireEvent.click(claudeCell!);

    expect(onnavigate).toHaveBeenCalledWith({
      tab: 'models',
      query: 'claude',
    });
  });
});
