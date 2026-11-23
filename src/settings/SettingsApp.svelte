<script lang="ts">
  import { onMount } from 'svelte';
  import '../styles/tokens.css';
  import '$lib/ui/ui.css';
  import './settings.css';
  import type { SettingsSection } from '$lib/contracts/ipc';
  import { BrandMark } from '$lib/ui';
  import { matchSettingsSearch, SETTINGS_NAV, type SearchableSetting } from './searchIndex';
  import { createSettingsStore } from './store.svelte';
  import AppAlertsView from './views/AppAlertsView.svelte';
  import CliAdvancedView from './views/CliAdvancedView.svelte';
  import PricingView from './views/PricingView.svelte';
  import ProvidersView from './views/ProvidersView.svelte';
  import SocialView from './views/SocialView.svelte';
  import SourcesView from './views/SourcesView.svelte';

  const store = createSettingsStore();
  let searchInputEl = $state<HTMLInputElement | null>(null);
  let openPricingAddForm = $state(false);

  const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.userAgent);

  onMount(() => {
    return store.init();
  });

  let currentNav = $derived(
    SETTINGS_NAV.find((item) => item.id === store.activeSection) ?? SETTINGS_NAV[0],
  );

  let searchHits = $derived(matchSettingsSearch(store.searchQuery));
  let isSearching = $derived(store.searchQuery.trim().length > 0);

  function selectSearchHit(hit: SearchableSetting): void {
    store.setSection(hit.section, hit.id);
    store.clearSearch();
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'f') {
      event.preventDefault();
      searchInputEl?.focus();
      searchInputEl?.select();
      return;
    }
    if (event.key === 'Escape' && isSearching) {
      event.preventDefault();
      store.clearSearch();
    }
  }

  function sectionIconPath(id: SettingsSection): string {
    switch (id) {
      case 'app':
        return 'M8 2.25a5.75 5.75 0 1 0 0 11.5 5.75 5.75 0 0 0 0-11.5Zm0 3.25a2.5 2.5 0 1 1 0 5 2.5 2.5 0 0 1 0-5Z';
      case 'sources':
        return 'M2.5 4.25A1.25 1.25 0 0 1 3.75 3h3l1.25 1.5h4.25a1.25 1.25 0 0 1 1.25 1.25v6a1.25 1.25 0 0 1-1.25 1.25h-8.5A1.25 1.25 0 0 1 2.5 11.75v-7.5Z';
      case 'providers':
        return 'M5.5 2.5v3M10.5 2.5v3M4 5.5h8v2.25a4 4 0 0 1-8 0V5.5ZM8 11.75V14';
      case 'pricing':
        return 'M8 2.25v11.5M10.75 5a2.5 2.5 0 0 0-2.5-1.5H7.25a1.75 1.75 0 0 0 0 3.5h1.5a1.75 1.75 0 0 1 0 3.5H7.25a2.5 2.5 0 0 1-2.5-1.5';
      case 'social':
        return 'M4.5 12.25a3.5 3.5 0 0 1-.4-6.98 4.25 4.25 0 0 1 8.15-1.15A3 3 0 0 1 12 12.25H4.5Z';
      case 'cli':
        return 'M3 4.25 6.75 8 3 11.75M8.25 11.75H13';
    }
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="settings-shell ui-root" data-view="settings">
  <aside class="settings-sidebar" aria-label="Settings navigation">
    <div class="settings-sidebar-top" class:is-mac={isMac} data-tauri-drag-region>
      <div class="settings-brand" data-tauri-drag-region>
        <BrandMark size={17} />
        <span class="settings-brand-name">tokscale</span>
        <span class="settings-brand-tag">Settings</span>
      </div>

      <div class="settings-search-wrap" data-tauri-drag-region="false">
        <svg
          class="settings-search-icon"
          viewBox="0 0 16 16"
          width="13"
          height="13"
          fill="none"
          aria-hidden="true"
        >
          <circle cx="7" cy="7" r="4.5" stroke="currentColor" stroke-width="1.4" />
          <path d="M10.5 10.5L13.5 13.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
        </svg>
        <input
          bind:this={searchInputEl}
          type="search"
          class="settings-search-input"
          placeholder="Search settings…"
          aria-label="Search settings"
          value={store.searchQuery}
          oninput={(e) => store.setSearchQuery((e.currentTarget as HTMLInputElement).value)}
        />
        {#if isSearching}
          <button
            type="button"
            class="settings-search-clear"
            aria-label="Clear settings search"
            onclick={store.clearSearch}
          >
            ×
          </button>
        {/if}
      </div>
    </div>

    <nav class="settings-nav-scroll" aria-label="Settings sections">
      {#if isSearching}
        <div class="settings-search-results" role="region" aria-label="Search results">
          {#if searchHits.length > 0}
            <div class="settings-search-group-label">Matches ({searchHits.length})</div>
            {#each searchHits as hit (hit.id)}
              <button
                type="button"
                class="settings-search-hit"
                onclick={() => selectSearchHit(hit)}
              >
                <span class="settings-search-hit-title">{hit.label}</span>
                <span class="settings-search-hit-desc">{hit.description}</span>
              </button>
            {/each}
          {:else}
            <div class="settings-search-empty">No settings matching “{store.searchQuery}”</div>
          {/if}
        </div>
      {:else}
        {#each SETTINGS_NAV as item (item.id)}
          {@const active = store.activeSection === item.id}
          <button
            type="button"
            class="settings-nav-item"
            class:is-active={active}
            aria-current={active ? 'page' : undefined}
            onclick={() => store.setSection(item.id)}
          >
            <span class="settings-nav-left">
              <span class="settings-nav-icon" aria-hidden="true">
                <svg viewBox="0 0 16 16" width="15" height="15" fill="none">
                  <path
                    d={sectionIconPath(item.id)}
                    stroke="currentColor"
                    stroke-width="1.4"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </span>
              <span class="settings-nav-label">{item.label}</span>
            </span>

            {#if item.id === 'sources'}
              <span class="settings-nav-badge">
                <span
                  class="settings-nav-dot"
                  class:is-warn={store.hasSourceWarnings}
                  aria-hidden="true"
                ></span>
                {store.detectedSourcesCount}
              </span>
            {:else if item.id === 'providers' && store.connectedProviderCount > 0}
              <span class="settings-nav-badge">
                <span class="settings-nav-dot" aria-hidden="true"></span>
                {store.connectedProviderCount}
              </span>
            {:else if item.id === 'social' && store.socialStatus?.loggedIn}
              <span class="settings-nav-dot" aria-hidden="true"></span>
            {/if}
          </button>
        {/each}
      {/if}
    </nav>

    <footer class="settings-sidebar-footer">
      <div class="settings-cli-pill">
        <span
          class="settings-cli-dot"
          class:is-ok={Boolean(store.cliInfo && store.cliInfo.resolvedVia !== 'none')}
          aria-hidden="true"
        ></span>
        <span class="settings-cli-ver">
          {store.cliInfo?.version ? `v${store.cliInfo.version}` : 'CLI not found'}
        </span>
      </div>
      {#if store.cliInfo?.resolvedVia && store.cliInfo.resolvedVia !== 'none'}
        <span class="settings-cli-via">{store.cliInfo.resolvedVia}</span>
      {/if}
    </footer>
  </aside>

  <main class="settings-main">
    <header class="settings-header" data-tauri-drag-region>
      <div class="settings-header-title-wrap" data-tauri-drag-region>
        <h1 class="settings-header-title">{currentNav.label}</h1>
      </div>

      <div class="settings-header-actions" data-tauri-drag-region="false">
        {#if store.toast}
          <span class="settings-toast" data-tone={store.toast.tone} role="status" aria-live="polite">
            {store.toast.text}
          </span>
        {/if}

        {#if store.activeSection === 'providers'}
          <button
            type="button"
            class="s-btn s-btn-sm"
            disabled={store.loadingProviders}
            onclick={() => void store.refreshProviders()}
          >
            Refresh
          </button>
          <button
            type="button"
            class="s-btn s-btn-sm s-btn-primary"
            disabled={store.syncingAllProviders}
            onclick={() => void store.syncAllConnectedProviders()}
          >
            {store.syncingAllProviders
              ? 'Syncing…'
              : `Sync All (${store.connectedProviderCount})`}
          </button>
        {:else if store.activeSection === 'pricing'}
          <button
            type="button"
            class="s-btn s-btn-sm"
            disabled={store.refreshingPricing}
            onclick={() => void store.refreshPricingCatalog()}
          >
            {store.refreshingPricing ? 'Refreshing…' : 'Refresh Catalog'}
          </button>
          <button
            type="button"
            class="s-btn s-btn-sm s-btn-primary"
            onclick={() => (openPricingAddForm = true)}
          >
            + Override
          </button>
        {:else if store.activeSection === 'social'}
          <button
            type="button"
            class="s-btn s-btn-sm"
            disabled={store.loadingSocial}
            onclick={() => void store.refreshSocial()}
          >
            Refresh Status
          </button>
        {/if}
      </div>
    </header>

    <div class="settings-canvas" class:is-flush={store.activeSection === 'providers'}>
      {#if store.activeSection === 'app'}
        <AppAlertsView {store} />
      {:else if store.activeSection === 'sources'}
        <SourcesView {store} />
      {:else if store.activeSection === 'providers'}
        <ProvidersView {store} />
      {:else if store.activeSection === 'pricing'}
        <PricingView
          {store}
          showAddForm={openPricingAddForm}
          oncloseaddform={() => (openPricingAddForm = false)}
        />
      {:else if store.activeSection === 'social'}
        <SocialView {store} />
      {:else if store.activeSection === 'cli'}
        <CliAdvancedView {store} />
      {/if}
    </div>
  </main>
</div>
