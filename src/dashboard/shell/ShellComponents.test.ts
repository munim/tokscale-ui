import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { DashboardQuery } from '$lib/contracts/ipc';
import { mockDashboardSnapshot } from '$lib/contracts/mocks';
import DashboardApp from '../DashboardApp.svelte';
import AppBar from './AppBar.svelte';
import KpiStrip from './KpiStrip.svelte';
import StatusBar from './StatusBar.svelte';
import TabBar from './TabBar.svelte';
import Toolbar from './Toolbar.svelte';

describe('Toolbar & TabBar', () => {
  it('renders all 10 tabs in order and supports arrow roving + Enter activation', async () => {
    const onchange = vi.fn();
    render(TabBar, {
      props: { activeTab: 'overview', onchange },
    });

    const tabs = screen.getAllByRole('tab');
    expect(tabs.map((el) => el.textContent?.trim())).toEqual([
      'Overview',
      'Usage',
      'Models',
      'Daily',
      'Hourly',
      'Monthly',
      'Sessions',
      'Projects',
      'Stats',
      'Agents',
    ]);

    expect(tabs[0]).toHaveAttribute('aria-selected', 'true');
    tabs[0].focus();
    await fireEvent.keyDown(tabs[0], { key: 'ArrowRight' });
    expect(document.activeElement).toBe(tabs[1]);

    await fireEvent.keyDown(tabs[1], { key: 'Enter' });
    expect(onchange).toHaveBeenCalledWith('usage');
  });

  it('shows inline validation error in Custom range when since > until and toggles harness chips', async () => {
    const query: DashboardQuery = {
      range: 'custom',
      customSince: '2026-11-20',
      customUntil: '2026-11-10',
      groupBy: 'model',
      activeClients: ['claude'],
      sort: 'cost',
    };
    const snapshot = mockDashboardSnapshot({ ...query, range: '30d', activeClients: [] });
    const onToggleClient = vi.fn();

    render(Toolbar, {
      props: {
        query,
        sources: snapshot.sources.clients,
        customRangeError: 'Start date must be on or before end date',
        onRangeChange: vi.fn(),
        onCustomDatesChange: vi.fn(),
        onGroupByChange: vi.fn(),
        onSortChange: vi.fn(),
        onToggleClient,
      },
    });

    expect(screen.getByRole('alert')).toHaveTextContent(
      'Start date must be on or before end date',
    );

    const claudeChip = screen.getByRole('button', { name: 'Claude Code' });
    expect(claudeChip).toHaveAttribute('aria-pressed', 'true');
    await fireEvent.click(claudeChip);
    expect(onToggleClient).toHaveBeenCalledWith('claude');
  });
});

describe('AppBar, KpiStrip, StatusBar, and DashboardApp', () => {
  it('renders AppBar actions, macOS inset, search clear, KpiStrip, and StatusBar', async () => {
    const query: DashboardQuery = {
      range: '30d',
      groupBy: 'model',
      activeClients: [],
      sort: 'cost',
    };
    const snapshot = mockDashboardSnapshot(query);
    const onClearSearch = vi.fn();
    const onOpenSources = vi.fn();
    const onThemeChange = vi.fn();
    const onRefresh = vi.fn();

    render(AppBar, {
      props: {
        search: 'sonnet',
        query,
        sources: snapshot.sources.clients,
        syncing: true,
        onSearchChange: vi.fn(),
        onClearSearch,
        onOpenSources,
        onThemeChange,
        onRefresh,
      },
    });

    const clearBtn = screen.getByRole('button', { name: 'Clear search' });
    await fireEvent.click(clearBtn);
    expect(onClearSearch).toHaveBeenCalledTimes(1);

    const sourcesBtn = screen.getByRole('button', { name: /Sources 7 \/ 10/i });
    await fireEvent.click(sourcesBtn);
    expect(onOpenSources).toHaveBeenCalledTimes(1);

    const themeBtn = screen.getByRole('button', { name: /Theme:/i });
    await fireEvent.click(themeBtn);
    expect(onThemeChange).toHaveBeenCalled();

    render(KpiStrip, { props: { kpis: snapshot.kpis } });
    expect(screen.getByText('Total cost')).toBeInTheDocument();
    expect(screen.getByText('Total tokens')).toBeInTheDocument();
    expect(screen.getByText('Sessions')).toBeInTheDocument();
    expect(screen.getByText('Cost / 1M tokens')).toBeInTheDocument();
    expect(screen.getByText('blended, all models')).toBeInTheDocument();

    render(StatusBar, {
      props: {
        lastSyncedAt: Date.now() - 120_000,
        syncing: true,
        syncMessage: 'Syncing…',
      },
    });
    expect(screen.getByText('Last sync 2m ago')).toBeInTheDocument();
    expect(screen.getByText('Syncing…')).toBeInTheDocument();
    expect(screen.getByText('← → tabs · d t c sort · r refresh · / search')).toBeInTheDocument();
  });

  it('mounts DashboardApp in mock mode and switches across Overview and Usage tabs', async () => {
    render(DashboardApp);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Daily cost by harness' })).toBeInTheDocument();
    });

    await fireEvent.click(screen.getByRole('tab', { name: 'Usage' }));
    expect(screen.getByText('Anthropic')).toBeInTheDocument();
    expect(screen.getByText('Near limit')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('tab', { name: 'Models' }));
    expect(document.querySelector('[data-view="models"]')).not.toBeNull();
  });
});
