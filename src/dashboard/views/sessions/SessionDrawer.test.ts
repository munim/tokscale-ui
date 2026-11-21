import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { SessionDetail, SessionRow } from '$lib/contracts/ipc';
import { mockSessionDetail, mockSessionsPayload } from '$lib/contracts/mocks';
import SessionDrawer from '../../components/SessionDrawer.svelte';
import * as api from './api';

describe('SessionDrawer', () => {
  const payload = mockSessionsPayload({
    query: { range: '30d', groupBy: 'model', activeClients: [] },
    enrichTitles: true,
  });
  const sessions: SessionRow[] = payload.sessions.slice(0, 3);

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('renders modal dialog with 3 sections, skeleton loading, and null fallbacks', async () => {
    let resolveDetail!: (value: SessionDetail) => void;
    const pendingDetail = new Promise<SessionDetail>((resolve) => {
      resolveDetail = resolve;
    });
    vi.spyOn(api, 'getSessionDetail').mockReturnValueOnce(pendingDetail);

    const { container } = render(SessionDrawer, {
      props: {
        session: sessions[0],
        sessions,
        onselect: vi.fn(),
        onclose: vi.fn(),
      },
    });

    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    expect(dialog).toHaveAttribute('aria-labelledby', 'drawer-title');
    expect(container.querySelectorAll('.skeleton-bar')).toHaveLength(3);

    const detail = mockSessionDetail(sessions[0].client, sessions[0].sessionId);
    resolveDetail({
      ...detail,
      complexity: undefined,
      taskCategory: undefined,
      description: undefined,
      session: {
        ...detail.session,
        workspace: undefined,
        createdAt: undefined,
      },
    });

    await waitFor(() => {
      expect(screen.getByText('Session overview')).toBeInTheDocument();
    });

    const headings = Array.from(container.querySelectorAll('.drawer-section-title')).map((el) =>
      el.textContent?.trim(),
    );
    expect(headings).toEqual(['Session overview', 'Activity', 'Tokens & cost']);

    // Null/absent values display em-dash
    expect(screen.getAllByText('—').length).toBeGreaterThanOrEqual(4);

    // Per-model breakdown table has the 7 required columns
    const modelHeaders = Array.from(container.querySelectorAll('thead th')).map((th) =>
      th.textContent?.trim(),
    );
    expect(modelHeaders).toEqual([
      'Model',
      'Input',
      'Output',
      'Cache read',
      'Cache write',
      'Reasoning',
      'Cost',
    ]);
  });

  it('closes on Escape and restores focus to opener, and closes on scrim click', async () => {
    vi.spyOn(api, 'getSessionDetail').mockImplementation(async (c, id) =>
      mockSessionDetail(c, id),
    );

    const opener = document.createElement('button');
    opener.textContent = 'Open row';
    document.body.appendChild(opener);
    opener.focus();

    const onclose = vi.fn();
    render(SessionDrawer, {
      props: {
        session: sessions[0],
        sessions,
        onselect: vi.fn(),
        onclose,
        returnFocusTo: opener,
      },
    });

    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });

    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(onclose).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(opener);

    await fireEvent.click(screen.getByTestId('drawer-scrim'));
    expect(onclose).toHaveBeenCalledTimes(2);

    opener.remove();
  });

  it('traps Tab focus inside the drawer and navigates Prev/Next with ArrowLeft/ArrowRight', async () => {
    vi.spyOn(api, 'getSessionDetail').mockImplementation(async (c, id) =>
      mockSessionDetail(c, id),
    );
    const onselect = vi.fn();

    render(SessionDrawer, {
      props: {
        session: sessions[1],
        sessions,
        onselect,
        onclose: vi.fn(),
      },
    });

    await waitFor(() => {
      expect(screen.getByText('Session overview')).toBeInTheDocument();
    });

    await fireEvent.keyDown(window, { key: 'ArrowLeft' });
    expect(onselect).toHaveBeenLastCalledWith(sessions[0]);

    await fireEvent.keyDown(window, { key: 'ArrowRight' });
    expect(onselect).toHaveBeenLastCalledWith(sessions[2]);

    const prevBtn = screen.getByRole('button', { name: /Prev/ });
    const copyBtn = screen.getByRole('button', { name: 'Copy session ID' });

    copyBtn.focus();
    await fireEvent.keyDown(window, { key: 'Tab' });
    expect(document.activeElement).toBe(prevBtn);

    prevBtn.focus();
    await fireEvent.keyDown(window, { key: 'Tab', shiftKey: true });
    expect(document.activeElement).toBe(copyBtn);
  });

  it('copies session ID to clipboard and shows Copied for 2s', async () => {
    vi.useFakeTimers();
    vi.spyOn(api, 'getSessionDetail').mockImplementation(async (c, id) =>
      mockSessionDetail(c, id),
    );

    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText },
      configurable: true,
    });

    render(SessionDrawer, {
      props: {
        session: sessions[0],
        sessions,
        onselect: vi.fn(),
        onclose: vi.fn(),
      },
    });

    const copyBtn = screen.getByRole('button', { name: 'Copy session ID' });
    await fireEvent.click(copyBtn);

    expect(writeText).toHaveBeenCalledWith(sessions[0].sessionId);
    expect(screen.getByRole('button', { name: 'Copied' })).toBeInTheDocument();

    await vi.advanceTimersByTimeAsync(2000);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Copy session ID' })).toBeInTheDocument();
    });
  });
});
