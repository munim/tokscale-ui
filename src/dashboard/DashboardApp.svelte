<script lang="ts">
  import '../styles/tokens.css';
  import {
    ipc,
    type DashboardNavigateEvent,
    type DashboardQuery,
    type DashboardSnapshot,
    type DashboardTab,
  } from '$lib/contracts/ipc';
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

  let activeTab = $state<DashboardTab>('overview');
  let query = $state<DashboardQuery>({
    range: '30d',
    groupBy: 'model',
    activeClients: [],
    sort: 'cost',
  });
  let snapshot = $state<DashboardSnapshot | null>(null);

  $effect(() => {
    const currentQuery = { ...query, activeClients: [...query.activeClients] };
    void ipc.getDashboardSnapshot(currentQuery).then((data) => {
      snapshot = data;
    });
  });

  function handleNavigate(handoff: DashboardNavigateEvent): void {
    activeTab = handoff.tab;
    if (handoff.range) {
      query.range = handoff.range;
    }
    if (handoff.query !== undefined) {
      query.search = handoff.query;
    }
  }
</script>

<main class="dashboard-stub">
  <nav aria-label="Dashboard tabs">
    {#each ['overview', 'usage', 'models', 'daily', 'hourly', 'monthly', 'sessions', 'projects', 'stats', 'agents'] as const as tab}
      <button type="button" onclick={() => (activeTab = tab)}>{tab}</button>
    {/each}
  </nav>

  {#if snapshot}
    {#if activeTab === 'overview'}
      <OverviewView {snapshot} {query} onnavigate={handleNavigate} />
    {:else if activeTab === 'usage'}
      <UsageView {snapshot} {query} onnavigate={handleNavigate} />
    {:else if activeTab === 'models'}
      <ModelsView {snapshot} {query} onnavigate={handleNavigate} />
    {:else if activeTab === 'daily'}
      <DailyView {snapshot} {query} onnavigate={handleNavigate} />
    {:else if activeTab === 'hourly'}
      <HourlyView {snapshot} {query} onnavigate={handleNavigate} />
    {:else if activeTab === 'monthly'}
      <MonthlyView {snapshot} {query} onnavigate={handleNavigate} />
    {:else if activeTab === 'sessions'}
      <SessionsView {snapshot} {query} onnavigate={handleNavigate} />
    {:else if activeTab === 'projects'}
      <ProjectsView {snapshot} {query} onnavigate={handleNavigate} />
    {:else if activeTab === 'stats'}
      <StatsView {snapshot} {query} onnavigate={handleNavigate} />
    {:else if activeTab === 'agents'}
      <AgentsView {snapshot} {query} onnavigate={handleNavigate} />
    {/if}
  {/if}
</main>
