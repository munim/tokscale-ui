import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { DashboardQuery, SourceHealthRow } from '$lib/contracts/ipc';
import { mockDashboardSnapshot } from '$lib/contracts/mocks';
import DashboardApp from '../DashboardApp.svelte';
import AppBar from './AppBar.svelte';
import KpiStrip from './KpiStrip.svelte';
import StatusBar from './StatusBar.svelte';
import TabBar from './TabBar.svelte';
import Toolbar from './Toolbar.svelte';

function make62Sources(): SourceHealthRow[] {
  const detectedDefs = [
    { client: 'claude', label: 'Claude Code', msgs: 1840, headless: 320 },
    { client: 'codex', label: 'Codex', msgs: 920, headless: 110 },
    { client: 'opencode', label: 'OpenCode', msgs: 640, headless: 85 },
    { client: 'cursor', label: 'Cursor', msgs: 410, headless: 0 },
  ];

  const rows: SourceHealthRow[] = [];
  let detectedIdx = 0;
  let otherIdx = 1;

  for (let i = 0; i < 62; i++) {
    // Interleave detected sources among undetected ones to verify primary-group ordering
    if ((i === 2 || i === 7 || i === 15 || i === 28) && detectedIdx < detectedDefs.length) {
      const d = detectedDefs[detectedIdx++];
      rows.push({
        client: d.client,
        label: d.label,
        sessionsPath: `~/.${d.client}/sessions`,
        sessionsPathExists: true,
        messageCount: d.msgs,
        headlessSupported: d.headless > 0,
        headlessMessageCount: d.headless,
        status: 'ok',
        credentialed: false,
        apiBacked: false,
      });
    } else {
      const id = otherIdx === 1 ? 'trae' : otherIdx === 2 ? 'windsurf' : `harness-${otherIdx}`;
      const label =
        otherIdx === 1 ? 'Trae' : otherIdx === 2 ? 'Windsurf' : `Agent Harness ${otherIdx}`;
      otherIdx++;
      rows.push({
        client: id,
        label,
        sessionsPath: `~/.${id}/sessions`,
        sessionsPathExists: false,
        messageCount: 0,
        headlessSupported: false,
        headlessMessageCount: 0,
        status: 'warn',
        credentialed: false,
        apiBacked: false,
      });
    }
  }
  return rows;
}

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

  it('renders 62 harnesses in a single-line smart-ordered rail with searchable grouped dropdown, Escape, and clear actions', async () => {
    const sources = make62Sources();
    let activeClients: string[] = [];

    const onToggleClient = vi.fn((clientId: string) => {
      if (activeClients.includes(clientId)) {
        activeClients = activeClients.filter((id) => id !== clientId);
      } else {
        activeClients = [...activeClients, clientId];
      }
      void rerender({
        query: { ...baseQuery, activeClients },
        sources,
        customRangeError: null,
        onRangeChange: vi.fn(),
        onCustomDatesChange: vi.fn(),
        onGroupByChange: vi.fn(),
        onSortChange: vi.fn(),
        onToggleClient,
        onClearClients,
      });
    });

    const onClearClients = vi.fn(() => {
      activeClients = [];
      void rerender({
        query: { ...baseQuery, activeClients: [] },
        sources,
        customRangeError: null,
        onRangeChange: vi.fn(),
        onCustomDatesChange: vi.fn(),
        onGroupByChange: vi.fn(),
        onSortChange: vi.fn(),
        onToggleClient,
        onClearClients,
      });
    });

    const baseQuery: DashboardQuery = {
      range: '30d',
      groupBy: 'model',
      activeClients: [],
      sort: 'cost',
    };

    const { container, rerender } = render(Toolbar, {
      props: {
        query: baseQuery,
        sources,
        customRangeError: null,
        onRangeChange: vi.fn(),
        onCustomDatesChange: vi.fn(),
        onGroupByChange: vi.fn(),
        onSortChange: vi.fn(),
        onToggleClient,
        onClearClients,
      },
    });

    // Single-line scrollable rail renders all 62 chips with 7px dots (no monogram badges)
    const rail = container.querySelector('.toolbar-harnesses-scroll') as HTMLDivElement;
    expect(rail).not.toBeNull();
    expect(rail.querySelectorAll('.ui-harness-badge')).toHaveLength(0);

    const chips = Array.from(rail.querySelectorAll<HTMLButtonElement>('.harness-chip'));
    expect(chips).toHaveLength(62);
    expect(chips.slice(0, 4).map((el) => el.textContent?.trim())).toEqual([
      'Claude Code',
      'Codex',
      'OpenCode',
      'Cursor',
    ]);
    expect(rail.querySelector('.toolbar-harnesses-sep')).not.toBeNull();

    // Pinned dropdown trigger shows "All 62" and opens grouped searchable popover
    const trigger = screen.getByRole('button', { name: 'All 62' });
    expect(trigger).toHaveAttribute('aria-haspopup', 'dialog');
    expect(trigger).toHaveAttribute('aria-expanded', 'false');

    await fireEvent.click(trigger);
    expect(trigger).toHaveAttribute('aria-expanded', 'true');

    const dialog = screen.getByRole('dialog', { name: 'Filter harnesses' });
    const searchInput = within(dialog).getByRole('searchbox', { name: 'Search harnesses' });
    await waitFor(() => {
      expect(document.activeElement).toBe(searchInput);
    });
    expect(searchInput).toHaveAttribute('placeholder', 'Search 62 harnesses…');

    expect(within(dialog).getByText('Detected on this Mac (4)')).toBeInTheDocument();
    expect(within(dialog).getByText('Other supported (58)')).toBeInTheDocument();
    expect(within(dialog).getByText('4 detected')).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Clear all' })).toBeDisabled();

    // Search filters case-insensitively and handles empty results
    await fireEvent.input(searchInput, { target: { value: 'wind' } });
    expect(within(dialog).queryByText(/Detected on this Mac/)).not.toBeInTheDocument();
    expect(within(dialog).getByText('Other supported (1)')).toBeInTheDocument();

    // Selecting an undetected harness via checkbox promotes it into the primary rail group
    const windsurfCheckbox = within(dialog).getByRole('checkbox', { name: 'Windsurf' });
    expect(windsurfCheckbox).toHaveAttribute('aria-checked', 'false');
    await fireEvent.click(windsurfCheckbox);
    expect(onToggleClient).toHaveBeenCalledWith('windsurf');

    await waitFor(() => {
      expect(within(dialog).getByRole('checkbox', { name: 'Windsurf' })).toHaveAttribute(
        'aria-checked',
        'true',
      );
    });

    const updatedChips = Array.from(rail.querySelectorAll<HTMLButtonElement>('.harness-chip'));
    expect(updatedChips.slice(0, 5).map((el) => el.textContent?.trim())).toEqual([
      'Windsurf',
      'Claude Code',
      'Codex',
      'OpenCode',
      'Cursor',
    ]);
    expect(screen.getByRole('button', { name: '1 selected' })).toBeInTheDocument();
    expect(within(dialog).getByText('1 selected')).toBeInTheDocument();

    // Empty search state + clear search button
    await fireEvent.input(searchInput, { target: { value: 'zzz-nonexistent' } });
    expect(within(dialog).getByText('No matching harnesses')).toBeInTheDocument();
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Clear harness search' }));
    expect((searchInput as HTMLInputElement).value).toBe('');

    // Escape closes popover and restores focus to trigger
    await fireEvent.keyDown(searchInput, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Filter harnesses' })).not.toBeInTheDocument();
    expect(document.activeElement).toBe(trigger);

    // Inline Clear resets active filters via onClearClients
    const inlineClear = screen.getByRole('button', { name: 'Clear harness filters' });
    await fireEvent.click(inlineClear);
    expect(onClearClients).toHaveBeenCalledTimes(1);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'All 62' })).toBeInTheDocument();
    });

    // Reopen, select two items, and verify popover "Clear all" + outside pointerdown dismissal
    await fireEvent.click(screen.getByRole('button', { name: 'All 62' }));
    const reopenedDialog = screen.getByRole('dialog', { name: 'Filter harnesses' });
    await fireEvent.click(within(reopenedDialog).getByRole('checkbox', { name: 'Claude Code' }));
    await fireEvent.click(within(reopenedDialog).getByRole('checkbox', { name: 'Codex' }));
    await waitFor(() => {
      expect(within(reopenedDialog).getByText('2 selected')).toBeInTheDocument();
    });
    await fireEvent.click(within(reopenedDialog).getByRole('button', { name: 'Clear all' }));
    expect(onClearClients).toHaveBeenCalledTimes(2);

    await fireEvent.pointerDown(document.body);
    expect(screen.queryByRole('dialog', { name: 'Filter harnesses' })).not.toBeInTheDocument();
  });

  it('supports overflow scroll nudge buttons, vertical wheel scrolling, and fallback clear without onClearClients', async () => {
    const sources = make62Sources();
    const onToggleClient = vi.fn();
    const query: DashboardQuery = {
      range: '30d',
      groupBy: 'model',
      activeClients: ['claude', 'codex'],
      sort: 'cost',
    };

    const { container } = render(Toolbar, {
      props: {
        query,
        sources,
        customRangeError: null,
        onRangeChange: vi.fn(),
        onCustomDatesChange: vi.fn(),
        onGroupByChange: vi.fn(),
        onSortChange: vi.fn(),
        onToggleClient,
      },
    });

    const rail = container.querySelector('.toolbar-harnesses-scroll') as HTMLDivElement;
    Object.defineProperty(rail, 'clientWidth', { value: 300, configurable: true });
    Object.defineProperty(rail, 'scrollWidth', { value: 1200, configurable: true });
    rail.scrollLeft = 0;
    await fireEvent.scroll(rail);

    const nudgeRight = await screen.findByRole('button', { name: 'Scroll harnesses right' });
    expect(screen.queryByRole('button', { name: 'Scroll harnesses left' })).not.toBeInTheDocument();

    await fireEvent.click(nudgeRight);
    await fireEvent.wheel(rail, { deltaY: 120, deltaX: 0 });
    expect(rail.scrollLeft).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: 'Scroll harnesses left' })).toBeInTheDocument();

    // Fallback clear calls onToggleClient for each active client when onClearClients is omitted
    await fireEvent.click(screen.getByRole('button', { name: 'Clear harness filters' }));
    expect(onToggleClient).toHaveBeenCalledTimes(2);
    expect(onToggleClient).toHaveBeenNthCalledWith(1, 'claude');
    expect(onToggleClient).toHaveBeenNthCalledWith(2, 'codex');
  });

  it('filters case-insensitively by client ID, renders message counts, and toggles closed when clicking trigger while open', async () => {
    const sources: SourceHealthRow[] = [
      {
        client: 'amp-cli',
        label: 'Sourcegraph Amp',
        sessionsPath: '~/.amp/sessions',
        sessionsPathExists: true,
        messageCount: 1200,
        headlessSupported: true,
        headlessMessageCount: 34,
        status: 'ok',
        credentialed: false,
        apiBacked: false,
      },
      {
        client: 'goose-ai',
        label: 'Goose',
        sessionsPath: '~/.goose/sessions',
        sessionsPathExists: false,
        messageCount: 0,
        headlessSupported: false,
        headlessMessageCount: 0,
        status: 'warn',
        credentialed: false,
        apiBacked: false,
      },
    ];

    render(Toolbar, {
      props: {
        query: { range: '30d', groupBy: 'model', activeClients: [], sort: 'cost' },
        sources,
        customRangeError: null,
        onRangeChange: vi.fn(),
        onCustomDatesChange: vi.fn(),
        onGroupByChange: vi.fn(),
        onSortChange: vi.fn(),
        onToggleClient: vi.fn(),
      },
    });

    const trigger = screen.getByRole('button', { name: 'All 2' });
    await fireEvent.click(trigger);

    const dialog = screen.getByRole('dialog', { name: 'Filter harnesses' });
    expect(within(dialog).getByText('1,234')).toBeInTheDocument();
    expect(within(dialog).getByText('—')).toBeInTheDocument();

    const searchInput = within(dialog).getByRole('searchbox', { name: 'Search harnesses' });
    await fireEvent.input(searchInput, { target: { value: 'AMP-CLI' } });
    expect(within(dialog).getByRole('checkbox', { name: 'Sourcegraph Amp' })).toBeInTheDocument();
    expect(within(dialog).queryByRole('checkbox', { name: 'Goose' })).not.toBeInTheDocument();

    // Clicking trigger while open (with pointerdown + click) closes without reopening
    await fireEvent.pointerDown(trigger);
    await fireEvent.click(trigger);
    expect(screen.queryByRole('dialog', { name: 'Filter harnesses' })).not.toBeInTheDocument();
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
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
    const onOpenSettings = vi.fn();
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
        onOpenSettings,
        onRefresh,
      },
    });

    const clearBtn = screen.getByRole('button', { name: 'Clear search' });
    await fireEvent.click(clearBtn);
    expect(onClearSearch).toHaveBeenCalledTimes(1);

    const sourcesBtn = screen.getByRole('button', { name: /Sources 7 \/ 10/i });
    await fireEvent.click(sourcesBtn);
    expect(onOpenSources).toHaveBeenCalledTimes(1);

    const settingsBtn = screen.getByRole('button', { name: 'Settings' });
    await fireEvent.click(settingsBtn);
    expect(onOpenSettings).toHaveBeenCalledTimes(1);

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

    render(StatusBar, {
      props: {
        lastSyncedAt: Math.floor(Date.now() / 1000) - 180,
        syncing: false,
      },
    });
    expect(screen.getByText('Last sync 3m ago')).toBeInTheDocument();

    render(StatusBar, {
      props: {
        lastSyncedAt: 0,
        syncing: false,
      },
    });
    expect(screen.getByText('Not synced yet')).toBeInTheDocument();
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

  it('isolates dropdown search typing and Escape from global dashboard shortcuts', async () => {
    render(DashboardApp);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Daily cost by harness' })).toBeInTheDocument();
    });

    const globalSearch = screen.getByRole('searchbox', {
      name: 'Search models, sessions, projects',
    }) as HTMLInputElement;
    await fireEvent.input(globalSearch, { target: { value: 'sonnet' } });
    expect(globalSearch.value).toBe('sonnet');

    const sortSelect = screen.getByRole('combobox', { name: 'Sort by' }) as HTMLSelectElement;
    expect(sortSelect.value).toBe('cost');

    const trigger = screen.getByRole('button', { name: 'All 10' });
    await fireEvent.click(trigger);

    const dialog = screen.getByRole('dialog', { name: 'Filter harnesses' });
    const dropdownSearch = within(dialog).getByRole('searchbox', { name: 'Search harnesses' });
    dropdownSearch.focus();

    // Typing shortcut keys inside dropdown search must not change sort, refresh, or steal focus
    for (const key of ['r', 'd', 't', 'c', '/']) {
      await fireEvent.keyDown(dropdownSearch, { key });
    }
    expect(sortSelect.value).toBe('cost');
    expect(document.activeElement).toBe(dropdownSearch);
    expect(screen.queryByRole('button', { name: /Syncing/i })).not.toBeInTheDocument();

    // Escape from dropdown search closes popover without clearing global AppBar search
    await fireEvent.keyDown(dropdownSearch, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Filter harnesses' })).not.toBeInTheDocument();
    expect(globalSearch.value).toBe('sonnet');
    expect(document.activeElement).toBe(trigger);

    // Reopen and focus a checkbox inside the popover; Escape still preserves global AppBar search
    await fireEvent.click(trigger);
    const reopened = screen.getByRole('dialog', { name: 'Filter harnesses' });
    const checkbox = within(reopened).getByRole('checkbox', { name: 'Claude Code' });
    checkbox.focus();
    await fireEvent.keyDown(checkbox, { key: 'Escape' });

    expect(screen.queryByRole('dialog', { name: 'Filter harnesses' })).not.toBeInTheDocument();
    expect(globalSearch.value).toBe('sonnet');
    expect(document.activeElement).toBe(trigger);
  });
});
