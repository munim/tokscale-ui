<script lang="ts">
  import { tick } from 'svelte';
  import type { SessionDetail, SessionModelBreakdown, SessionRow } from '$lib/contracts/ipc';
  import { formatDuration, formatRelative } from '$lib/format/date';
  import { formatUsd } from '$lib/format/money';
  import { formatTokens, formatTokensFull } from '$lib/format/tokens';
  import { resolveHarness } from '$lib/harnesses/registry';
  import { DataTable, HarnessBadge, StatusPill, type ColumnDef } from '$lib/ui';
  import { getSessionDetail } from '../views/sessions/api';
  import { formatLocalDateTime, formatSessionTitle } from '../views/sessions/helpers';

  interface Props {
    session: SessionRow | null;
    sessions?: SessionRow[];
    onselect?: (session: SessionRow) => void;
    onclose: () => void;
    returnFocusTo?: HTMLElement | null;
  }

  let { session, sessions, onselect, onclose, returnFocusTo = null }: Props = $props();

  let drawerEl = $state<HTMLElement | undefined>(undefined);
  let closeBtnEl = $state<HTMLButtonElement | undefined>(undefined);
  let detail = $state<SessionDetail | null>(null);
  let loading = $state(false);
  let errorMessage = $state<string | null>(null);
  let copied = $state(false);

  let wasOpen = false;
  let capturedOpener: HTMLElement | null = null;
  let detailRequestSeq = 0;
  let copyTimer: ReturnType<typeof setTimeout> | null = null;

  let sessionList = $derived(sessions && sessions.length > 0 ? sessions : session ? [session] : []);
  let currentIndex = $derived(
    session ? sessionList.findIndex((item) => item.sessionId === session.sessionId) : -1,
  );
  let hasPrev = $derived(Boolean(onselect) && currentIndex > 0);
  let hasNext = $derived(
    Boolean(onselect) && currentIndex >= 0 && currentIndex < sessionList.length - 1,
  );

  let activeSession = $derived(detail?.session ?? session);
  let drawerTitle = $derived(activeSession ? formatSessionTitle(activeSession) : 'Session details');

  function clearCopyTimer(): void {
    if (copyTimer !== null) {
      clearTimeout(copyTimer);
      copyTimer = null;
    }
  }

  async function loadDetail(target: SessionRow): Promise<void> {
    const seq = ++detailRequestSeq;
    loading = true;
    errorMessage = null;
    detail = null;
    try {
      const loaded = await getSessionDetail(target.client, target.sessionId);
      if (seq !== detailRequestSeq) {
        return;
      }
      detail = loaded;
    } catch (err) {
      if (seq !== detailRequestSeq) {
        return;
      }
      errorMessage = err instanceof Error ? err.message : 'Unable to load session details';
    } finally {
      if (seq === detailRequestSeq) {
        loading = false;
      }
    }
  }

  $effect(() => {
    const current = session;
    if (!current) {
      wasOpen = false;
      detail = null;
      loading = false;
      errorMessage = null;
      copied = false;
      clearCopyTimer();
      return;
    }

    if (!wasOpen) {
      wasOpen = true;
      const active =
        typeof document !== 'undefined' && document.activeElement instanceof HTMLElement
          ? document.activeElement
          : null;
      capturedOpener = returnFocusTo ?? (active && active !== document.body ? active : null);
      void tick().then(() => {
        if (closeBtnEl && closeBtnEl.isConnected) {
          closeBtnEl.focus();
        } else {
          drawerEl?.focus();
        }
      });
    }

    copied = false;
    clearCopyTimer();
    void loadDetail(current);
  });

  function closeDrawer(): void {
    const target = returnFocusTo ?? capturedOpener;
    capturedOpener = null;
    wasOpen = false;
    clearCopyTimer();
    onclose();
    if (target && target.isConnected) {
      target.focus();
    }
  }

  function goPrev(): void {
    if (!hasPrev || !onselect) return;
    const prev = sessionList[currentIndex - 1];
    if (prev) {
      onselect(prev);
    }
  }

  function goNext(): void {
    if (!hasNext || !onselect) return;
    const next = sessionList[currentIndex + 1];
    if (next) {
      onselect(next);
    }
  }

  async function handleCopyId(): Promise<void> {
    if (!session) return;
    try {
      if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(session.sessionId);
      }
      copied = true;
      clearCopyTimer();
      copyTimer = setTimeout(() => {
        copied = false;
        copyTimer = null;
      }, 2000);
    } catch {
      copied = false;
    }
  }

  function getFocusableElements(): HTMLElement[] {
    if (!drawerEl) return [];
    const selector = [
      'button:not([disabled])',
      '[href]',
      'input:not([disabled])',
      'select:not([disabled])',
      'textarea:not([disabled])',
      '[tabindex]:not([tabindex="-1"]):not([disabled])',
    ].join(', ');
    return Array.from(drawerEl.querySelectorAll<HTMLElement>(selector)).filter(
      (el) => !el.hasAttribute('hidden') && el.getAttribute('aria-hidden') !== 'true',
    );
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if (!session) return;

    if (event.key === 'Escape') {
      event.preventDefault();
      closeDrawer();
      return;
    }

    if (event.key === 'ArrowLeft') {
      if (hasPrev) {
        event.preventDefault();
        goPrev();
      }
      return;
    }

    if (event.key === 'ArrowRight') {
      if (hasNext) {
        event.preventDefault();
        goNext();
      }
      return;
    }

    if (event.key === 'Tab' && drawerEl) {
      const focusables = getFocusableElements();
      if (focusables.length === 0) {
        event.preventDefault();
        drawerEl.focus();
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = document.activeElement as HTMLElement | null;
      const insideDrawer = Boolean(active && drawerEl.contains(active));

      if (event.shiftKey) {
        if (!insideDrawer || active === first || active === drawerEl) {
          event.preventDefault();
          last.focus();
        }
      } else if (!insideDrawer || active === last) {
        event.preventDefault();
        first.focus();
      }
    }
  }

  function formatRelativeHint(value?: string): string | undefined {
    if (!value) return undefined;
    const ms = Date.parse(value);
    if (!Number.isFinite(ms)) return undefined;
    return formatRelative(ms);
  }

  const modelColumns: ColumnDef<SessionModelBreakdown>[] = [
    {
      key: 'model',
      label: 'Model',
      render: (row) => row.model,
      title: (row) => `${row.provider} / ${row.model}`,
    },
    {
      key: 'input',
      label: 'Input',
      align: 'end',
      numeric: true,
      render: (row) => formatTokens(row.tokens.input),
      title: (row) => formatTokensFull(row.tokens.input),
    },
    {
      key: 'output',
      label: 'Output',
      align: 'end',
      numeric: true,
      render: (row) => formatTokens(row.tokens.output),
      title: (row) => formatTokensFull(row.tokens.output),
    },
    {
      key: 'cacheRead',
      label: 'Cache read',
      align: 'end',
      numeric: true,
      render: (row) => formatTokens(row.tokens.cacheRead),
      title: (row) => formatTokensFull(row.tokens.cacheRead),
    },
    {
      key: 'cacheWrite',
      label: 'Cache write',
      align: 'end',
      numeric: true,
      render: (row) => formatTokens(row.tokens.cacheWrite),
      title: (row) => formatTokensFull(row.tokens.cacheWrite),
    },
    {
      key: 'reasoning',
      label: 'Reasoning',
      align: 'end',
      numeric: true,
      render: (row) => formatTokens(row.tokens.reasoning),
      title: (row) => formatTokensFull(row.tokens.reasoning),
    },
    {
      key: 'cost',
      label: 'Cost',
      align: 'end',
      numeric: true,
      render: (row) => formatUsd(row.cost),
    },
  ];
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if session}
  <div
    class="drawer-scrim"
    data-testid="drawer-scrim"
    aria-hidden="true"
    onclick={closeDrawer}
  ></div>

  <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
  <aside
    bind:this={drawerEl}
    class="session-drawer"
    role="dialog"
    aria-modal="true"
    aria-labelledby="drawer-title"
    tabindex="-1"
  >
    <header class="drawer-header">
      <div class="drawer-header-top">
        <span class="drawer-eyebrow">Session details</span>
        <div class="drawer-nav-actions">
          <button
            type="button"
            class="drawer-btn"
            disabled={!hasPrev}
            onclick={goPrev}
          >
            ← Prev
          </button>
          <button
            type="button"
            class="drawer-btn"
            disabled={!hasNext}
            onclick={goNext}
          >
            Next →
          </button>
          <button
            bind:this={closeBtnEl}
            type="button"
            class="drawer-close-btn"
            aria-label="Close session details"
            title="Close (Esc)"
            onclick={closeDrawer}
          >
            ×
          </button>
        </div>
      </div>

      <h2 id="drawer-title" class="drawer-title">{drawerTitle}</h2>

      <div class="drawer-id-row">
        <code class="drawer-session-id" title={session.sessionId}>{session.sessionId}</code>
        <button type="button" class="drawer-btn drawer-copy-btn" onclick={handleCopyId}>
          {copied ? 'Copied' : 'Copy session ID'}
        </button>
      </div>
    </header>

    <div class="drawer-body">
      {#if loading}
        <div class="drawer-skeleton" aria-busy="true" aria-label="Loading session details">
          <div class="skeleton-bar skeleton-bar-lg"></div>
          <div class="skeleton-bar skeleton-bar-md"></div>
          <div class="skeleton-bar skeleton-bar-sm"></div>
        </div>
      {:else if errorMessage}
        <div class="drawer-error" role="alert">
          <p>{errorMessage}</p>
          <button type="button" class="drawer-btn" onclick={() => loadDetail(session)}>
            Retry
          </button>
        </div>
      {:else if detail}
        {@const s = detail.session}
        {@const harnessMeta = resolveHarness(s.client)}
        {@const lastActiveRelative = formatRelativeHint(s.lastActive)}

        <section class="drawer-section" aria-labelledby="drawer-sec-overview">
          <h3 id="drawer-sec-overview" class="drawer-section-title">Session overview</h3>
          <dl class="drawer-kv-list">
            <div class="drawer-kv-row">
              <dt>Harness</dt>
              <dd class="drawer-harness-val">
                <HarnessBadge id={s.client} size={16} />
                <span>{harnessMeta.label}</span>
              </dd>
            </div>
            <div class="drawer-kv-row">
              <dt>Workspace</dt>
              <dd>{s.workspaceLabel?.trim() || '—'}</dd>
            </div>
            <div class="drawer-kv-row">
              <dt>Workspace key</dt>
              <dd class="drawer-mono">{s.workspace?.trim() || '—'}</dd>
            </div>
            <div class="drawer-kv-row">
              <dt>Created at</dt>
              <dd class="drawer-mono">{formatLocalDateTime(s.createdAt)}</dd>
            </div>
            <div class="drawer-kv-row">
              <dt>Last active</dt>
              <dd class="drawer-mono" title={lastActiveRelative}>
                {formatLocalDateTime(s.lastActive)}
              </dd>
            </div>
            <div class="drawer-kv-row">
              <dt>Messages</dt>
              <dd class="drawer-mono">{formatTokensFull(s.messageCount)}</dd>
            </div>
            <div class="drawer-kv-row">
              <dt>Duration</dt>
              <dd class="drawer-mono">{formatDuration(s.durationMinutes)}</dd>
            </div>
            <div class="drawer-kv-row">
              <dt>Complexity</dt>
              <dd>{detail.complexity?.trim() || '—'}</dd>
            </div>
            <div class="drawer-kv-row">
              <dt>Task category</dt>
              <dd>{detail.taskCategory?.trim() || '—'}</dd>
            </div>
          </dl>
        </section>

        <section class="drawer-section" aria-labelledby="drawer-sec-activity">
          <h3 id="drawer-sec-activity" class="drawer-section-title">Activity</h3>
          <p class="drawer-description">{detail.description?.trim() || '—'}</p>
          {#if detail.taskGroup?.trim()}
            <div class="drawer-pill-row">
              <StatusPill tone="neutral" label={detail.taskGroup} />
            </div>
          {/if}
        </section>

        <section class="drawer-section" aria-labelledby="drawer-sec-tokens">
          <h3 id="drawer-sec-tokens" class="drawer-section-title">Tokens &amp; cost</h3>
          <dl class="drawer-totals-grid">
            <div class="drawer-total-cell">
              <dt>Input</dt>
              <dd class="drawer-mono" title={formatTokensFull(detail.totalTokens.input)}>
                {formatTokens(detail.totalTokens.input)}
              </dd>
            </div>
            <div class="drawer-total-cell">
              <dt>Output</dt>
              <dd class="drawer-mono" title={formatTokensFull(detail.totalTokens.output)}>
                {formatTokens(detail.totalTokens.output)}
              </dd>
            </div>
            <div class="drawer-total-cell">
              <dt>Cache read</dt>
              <dd class="drawer-mono" title={formatTokensFull(detail.totalTokens.cacheRead)}>
                {formatTokens(detail.totalTokens.cacheRead)}
              </dd>
            </div>
            <div class="drawer-total-cell">
              <dt>Cache write</dt>
              <dd class="drawer-mono" title={formatTokensFull(detail.totalTokens.cacheWrite)}>
                {formatTokens(detail.totalTokens.cacheWrite)}
              </dd>
            </div>
            <div class="drawer-total-cell">
              <dt>Reasoning</dt>
              <dd class="drawer-mono" title={formatTokensFull(detail.totalTokens.reasoning)}>
                {formatTokens(detail.totalTokens.reasoning)}
              </dd>
            </div>
            <div class="drawer-total-cell">
              <dt>Cost</dt>
              <dd class="drawer-mono drawer-total-cost">{formatUsd(detail.totalCost)}</dd>
            </div>
          </dl>

          <DataTable
            columns={modelColumns}
            rows={detail.models}
            rowKey={(m) => `${m.provider}:${m.model}`}
            rowHeight={30}
            maxHeight={240}
            ariaLabel="Per-model token and cost breakdown"
          />
        </section>
      {/if}
    </div>
  </aside>
{/if}

<style>
  .drawer-scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: color-mix(in oklch, var(--shadow-ink) 35%, transparent);
  }

  .session-drawer {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 41;
    width: 440px;
    max-width: 100vw;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    color: var(--fg);
    border-left: 1px solid var(--border);
    box-shadow: -8px 0 28px color-mix(in oklch, var(--shadow-ink) 18%, transparent);
    font-family: var(--font-body);
  }

  .session-drawer:focus {
    outline: none;
  }

  .drawer-header {
    padding: 16px 18px 14px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .drawer-header-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .drawer-eyebrow {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
  }

  .drawer-nav-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .drawer-btn,
  .drawer-close-btn {
    appearance: none;
    height: 28px;
    padding: 0 10px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    font-family: var(--font-body);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .drawer-close-btn {
    width: 28px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    line-height: 1;
  }

  .drawer-btn:hover:not(:disabled),
  .drawer-close-btn:hover:not(:disabled) {
    background: var(--fg-soft);
  }

  .drawer-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .drawer-btn:focus-visible,
  .drawer-close-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .drawer-title {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
    line-height: 1.35;
    color: var(--fg);
    word-break: break-word;
  }

  .drawer-id-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .drawer-session-id {
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .drawer-copy-btn {
    flex-shrink: 0;
  }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 18px 22px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .drawer-skeleton {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 8px 0;
  }

  .skeleton-bar {
    height: 14px;
    border-radius: var(--radius);
    background: color-mix(in oklch, var(--fg) 10%, var(--surface));
    animation: drawer-pulse 1.2s ease-in-out infinite;
  }

  .skeleton-bar-lg {
    width: 100%;
  }

  .skeleton-bar-md {
    width: 78%;
  }

  .skeleton-bar-sm {
    width: 54%;
  }

  @keyframes drawer-pulse {
    0%,
    100% {
      opacity: 0.5;
    }
    50% {
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .skeleton-bar {
      animation: none;
    }
  }

  .drawer-error {
    padding: 12px;
    border-radius: var(--radius);
    border: 1px solid color-mix(in oklch, var(--danger) 40%, var(--border));
    background: color-mix(in oklch, var(--danger) 10%, var(--surface));
    color: var(--danger-ink);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    font-size: 12.5px;
  }

  .drawer-error p {
    margin: 0;
  }

  .drawer-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-bottom: 16px;
    border-bottom: 1px solid color-mix(in oklch, var(--border) 65%, transparent);
  }

  .drawer-section:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .drawer-section-title {
    margin: 0;
    font-size: 12px;
    font-weight: 650;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }

  .drawer-kv-list {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .drawer-kv-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    font-size: 12.5px;
  }

  .drawer-kv-row dt {
    color: var(--muted);
    flex-shrink: 0;
  }

  .drawer-kv-row dd {
    margin: 0;
    color: var(--fg);
    text-align: end;
    word-break: break-word;
  }

  .drawer-harness-val {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .drawer-mono {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .drawer-description {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--fg);
  }

  .drawer-pill-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .drawer-totals-grid {
    margin: 0;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .drawer-total-cell {
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--fg-soft);
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .drawer-total-cell dt {
    font-size: 11px;
    color: var(--muted);
  }

  .drawer-total-cell dd {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--fg);
  }

  .drawer-total-cost {
    color: var(--accent-strong);
  }
</style>
