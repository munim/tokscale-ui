<script lang="ts">
  import { onMount } from 'svelte';
  import '../styles/tokens.css';
  import '$lib/ui/ui.css';
  import './tray.css';
  import { EmptyState } from '$lib/ui';
  import FooterActions from './components/FooterActions.svelte';
  import HarnessChart from './components/HarnessChart.svelte';
  import KpiRow from './components/KpiRow.svelte';
  import QuickSettings from './components/QuickSettings.svelte';
  import RangeSwitch from './components/RangeSwitch.svelte';
  import StateBanner from './components/StateBanner.svelte';
  import TopModels from './components/TopModels.svelte';
  import TrayHeader from './components/TrayHeader.svelte';
  import { createTrayStore } from './store.svelte';

  const store = createTrayStore();
  let firstActionEl = $state<HTMLButtonElement | null>(null);

  onMount(() => {
    const cleanup = store.init();
    firstActionEl?.focus();
    return cleanup;
  });

  function handleWindowKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      store.dismiss();
    }
  }

  function handleBannerRetry(): void {
    if (store.warnings.length === 1) {
      void store.triggerSync({ retryClient: store.warnings[0].source });
      return;
    }
    void store.triggerSync('full_manual');
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<main class="tray-shell ui-root" data-view="tray">
  <TrayHeader
    syncing={store.isSyncing}
    settingsOpen={store.settingsOpen}
    onsync={() => void store.triggerSync('full_manual')}
    ontogglesettings={store.toggleSettings}
    bind:firstActionEl
  />

  <StateBanner
    syncing={store.isSyncing}
    syncProgress={store.syncProgress}
    partial={store.isPartial}
    warnings={store.warnings}
    error={store.error}
    onretry={handleBannerRetry}
  />

  <div class="tray-body">
    {#if store.settingsOpen}
      <QuickSettings
        sources={store.snapshot?.sources ?? []}
        preferences={store.preferences}
        cliInfo={store.cliInfo ?? store.snapshot?.cliInfo ?? null}
        cliInstalling={store.cliInstalling}
        cliInstallResult={store.cliInstallResult}
        onretry={(client) => void store.triggerSync({ retryClient: client })}
        onupdatepreferences={store.updatePreferences}
        oninstallcli={() => void store.installOrUpgradeCli()}
      />
    {:else}
      <RangeSwitch value={store.activeRange} onchange={store.setRange} />

      {#if store.snapshot}
        <KpiRow
          cost={store.snapshot.total.cost}
          tokens={store.snapshot.total.tokens}
          sessions={store.snapshot.total.sessions}
        />

        {#if store.isEmpty}
          <EmptyState
            title="No usage in this range"
            body="Switch to a wider range or scan your local sources to see usage here."
            actionLabel={store.activeRange === 'all' ? 'Scan sources' : 'View all time'}
            onaction={() => {
              if (store.activeRange === 'all') {
                void store.triggerSync('full_manual');
              } else {
                store.setRange('all');
              }
            }}
          />
        {:else}
          <HarnessChart bars={store.snapshot.bars} harnesses={store.snapshot.harnesses} />
          <TopModels models={store.snapshot.topModels} onselect={store.openModelInDashboard} />
        {/if}
      {:else if store.loading}
        <div class="tray-loading" role="status">Loading usage…</div>
      {/if}
    {/if}
  </div>

  <FooterActions
    syncing={store.isSyncing}
    lastSyncedAt={store.snapshot?.syncStatus.lastSyncedAt}
    hasWarnings={store.isPartial}
    onopen={store.openDashboard}
    onquit={store.quit}
  />
</main>

<style>
  .tray-loading {
    padding: 36px 12px;
    text-align: center;
    font-size: 12.5px;
    color: var(--muted);
  }
</style>
