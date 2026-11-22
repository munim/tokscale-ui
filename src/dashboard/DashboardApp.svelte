<script lang="ts">
  import '../styles/tokens.css';
  import '../lib/ui/ui.css';
  import './dashboard.css';
  import type { DashboardNavigateEvent } from '$lib/contracts/ipc';
  import AppBar from './shell/AppBar.svelte';
  import KpiStrip from './shell/KpiStrip.svelte';
  import { createDashboardRouter } from './shell/router.svelte';
  import { installDashboardShortcuts } from './shell/shortcuts';
  import StatusBar from './shell/StatusBar.svelte';
  import TabBar from './shell/TabBar.svelte';
  import Toolbar from './shell/Toolbar.svelte';
  import AgentsView from './views/AgentsView.svelte';
  import DailyView from './views/DailyView.svelte';
  import HourlyView from './views/HourlyView.svelte';
  import ModelsView from './views/ModelsView.svelte';
  import MonthlyView from './views/MonthlyView.svelte';
  import OverviewView from './views/OverviewView.svelte';
  import ProjectsView from './views/ProjectsView.svelte';
  import SessionsView from './views/SessionsView.svelte';
  import StatsView from './views/StatsView.svelte';
  import UsageView from './views/UsageView.svelte';

  const router = createDashboardRouter();
  let appBarRef = $state<ReturnType<typeof AppBar> | undefined>(undefined);

  $effect(() => {
    const stopRouter = router.start();
    const stopShortcuts = installDashboardShortcuts({
      onPrevTab: () => router.stepTab(-1),
      onNextTab: () => router.stepTab(1),
      onSort: (sort) => router.setSort(sort),
      onRefresh: () => void router.refresh(),
      onFocusSearch: () => appBarRef?.focusSearch(),
      onClearSearch: () => router.clearSearch(),
    });
    return () => {
      stopShortcuts();
      stopRouter();
    };
  });

  function handleNavigate(handoff: DashboardNavigateEvent): void {
    router.applyHandoff(handoff);
  }
</script>

<div class="dashboard-shell ui-root">
  <AppBar
    bind:this={appBarRef}
    search={router.query.search ?? ''}
    query={router.query}
    sources={router.snapshot?.sources.clients ?? []}
    syncing={router.syncing}
    onSearchChange={(value) => router.setSearch(value)}
    onClearSearch={() => router.clearSearch()}
    onOpenSources={() => router.setTab('agents')}
    onThemeChange={(mode) => void router.setThemeMode(mode)}
    onRefresh={() => void router.refresh()}
  />

  <TabBar
    activeTab={router.activeTab}
    onchange={(tab) => router.setTab(tab)}
  />

  <Toolbar
    query={router.query}
    sources={router.snapshot?.sources.clients ?? []}
    customRangeError={router.customRangeError}
    onRangeChange={(range) => router.setRange(range)}
    onCustomDatesChange={(since, until) => router.setCustomDates(since, until)}
    onGroupByChange={(groupBy) => router.setGroupBy(groupBy)}
    onSortChange={(sort) => router.setSort(sort)}
    onToggleClient={(clientId) => router.toggleClient(clientId)}
    onClearClients={() => router.clearClients()}
  />

  {#if router.snapshot}
    <KpiStrip kpis={router.snapshot.kpis} />
  {:else}
    <div class="kpi-strip-placeholder" aria-hidden="true"></div>
  {/if}

  <main class="dashboard-content">
    {#if router.error}
      <div class="dashboard-banner" role="alert">
        <span>{router.error}</span>
      </div>
    {/if}

    {#if router.snapshot}
      {#if router.activeTab === 'overview'}
        <OverviewView
          snapshot={router.snapshot}
          query={router.query}
          onnavigate={handleNavigate}
        />
      {:else if router.activeTab === 'usage'}
        <UsageView
          snapshot={router.snapshot}
          query={router.query}
          onnavigate={handleNavigate}
        />
      {:else if router.activeTab === 'models'}
        <ModelsView
          snapshot={router.snapshot}
          query={router.query}
          onnavigate={handleNavigate}
        />
      {:else if router.activeTab === 'daily'}
        <DailyView
          snapshot={router.snapshot}
          query={router.query}
          onnavigate={handleNavigate}
        />
      {:else if router.activeTab === 'hourly'}
        <HourlyView
          snapshot={router.snapshot}
          query={router.query}
          onnavigate={handleNavigate}
        />
      {:else if router.activeTab === 'monthly'}
        <MonthlyView
          snapshot={router.snapshot}
          query={router.query}
          onnavigate={handleNavigate}
        />
      {:else if router.activeTab === 'sessions'}
        <SessionsView
          snapshot={router.snapshot}
          query={router.query}
          onnavigate={handleNavigate}
        />
      {:else if router.activeTab === 'projects'}
        <ProjectsView
          snapshot={router.snapshot}
          query={router.query}
          onnavigate={handleNavigate}
        />
      {:else if router.activeTab === 'stats'}
        <StatsView
          snapshot={router.snapshot}
          query={router.query}
          onnavigate={handleNavigate}
        />
      {:else if router.activeTab === 'agents'}
        <AgentsView
          snapshot={router.snapshot}
          query={router.query}
          onnavigate={handleNavigate}
        />
      {/if}
    {:else}
      <div class="dashboard-loading" role="status" aria-live="polite">
        Loading usage snapshot…
      </div>
    {/if}
  </main>

  <StatusBar
    lastSyncedAt={router.lastSyncedAt}
    syncing={router.syncing}
    syncMessage={router.syncMessage}
  />
</div>
