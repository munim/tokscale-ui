import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { DashboardQuery, SessionsPayload } from '$lib/contracts/ipc';
import { mockDashboardSnapshot, mockSessionsPayload } from '$lib/contracts/mocks';
import ProjectsView from '../ProjectsView.svelte';
import * as api from './api';

const baseQuery: DashboardQuery = {
  range: '30d',
  groupBy: 'model',
  activeClients: [],
  sort: 'cost',
};

describe('ProjectsView', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders 6 columns in exact order and emits navigate to sessions on row click', async () => {
    const payload = mockSessionsPayload({ query: baseQuery, enrichTitles: false });
    vi.spyOn(api, 'getSessionsView').mockResolvedValue(payload);
    const onnavigate = vi.fn();

    const { container } = render(ProjectsView, {
      props: {
        snapshot: mockDashboardSnapshot(baseQuery),
        query: baseQuery,
        onnavigate,
      },
    });

    await waitFor(() => {
      expect(container.querySelectorAll('thead th')).toHaveLength(6);
    });

    const headers = Array.from(container.querySelectorAll('thead th')).map((th) =>
      th.textContent?.replace(/[▲▼↕]/g, '').trim(),
    );
    expect(headers).toEqual(['Project', 'Sessions', 'Tokens', 'Cost', 'Models', 'Last active']);

    const topProject = [...payload.projects].sort((a, b) => b.totalCost - a.totalCost)[0];
    const firstRow = container.querySelector('tbody tr[data-row-key]') as HTMLTableRowElement;
    expect(firstRow).toBeTruthy();

    await fireEvent.click(firstRow);
    expect(onnavigate).toHaveBeenCalledWith({
      tab: 'sessions',
      query: topProject.workspaceLabel,
    });
  });

  it('renders (no workspace) and em-dash fallbacks, and shows empty state when empty', async () => {
    const customPayload: SessionsPayload = {
      sessions: [],
      projects: [
        {
          workspaceKey: null,
          workspaceLabel: '',
          sessionCount: 2,
          totalTokens: 12000,
          totalCost: 1.5,
          models: ['claude-sonnet-4-5'],
          lastActive: null,
        },
      ],
    };
    const spy = vi.spyOn(api, 'getSessionsView').mockResolvedValueOnce(customPayload);

    render(ProjectsView, {
      props: {
        snapshot: mockDashboardSnapshot(baseQuery),
        query: baseQuery,
        onnavigate: vi.fn(),
      },
    });

    expect(await screen.findByText('(no workspace)')).toBeInTheDocument();
    expect(screen.getByText('—')).toBeInTheDocument();

    cleanup();
    spy.mockResolvedValueOnce({ sessions: [], projects: [] });

    render(ProjectsView, {
      props: {
        snapshot: mockDashboardSnapshot(baseQuery),
        query: baseQuery,
        onnavigate: vi.fn(),
      },
    });

    expect(await screen.findByText('No projects in this range')).toBeInTheDocument();
  });
});
