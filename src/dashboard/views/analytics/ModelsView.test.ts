import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { DashboardQuery, DashboardSnapshot, ModelViewRow } from '$lib/contracts/ipc';
import { mockDashboardSnapshot } from '$lib/contracts/mocks';
import ModelsView from '../ModelsView.svelte';

function makeQuery(overrides: Partial<DashboardQuery> = {}): DashboardQuery {
  return {
    range: 'all',
    groupBy: 'model',
    activeClients: [],
    sort: 'cost',
    ...overrides,
  };
}

describe('ModelsView', () => {
  it('renders all 13 columns in exact order and displays msPer1KTokens tooltip', () => {
    const query = makeQuery();
    const snapshot = mockDashboardSnapshot(query);
    const { container } = render(ModelsView, {
      props: { snapshot, query, onnavigate: vi.fn() },
    });

    const headers = Array.from(container.querySelectorAll('thead th')).map((th) =>
      th.textContent?.replace(/[▲▼↕]/g, '').trim(),
    );
    expect(headers).toEqual([
      'Model',
      'Harness',
      'Provider',
      'Workspace',
      'Session',
      'Sessions',
      'Messages',
      'Input',
      'Output',
      'Cache read',
      'Cache write',
      'Reasoning',
      'Cost',
    ]);

    const firstRowModel = snapshot.models[0];
    const modelText = screen.getAllByText(firstRowModel.model)[0];
    expect(modelText).toHaveAttribute(
      'title',
      expect.stringContaining(`${firstRowModel.msPer1KTokens} ms / 1K tokens`),
    );

    const sessionCells = Array.from(
      container.querySelectorAll('tbody tr[data-row-key] td:nth-child(5)'),
    );
    expect(sessionCells.every((td) => td.textContent?.trim() === '—')).toBe(true);
  });

  it('shows collapsible group header rows in client,model mode', async () => {
    const query = makeQuery({ groupBy: 'client,model' });
    const snapshot = mockDashboardSnapshot(query);
    const { container } = render(ModelsView, {
      props: { snapshot, query, onnavigate: vi.fn() },
    });

    const groupRows = container.querySelectorAll('tr.ui-table-group');
    expect(groupRows.length).toBeGreaterThan(1);

    const claudeGroupButton = screen.getByRole('button', { name: /Claude Code/ });
    expect(claudeGroupButton).toHaveAttribute('aria-expanded', 'true');
    expect(screen.queryAllByText('claude-opus-4-1').length).toBeGreaterThan(0);

    await fireEvent.click(claudeGroupButton);

    const collapsedButton = screen.getByRole('button', { name: /Claude Code/ });
    expect(collapsedButton).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('claude-opus-4-1')).not.toBeInTheDocument();

    await fireEvent.click(collapsedButton);
    expect(screen.queryAllByText('claude-opus-4-1').length).toBeGreaterThan(0);
  });

  it('shows session shortId in client,session,model mode and workspace groups in workspace,model mode', () => {
    const sessionRow: ModelViewRow = {
      model: 'claude-sonnet-4-5',
      colorKey: 'claude',
      provider: 'anthropic',
      client: 'claude',
      workspaceKey: '/repo/alpha',
      workspaceLabel: 'alpha',
      sessionId: 'sess-claude-99aabb',
      tokens: { input: 1000, output: 500, cacheRead: 2000, cacheWrite: 100, reasoning: 50 },
      cost: 2.5,
      sessionCount: 1,
      messageCount: 12,
      groupKey: 'claude:sess-claude-99aabb:claude-sonnet-4-5',
    };
    const base = mockDashboardSnapshot(makeQuery({ groupBy: 'client,session,model' }));
    const snapshot: DashboardSnapshot = {
      ...base,
      models: [sessionRow],
    };

    const { container, unmount } = render(ModelsView, {
      props: {
        snapshot,
        query: makeQuery({ groupBy: 'client,session,model' }),
        onnavigate: vi.fn(),
      },
    });

    expect(container.querySelector('tr.ui-table-group')).toHaveTextContent('99aabb');
    const sessionCell = container.querySelector('tbody tr[data-row-key^="model::"] td:nth-child(5)');
    expect(sessionCell).toHaveTextContent('99aabb');
    unmount();

    const wsQuery = makeQuery({ groupBy: 'workspace,model' });
    const wsRender = render(ModelsView, {
      props: { snapshot, query: wsQuery, onnavigate: vi.fn() },
    });
    expect(wsRender.container.querySelector('tr.ui-table-group')).toHaveTextContent('alpha');
  });

  it('renders EmptyState when no models exist', () => {
    const query = makeQuery();
    const snapshot: DashboardSnapshot = { ...mockDashboardSnapshot(query), models: [] };
    render(ModelsView, { props: { snapshot, query, onnavigate: vi.fn() } });
    expect(screen.getByText('No models in this range')).toBeInTheDocument();
  });
});
