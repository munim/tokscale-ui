import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  DashboardQuery,
  SessionTitlesUpdatedEvent,
  SessionsPayload,
} from '$lib/contracts/ipc';
import { mockDashboardSnapshot, mockSessionDetail, mockSessionsPayload } from '$lib/contracts/mocks';
import SessionsView from '../SessionsView.svelte';
import * as api from './api';

const baseQuery: DashboardQuery = {
  range: '30d',
  groupBy: 'model',
  activeClients: [],
  sort: 'cost',
};

describe('SessionsView', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('renders 9 columns in exact order and displays workspace · #shortId before titles', async () => {
    const basePayload = mockSessionsPayload({ query: baseQuery, enrichTitles: false });
    const firstRow = {
      ...basePayload.sessions[0],
      workspaceLabel: 'toksacleui',
      shortId: 'a1b2c0',
      title: undefined,
      displayLabel: 'toksacleui · #a1b2c0',
    };
    const payload: SessionsPayload = {
      sessions: [firstRow, ...basePayload.sessions.slice(1)],
      projects: basePayload.projects,
    };

    vi.spyOn(api, 'getPreferences').mockResolvedValue({
      menubarDisplay: 'cost',
      theme: 'system',
      launchAtLogin: false,
      notificationsEnabled: false,
      quotaAlertPercent: 85,
      sessionTitleEnrichment: true,
      autoCheckUpdates: true,
    });
    vi.spyOn(api, 'getSessionsView').mockResolvedValue(payload);
    vi.spyOn(api, 'onSessionTitlesUpdated').mockResolvedValue(() => {});

    const { container } = render(SessionsView, {
      props: {
        snapshot: mockDashboardSnapshot(baseQuery),
        query: baseQuery,
        onnavigate: vi.fn(),
      },
    });

    await waitFor(() => {
      expect(container.querySelectorAll('thead th')).toHaveLength(9);
    });

    const headers = Array.from(container.querySelectorAll('thead th')).map((th) =>
      th.textContent?.replace(/[▲▼↕]/g, '').trim(),
    );
    expect(headers).toEqual([
      'Session',
      'Title',
      'Harness',
      'Models',
      'Messages',
      'Duration',
      'Input',
      'Cache read',
      'Cost',
    ]);

    const firstTr = container.querySelector(`tbody tr[data-row-key="${firstRow.sessionId}"]`);
    expect(firstTr).not.toBeNull();
    const cells = firstTr!.querySelectorAll('td');
    expect(cells[0]).toHaveTextContent('toksacleui · #a1b2c0');
    expect(cells[1]).toHaveTextContent('toksacleui · #a1b2c0');
  });

  it('re-fetches and shows Titles updated (N) for 4s on tokscale://session-titles-updated', async () => {
    vi.useFakeTimers();

    const unenriched = mockSessionsPayload({ query: baseQuery, enrichTitles: false });
    const enriched = mockSessionsPayload({ query: baseQuery, enrichTitles: true });

    const getSessionsSpy = vi
      .spyOn(api, 'getSessionsView')
      .mockResolvedValueOnce(unenriched)
      .mockResolvedValueOnce(enriched);
    vi.spyOn(api, 'getPreferences').mockResolvedValue({
      menubarDisplay: 'cost',
      theme: 'system',
      launchAtLogin: false,
      notificationsEnabled: false,
      quotaAlertPercent: 85,
      sessionTitleEnrichment: true,
      autoCheckUpdates: true,
    });

    let titleHandler: ((payload: SessionTitlesUpdatedEvent) => void) | null = null;
    vi.spyOn(api, 'onSessionTitlesUpdated').mockImplementation(async (handler) => {
      titleHandler = handler;
      return () => {};
    });

    render(SessionsView, {
      props: {
        snapshot: mockDashboardSnapshot(baseQuery),
        query: baseQuery,
        onnavigate: vi.fn(),
      },
    });

    await waitFor(() => {
      expect(getSessionsSpy).toHaveBeenCalledTimes(1);
    });
    expect(titleHandler).not.toBeNull();

    titleHandler!({ updatedCount: 5 });

    await waitFor(() => {
      expect(getSessionsSpy).toHaveBeenCalledTimes(2);
    });
    expect(screen.getByText('Titles updated (5)')).toBeInTheDocument();

    await vi.advanceTimersByTimeAsync(4000);
    await waitFor(() => {
      expect(screen.queryByText('Titles updated (5)')).not.toBeInTheDocument();
    });
  });

  it('toggles Enrich titles locally, persists preference, and re-requests sessions', async () => {
    const getSessionsSpy = vi
      .spyOn(api, 'getSessionsView')
      .mockResolvedValue(mockSessionsPayload({ query: baseQuery, enrichTitles: true }));
    vi.spyOn(api, 'getPreferences').mockResolvedValue({
      menubarDisplay: 'cost',
      theme: 'system',
      launchAtLogin: false,
      notificationsEnabled: false,
      quotaAlertPercent: 85,
      sessionTitleEnrichment: true,
      autoCheckUpdates: true,
    });
    const updatePrefsSpy = vi.spyOn(api, 'updatePreferences').mockResolvedValue({
      menubarDisplay: 'cost',
      theme: 'system',
      launchAtLogin: false,
      notificationsEnabled: false,
      quotaAlertPercent: 85,
      sessionTitleEnrichment: false,
      autoCheckUpdates: true,
    });
    vi.spyOn(api, 'onSessionTitlesUpdated').mockResolvedValue(() => {});

    render(SessionsView, {
      props: {
        snapshot: mockDashboardSnapshot(baseQuery),
        query: baseQuery,
        onnavigate: vi.fn(),
      },
    });

    const checkbox = await screen.findByRole('checkbox', { name: /Enrich titles locally/i });
    expect(checkbox).toBeChecked();
    expect(checkbox).toHaveAttribute(
      'title',
      "Uses tokscale's free local summarizer. No cloud LLM is called.",
    );

    await waitFor(() => {
      expect(getSessionsSpy).toHaveBeenCalledTimes(1);
    });

    await fireEvent.click(checkbox);

    await waitFor(() => {
      expect(updatePrefsSpy).toHaveBeenCalledWith({ sessionTitleEnrichment: false });
      expect(getSessionsSpy).toHaveBeenLastCalledWith({
        query: expect.objectContaining({ range: '30d' }),
        enrichTitles: false,
      });
    });
  });

  it('shows empty state and opens SessionDrawer when clicking a row', async () => {
    vi.spyOn(api, 'getPreferences').mockResolvedValue({
      menubarDisplay: 'cost',
      theme: 'system',
      launchAtLogin: false,
      notificationsEnabled: false,
      quotaAlertPercent: 85,
      sessionTitleEnrichment: true,
      autoCheckUpdates: true,
    });
    vi.spyOn(api, 'onSessionTitlesUpdated').mockResolvedValue(() => {});

    const emptySpy = vi
      .spyOn(api, 'getSessionsView')
      .mockResolvedValueOnce({ sessions: [], projects: [] });

    render(SessionsView, {
      props: {
        snapshot: mockDashboardSnapshot(baseQuery),
        query: baseQuery,
        onnavigate: vi.fn(),
      },
    });

    expect(await screen.findByText('No sessions in this range')).toBeInTheDocument();
    cleanup();
    emptySpy.mockRestore();

    const payload = mockSessionsPayload({ query: baseQuery, enrichTitles: true });
    vi.spyOn(api, 'getSessionsView').mockResolvedValue(payload);
    vi.spyOn(api, 'getSessionDetail').mockImplementation(async (client, id) =>
      mockSessionDetail(client, id),
    );

    const { container } = render(SessionsView, {
      props: {
        snapshot: mockDashboardSnapshot(baseQuery),
        query: baseQuery,
        onnavigate: vi.fn(),
      },
    });

    const firstRow = await waitFor(() => {
      const el = container.querySelector('tbody tr[data-row-key]') as HTMLTableRowElement | null;
      expect(el).not.toBeNull();
      return el!;
    });

    firstRow.focus();
    await fireEvent.click(firstRow);

    const dialog = await screen.findByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');

    await fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
    expect(document.activeElement).toBe(firstRow);
  });
});
