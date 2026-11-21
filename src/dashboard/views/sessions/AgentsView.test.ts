import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { DashboardQuery, SyncStatus } from '$lib/contracts/ipc';
import { mockDashboardSnapshot } from '$lib/contracts/mocks';
import AgentsView from '../AgentsView.svelte';
import * as api from './api';

const baseQuery: DashboardQuery = {
  range: '30d',
  groupBy: 'model',
  activeClients: [],
  sort: 'cost',
};

describe('AgentsView', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders both panels, headless roots/note, and shows Sync button only for credentialed API providers', async () => {
    const snapshot = mockDashboardSnapshot(baseQuery);

    const { container } = render(AgentsView, {
      props: {
        snapshot,
        query: baseQuery,
        onnavigate: vi.fn(),
      },
    });

    expect(screen.getByRole('heading', { name: 'Sub-agent personas' })).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: 'Local harness sources and scan paths' }),
    ).toBeInTheDocument();

    const noteBlock = container.querySelector('.sources-note-block') as HTMLElement;
    expect(within(noteBlock).getByText(snapshot.sources.note)).toBeInTheDocument();
    for (const root of snapshot.sources.headlessRoots) {
      expect(within(noteBlock).getByText(root)).toBeInTheDocument();
    }

    const agentsTable = screen.getByRole('table', { name: 'Sub-agent personas' });
    const agentHeaders = Array.from(agentsTable.querySelectorAll('thead th')).map((th) =>
      th.textContent?.replace(/[▲▼↕]/g, '').trim(),
    );
    expect(agentHeaders).toEqual(['Agent', 'Harnesses', 'Messages', 'Input', 'Output', 'Cost']);

    const sourcesTable = screen.getByRole('table', {
      name: 'Local harness sources and scan paths',
    });

    // Only credentialed API-backed providers (capy, cursor) get a Sync button
    const syncButtons = within(sourcesTable).getAllByRole('button', { name: 'Sync' });
    expect(syncButtons).toHaveLength(2);

    // Non-credentialed API-backed providers (trae, warp, antigravity) show "Not signed in" pill
    expect(within(sourcesTable).getAllByText('Not signed in')).toHaveLength(3);

    // Missing paths show "Not found on this machine"
    expect(within(sourcesTable).getAllByText('Not found on this machine')).toHaveLength(3);
  });

  it('handles Sync lifecycle: Syncing… while running, then Synced or warning message', async () => {
    const snapshot = mockDashboardSnapshot(baseQuery);

    let resolveSync!: (status: SyncStatus) => void;
    const syncSpy = vi.spyOn(api, 'triggerSync').mockImplementation(
      () =>
        new Promise<SyncStatus>((resolve) => {
          resolveSync = resolve;
        }),
    );

    render(AgentsView, {
      props: {
        snapshot,
        query: baseQuery,
        onnavigate: vi.fn(),
      },
    });

    const sourcesTable = screen.getByRole('table', {
      name: 'Local harness sources and scan paths',
    });
    const cursorRow = sourcesTable.querySelector('tr[data-row-key="cursor"]') as HTMLTableRowElement;
    const capyRow = sourcesTable.querySelector('tr[data-row-key="capy"]') as HTMLTableRowElement;

    const cursorSyncBtn = within(cursorRow).getByRole('button', { name: 'Sync' });
    await fireEvent.click(cursorSyncBtn);

    expect(syncSpy).toHaveBeenCalledWith({ retryClient: 'cursor' });
    expect(within(cursorRow).getByRole('button', { name: 'Syncing…' })).toBeDisabled();

    resolveSync({
      syncing: false,
      lastSyncedAt: 1795262400000,
      warnings: [],
    });

    await waitFor(() => {
      expect(within(cursorRow).getByRole('button', { name: 'Synced' })).toBeEnabled();
    });

    // Trigger capy sync and return a warning
    syncSpy.mockResolvedValueOnce({
      syncing: false,
      lastSyncedAt: 1795262400000,
      warnings: [{ source: 'capy', message: 'Token expired' }],
    });

    const capySyncBtn = within(capyRow).getByRole('button', { name: 'Sync' });
    await fireEvent.click(capySyncBtn);

    await waitFor(() => {
      expect(within(capyRow).getByText('Token expired')).toBeInTheDocument();
    });
  });

  it('renders EmptyState when snapshot.agents is empty', () => {
    const snapshot = {
      ...mockDashboardSnapshot(baseQuery),
      agents: [],
    };

    render(AgentsView, {
      props: {
        snapshot,
        query: baseQuery,
        onnavigate: vi.fn(),
      },
    });

    expect(screen.getByText('No sub-agent activity')).toBeInTheDocument();
  });
});
