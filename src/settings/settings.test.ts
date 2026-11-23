import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  COMMANDS,
  EVENTS,
  type CompanionPreferences,
  type CustomPricingEntry,
  type TokscaleSettingsPatch,
  type UpdateStatusPayload,
} from '$lib/contracts/ipc';
import {
  mockCliInfo,
  mockCustomPricing,
  mockDashboardSnapshot,
  mockPreferences,
  mockProvidersStatus,
  mockSocialStatus,
  mockTokscaleSettings,
  mockUpdateStatus,
} from '$lib/contracts/mocks';

const { invokeMock, listenMock, eventHandlers, unlistenMocks } = vi.hoisted(() => {
  const eventHandlers = new Map<string, Set<(event: { payload: unknown }) => void>>();
  const unlistenMocks: Array<ReturnType<typeof vi.fn>> = [];
  const invokeMock = vi.fn();
  const listenMock = vi.fn(async (eventName: string, cb: (event: { payload: unknown }) => void) => {
    let listeners = eventHandlers.get(eventName);
    if (!listeners) {
      listeners = new Set();
      eventHandlers.set(eventName, listeners);
    }
    listeners.add(cb);
    const unlisten = vi.fn(() => {
      listeners?.delete(cb);
    });
    unlistenMocks.push(unlisten);
    return unlisten;
  });
  return { invokeMock, listenMock, eventHandlers, unlistenMocks };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

import SettingsApp from './SettingsApp.svelte';

function emitTauriEvent(eventName: string, payload: unknown): void {
  const listeners = eventHandlers.get(eventName);
  if (!listeners) return;
  for (const cb of listeners) {
    cb({ payload });
  }
}

describe('SettingsApp (Option B + Sidebar Search)', () => {
  let prefsState: CompanionPreferences;
  let tsSettings = mockTokscaleSettings();
  let customPricing = mockCustomPricing();
  let providers = mockProvidersStatus();
  let social = mockSocialStatus();

  beforeEach(() => {
    prefsState = mockPreferences();
    tsSettings = mockTokscaleSettings();
    customPricing = mockCustomPricing();
    providers = mockProvidersStatus();
    social = mockSocialStatus();
    eventHandlers.clear();
    unlistenMocks.length = 0;
    invokeMock.mockReset();
    listenMock.mockClear();
    delete document.documentElement.dataset.theme;
    document.documentElement.style.colorScheme = '';

    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      value: {},
      configurable: true,
    });

    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      switch (cmd) {
        case COMMANDS.takePendingSettingsSection:
          return null;
        case COMMANDS.getPreferences:
          return { ...prefsState };
        case COMMANDS.updatePreferences: {
          const patch = (args?.patch as Partial<CompanionPreferences>) ?? {};
          prefsState = { ...prefsState, ...patch };
          return { ...prefsState };
        }
        case COMMANDS.getCliInfo:
          return mockCliInfo();
        case COMMANDS.installOrUpgradeCli:
          return { ok: true, message: 'tokscale 4.24.0 installed via bun' };
        case COMMANDS.getSourcesHealth:
          return mockDashboardSnapshot().sources;
        case COMMANDS.getTokscaleSettings:
          return structuredClone(tsSettings);
        case COMMANDS.updateTokscaleSettings: {
          const patch = (args?.patch as TokscaleSettingsPatch) ?? {};
          tsSettings = {
            ...tsSettings,
            ...(patch.defaultClients ? { defaultClients: patch.defaultClients } : {}),
            ...(patch.extraScanPaths ? { extraScanPaths: patch.extraScanPaths } : {}),
            ...(patch.opencodeDbPaths ? { opencodeDbPaths: patch.opencodeDbPaths } : {}),
            ...(patch.includeUnusedModels !== undefined
              ? { includeUnusedModels: patch.includeUnusedModels }
              : {}),
            ...(patch.modelAliases ? { modelAliases: patch.modelAliases } : {}),
            ...(patch.nativeTimeoutMs ? { nativeTimeoutMs: patch.nativeTimeoutMs } : {}),
            ...(patch.colorPalette ? { colorPalette: patch.colorPalette } : {}),
          };
          return structuredClone(tsSettings);
        }
        case COMMANDS.getCustomPricing:
          return structuredClone(customPricing);
        case COMMANDS.saveCustomPricing: {
          const entries = (args?.entries as CustomPricingEntry[]) ?? [];
          customPricing = {
            ...customPricing,
            entries: structuredClone(entries),
            valid: true,
            validationMessage: 'Validated by tokscale CLI',
          };
          return structuredClone(customPricing);
        }
        case COMMANDS.lookupModelPricing:
          return {
            modelId: String(args?.modelId ?? ''),
            matchedKey: String(args?.modelId ?? ''),
            source: 'LiteLLM',
            scope: 'upstream',
            inputPerMillion: 3,
            outputPerMillion: 15,
            cacheReadPerMillion: 0.3,
            cacheWritePerMillion: 3.75,
            submissionSafe: true,
          };
        case COMMANDS.refreshPricingCatalog:
          return { ok: true, message: 'Pricing catalogs refreshed' };
        case COMMANDS.getProvidersStatus:
          return structuredClone(providers);
        case COMMANDS.runProviderAction:
          return { ok: true, message: 'Provider action completed' };
        case COMMANDS.getSocialStatus:
          return structuredClone(social);
        case COMMANDS.runSocialAction:
          return {
            ok: true,
            message: 'Dry run ready',
            output: 'Dry run summary: 0 excluded messages',
          };
        case COMMANDS.configureAutosubmit:
          return { ok: true, message: 'Autosubmit enabled' };
        case COMMANDS.triggerSync:
          return { syncing: false, lastSyncedAt: 1795262500000, warnings: [] };
        case COMMANDS.revealConfigPath:
          return undefined;
        case COMMANDS.pickPath:
          return '/Users/dev/custom/tokscale';
        case COMMANDS.getUpdateStatus:
          return mockUpdateStatus();
        case COMMANDS.checkForUpdates:
          return {
            ...mockUpdateStatus(),
            phase: 'available',
            availableVersion: '0.2.0',
            releaseNotes: '- Faster startup\n- Signed updater',
          } satisfies UpdateStatusPayload;
        case COMMANDS.downloadAndInstallUpdate:
          return {
            ...mockUpdateStatus(),
            phase: 'ready',
            availableVersion: '0.2.0',
          } satisfies UpdateStatusPayload;
        case COMMANDS.skipUpdateVersion:
          return {
            ...mockUpdateStatus(),
            phase: 'idle',
          } satisfies UpdateStatusPayload;
        case COMMANDS.restartToApplyUpdate:
          return undefined;
        default:
          return undefined;
      }
    });
  });

  afterEach(() => {
    cleanup();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it('renders App & Alerts live preview, saves preferences, records shortcut, and enables daily budget', async () => {
    render(SettingsApp);

    await screen.findByText('Menubar Live Preview');

    // Switch Menubar display -> updates preview text & persists
    const menubarGroup = screen.getByRole('radiogroup', { name: 'Menubar display' });
    await fireEvent.click(within(menubarGroup).getByRole('radio', { name: 'Tokens' }));
    expect(screen.getByLabelText('Menubar preview')).toHaveTextContent('14.2M');
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.updatePreferences, {
      patch: { menubarDisplay: 'tokens' },
    });

    // Switch Theme -> updates document dataset
    const themeGroup = screen.getByRole('radiogroup', { name: 'Theme' });
    await fireEvent.click(within(themeGroup).getByRole('radio', { name: 'Dark' }));
    expect(document.documentElement.dataset.theme).toBe('dark');

    // Interactive shortcut recorder
    const recorder = screen.getByRole('button', { name: 'Global shortcut' });
    await fireEvent.click(recorder);
    await fireEvent.keyDown(recorder, { key: 't', metaKey: true, shiftKey: true });
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.updatePreferences, {
      patch: { globalShortcut: 'CmdOrCtrl+Shift+T' },
    });

    // Enable daily budget USD
    const budgetSwitch = screen.getByRole('switch', { name: 'Enable daily spend budget' });
    await fireEvent.click(budgetSwitch);
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.updatePreferences, {
      patch: { dailyBudgetUsd: 25 },
    });
  });

  it('supports sidebar search to jump directly to matching sections and settings', async () => {
    render(SettingsApp);
    await screen.findByText('Menubar Live Preview');

    const searchInput = screen.getByRole('searchbox', { name: 'Search settings' });
    await fireEvent.input(searchInput, { target: { value: 'warp' } });

    const hit = await screen.findByRole('button', { name: /Warp \/ Oz integration/i });
    await fireEvent.click(hit);

    // Jumps straight to Providers split master-detail with Warp selected
    await screen.findByText('Warp / Oz Authentication');
    expect((searchInput as HTMLInputElement).value).toBe('');
  });

  it('drives Providers split master-detail, Pricing lookup + overrides, Social dry-run, and CLI upgrade', async () => {
    render(SettingsApp);
    await screen.findByText('Menubar Live Preview');

    // Navigate to Providers
    await fireEvent.click(screen.getByRole('button', { name: /^Providers/i }));
    await screen.findByText('Saved Cursor Accounts');

    // Switch to Warp and submit a token (verifying secret input clears after submit)
    await fireEvent.click(screen.getByRole('button', { name: /Warp \/ Oz/i }));
    const warpInput = await screen.findByLabelText('Warp token');
    await fireEvent.input(warpInput, { target: { value: 'secret_warp_tok' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save Token' }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(COMMANDS.runProviderAction, {
        request: expect.objectContaining({
          provider: 'warp',
          action: 'login',
          token: 'secret_warp_tok',
        }),
      });
    });
    expect((warpInput as HTMLInputElement).value).toBe('');

    // Navigate to Pricing & Models via warm settings-navigate event
    emitTauriEvent(EVENTS.settingsNavigate, { section: 'pricing' });
    await screen.findByText('Quick Model Price Lookup');

    await fireEvent.click(screen.getByRole('button', { name: 'Lookup' }));
    await screen.findByText('LiteLLM');

    await fireEvent.click(screen.getByRole('button', { name: '+ Override' }));
    await fireEvent.input(screen.getByLabelText('Override model ID'), {
      target: { value: 'custom/test-model' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save & Validate' }));
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.saveCustomPricing, {
      entries: expect.arrayContaining([
        expect.objectContaining({ modelId: 'custom/test-model' }),
      ]),
    });

    // Navigate to Social & Autosubmit
    await fireEvent.click(screen.getByRole('button', { name: /Social & Autosubmit/i }));
    await screen.findByText('Tokscale Social Account');
    await fireEvent.click(screen.getByRole('button', { name: 'Dry-Run Preview' }));
    await screen.findByText('Dry run summary: 0 excluded messages');

    // Navigate to CLI & Advanced
    await fireEvent.click(screen.getByRole('button', { name: /CLI & Advanced/i }));
    await screen.findByText('Tokscale CLI Binary');
    await fireEvent.click(screen.getByRole('button', { name: 'Install / Upgrade global tokscale' }));
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.installOrUpgradeCli);
  });

  it('supports App & Updates check, download, restart, and notifyOnly fallback in AppAlertsView', async () => {
    render(SettingsApp);
    await screen.findByText('App & Updates');

    // Toggle autoCheckUpdates
    const autoCheck = screen.getByRole('switch', { name: 'Check for updates automatically' });
    await fireEvent.click(autoCheck);
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.updatePreferences, {
      patch: { autoCheckUpdates: false },
    });

    // Manual update check reveals release notes & Download button
    await fireEvent.click(screen.getByRole('button', { name: 'Check for Updates' }));
    await screen.findByRole('button', { name: 'Download & Install v0.2.0' });
    expect(screen.getByText(/Faster startup/)).toBeInTheDocument();

    // Download & install transitions to Ready -> Restart button
    await fireEvent.click(screen.getByRole('button', { name: 'Download & Install v0.2.0' }));
    const restartBtn = await screen.findByRole('button', { name: 'Restart to Apply v0.2.0' });
    await fireEvent.click(restartBtn);
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.restartToApplyUpdate);

    // NotifyOnly install method renders fallback hint instead of Download button
    emitTauriEvent(EVENTS.updateStatus, {
      ...mockUpdateStatus(),
      phase: 'available',
      capability: 'notifyOnly',
      installMethod: 'linux-package',
      availableVersion: '0.3.0',
      fallbackHint: 'curl -fsSL https://raw.githubusercontent.com/munim/tokscale-ui/main/scripts/install.sh | sh',
    } satisfies UpdateStatusPayload);

    await screen.findByText(/install\.sh \| sh/);
    expect(screen.queryByRole('button', { name: 'Download & Install v0.3.0' })).not.toBeInTheDocument();
  });
});
