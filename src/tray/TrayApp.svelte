<script lang="ts">
  import { onMount } from 'svelte';
  import '../styles/tokens.css';
  import '$lib/ui/ui.css';
  import './tray.css';
  import { ipc } from '$lib/contracts/ipc';
  import { EmptyState } from '$lib/ui';
  import FooterActions from './components/FooterActions.svelte';
  import HarnessChart from './components/HarnessChart.svelte';
  import KpiRow from './components/KpiRow.svelte';
  import RangeSwitch from './components/RangeSwitch.svelte';
  import StateBanner from './components/StateBanner.svelte';
  import TopModels from './components/TopModels.svelte';
  import TrayHeader from './components/TrayHeader.svelte';
  import UpdateBanner from './components/UpdateBanner.svelte';
  import { createTrayStore } from './store.svelte';

  const store = createTrayStore();
  let shellEl = $state<HTMLElement | null>(null);
  let firstActionEl = $state<HTMLButtonElement | null>(null);

  onMount(() => {
    const cleanup = store.init();
    firstActionEl?.focus();
    return cleanup;
  });

  $effect(() => {
    const el = shellEl;
    if (!el) return;

    let lastHeight = 0;
    const syncHeight = () => {
      const rectHeight = Math.ceil(el.getBoundingClientRect().height);
      const nextHeight = rectHeight > 0 ? rectHeight : el.offsetHeight;
      if (nextHeight > 0 && nextHeight !== lastHeight) {
        lastHeight = nextHeight;
        void ipc.resizeTray(nextHeight).catch(() => {});
      }
    };

    syncHeight();

    if (typeof ResizeObserver === 'undefined') {
      return;
    }

    const observer = new ResizeObserver(() => {
      syncHeight();
    });
    observer.observe(el);
    return () => observer.disconnect();
  });

  function handleWindowKeydown(event: KeyboardEvent): void {
    if ((event.metaKey || event.ctrlKey) && event.key === ',') {
      event.preventDefault();
      store.openSettings();
      return;
    }
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

<main bind:this={shellEl} class="tray-shell ui-root" data-view="tray">
  <TrayHeader
    syncing={store.isSyncing}
    onsync={() => void store.triggerSync('full_manual')}
    onopensettings={() => store.openSettings()}
    bind:firstActionEl
  />

  <UpdateBanner
    update={store.visibleUpdate}
    onupdate={() => void store.downloadAndInstallUpdate()}
    onrestart={() => void store.restartToApplyUpdate()}
    ondetails={() => store.openSettings('app')}
    ondismiss={store.dismissUpdateBanner}
  />

  <StateBanner
    partial={store.isPartial}
    warnings={store.warnings}
    error={store.error}
    onretry={handleBannerRetry}
  />

  <div class="tray-body">
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
  </div>

  <FooterActions
    syncing={store.isSyncing}
    syncProgress={store.syncProgress}
    syncMessage={store.syncProgress?.message}
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
