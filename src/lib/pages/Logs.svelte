<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import { logsStore } from '$lib/stores/logs.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';
  import { portLabel } from '$lib/utils/port';
  import { api } from '$lib/services/api';
  import type { LogLevelFilter } from '$lib/types/logs';
  import { onMount, tick } from 'svelte';

  let logContainerEl: HTMLElement | undefined = $state();
  let userScrolledUp = $state(false);

  const levelOptions: Array<{ id: LogLevelFilter; label: string; count: number; badgeCls: string }> = $derived([
    { id: 'ALL', label: 'All', count: logsStore.counts.all, badgeCls: 'badge-muted' },
    { id: 'ERROR', label: 'Error', count: logsStore.counts.error, badgeCls: 'bg-danger/20 text-danger border-danger/40' },
    { id: 'WARN', label: 'Warn', count: logsStore.counts.warn, badgeCls: 'bg-warning/20 text-warning border-warning/40' },
    { id: 'INFO', label: 'Info', count: logsStore.counts.info, badgeCls: 'bg-primary/20 text-primary border-primary/40' },
  ]);

  function scrollToBottom(force = false) {
    if (!logContainerEl) return;
    if (force || (logsStore.autoScroll && !userScrolledUp)) {
      logContainerEl.scrollTop = logContainerEl.scrollHeight;
    }
  }

  function handleScroll() {
    if (!logContainerEl) return;
    const distanceToBottom =
      logContainerEl.scrollHeight - logContainerEl.scrollTop - logContainerEl.clientHeight;
    // If scrolled up more than 40px, pause auto-scroll
    userScrolledUp = distanceToBottom > 40;
  }

  $effect(() => {
    // Whenever logs change, auto scroll if enabled
    void logsStore.filtered.length;
    tick().then(() => scrollToBottom());
  });

  let refreshTimer: ReturnType<typeof setInterval> | null = null;

  async function fetchLogs() {
    if (!logsStore.isStreaming) return;
    await api.getLogs(1000);
  }

  onMount(() => {
    void fetchLogs().then(() => tick().then(() => scrollToBottom(true)));
    refreshTimer = setInterval(() => {
      void fetchLogs();
    }, 1000);
    return () => {
      if (refreshTimer) clearInterval(refreshTimer);
    };
  });

  function levelPill(level: string) {
    const l = level.toUpperCase();
    if (l === 'ERROR') return { text: 'ERR', cls: 'bg-danger/20 text-danger border-danger/40' };
    if (l === 'WARN') return { text: 'WARN', cls: 'bg-warning/20 text-warning border-warning/40' };
    return { text: 'INFO', cls: 'bg-primary/15 text-primary border-primary/30' };
  }
</script>

<div class="flex-1 flex flex-col min-h-0 bg-background overflow-hidden">
  <!-- Logs Page Header -->
  <header class="page-header border-b border-border/80 px-4 py-3 bg-surface/40 backdrop-blur-md">
    <div class="flex items-center gap-3 mr-auto flex-wrap">
      <div class="flex items-center gap-2">
        <Icon name="terminal" size={18} class="text-primary" />
        <h1 class="page-title text-base">Live Logs &amp; Diagnostics</h1>
      </div>

      <div
        class="badge {logsStore.isStreaming ? 'badge-success' : 'badge-warning'} font-mono tabular-nums text-xs px-2.5 py-0.5 inline-flex items-center gap-1.5"
      >
        <span
          class="w-1.5 h-1.5 rounded-full shrink-0 aspect-square {logsStore.isStreaming ? 'bg-success animate-pulse-dot' : 'bg-warning'}"
          aria-hidden="true"
        ></span>
        <span>{logsStore.isStreaming ? 'LIVE' : 'PAUSED'}</span>
      </div>

      <!-- Level Filter Pills -->
      <div class="flex items-center gap-1 bg-elevated/40 p-0.5 rounded-lg border border-border/50">
        {#each levelOptions as opt (opt.id)}
          <button
            class="px-2 py-1 text-xs rounded-md font-medium transition-all duration-150 flex items-center gap-1.5
                   {logsStore.filterLevel === opt.id
                     ? 'bg-primary text-primary-foreground shadow-sm font-semibold'
                     : 'text-muted-foreground hover:text-foreground hover:bg-elevated/80'}"
            onclick={() => (logsStore.filterLevel = opt.id)}
          >
            <span>{opt.label}</span>
            <span
              class="text-[10px] px-1.5 py-0.2 rounded-full tabular-nums font-mono
                     {logsStore.filterLevel === opt.id
                       ? 'bg-primary-foreground/20 text-primary-foreground'
                       : 'bg-surface/80 text-muted-foreground'}"
            >
              {opt.count}
            </span>
          </button>
        {/each}
      </div>
    </div>

    <!-- Right-side Action Controls -->
    <div class="flex items-center gap-2 flex-wrap">
      <!-- Port Filter Dropdown -->
      {#if portsStore.items.length > 0}
        <div class="relative">
          <select
            class="input h-8 text-xs font-mono pr-7 pl-2 py-0 bg-surface min-w-[110px] cursor-pointer"
            value={logsStore.portFilter ?? ''}
            onchange={(e) => {
              const val = (e.target as HTMLSelectElement).value;
              logsStore.portFilter = val ? val : null;
            }}
            aria-label="Filter logs by COM port"
          >
            <option value="">All Ports</option>
            {#each portsStore.items as p (p.name)}
              <option value={p.name}>{portLabel(p.name)} ({p.name})</option>
            {/each}
          </select>
        </div>
      {/if}

      <!-- Search Input -->
      <div class="relative w-44 sm:w-56">
        <Icon name="search" size={13} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none" />
        <input
          type="text"
          placeholder="Filter logs (e.g. timeout, AT)..."
          class="input pl-8 pr-7 py-1 h-8 text-xs font-mono w-full bg-surface"
          bind:value={logsStore.searchQuery}
        />
        {#if logsStore.searchQuery}
          <button
            class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground p-0.5 rounded"
            onclick={() => (logsStore.searchQuery = '')}
            title="Clear search"
          >
            <Icon name="x" size={12} />
          </button>
        {/if}
      </div>

      <!-- Streaming Toggle Button -->
      <button
        class="btn {logsStore.isStreaming ? 'btn-secondary text-warning' : 'btn-success'} h-8 px-2.5 text-xs gap-1.5"
        onclick={() => (logsStore.isStreaming = !logsStore.isStreaming)}
        title={logsStore.isStreaming ? 'Pause real-time stream' : 'Resume real-time stream'}
      >
        <Icon name={logsStore.isStreaming ? 'pause' : 'play'} size={13} />
        <span>{logsStore.isStreaming ? 'Pause' : 'Resume'}</span>
      </button>

      <!-- Auto Scroll Toggle -->
      <button
        class="btn {logsStore.autoScroll ? 'btn-primary' : 'btn-secondary'} h-8 px-2.5 text-xs gap-1.5"
        onclick={() => {
          logsStore.autoScroll = !logsStore.autoScroll;
          if (logsStore.autoScroll) {
            userScrolledUp = false;
            scrollToBottom(true);
          }
        }}
        title="Keep scrolled to bottom on new logs"
      >
        <Icon name="chevron-down" size={13} />
        <span>Auto-scroll</span>
      </button>

      <!-- Copy Filtered Logs -->
      <button
        class="btn btn-secondary h-8 px-2.5 text-xs gap-1.5"
        onclick={() => logsStore.copyAll()}
        title="Copy filtered logs to clipboard"
      >
        <Icon name="copy" size={13} />
        <span class="hidden md:inline">Copy</span>
      </button>

      <!-- Export to File -->
      <button
        class="btn btn-secondary h-8 px-2.5 text-xs gap-1.5"
        onclick={() => logsStore.exportLogs()}
        title="Save logs to file"
      >
        <Icon name="download" size={13} />
        <span class="hidden md:inline">Export</span>
      </button>

      <!-- Open Log Folder -->
      <button
        class="btn btn-secondary h-8 px-2.5 text-xs gap-1.5"
        onclick={() => api.openLogFolder()}
        title="Open application log directory"
      >
        <Icon name="folder" size={13} />
        <span class="hidden lg:inline">Folder</span>
      </button>

      <!-- Clear Console -->
      <button
        class="btn btn-danger-quiet h-8 px-2.5 text-xs gap-1.5"
        onclick={() => api.clearLogs()}
        title="Clear log display"
      >
        <Icon name="trash" size={13} />
        <span>Clear</span>
      </button>
    </div>
  </header>

  <!-- Log Viewport / Terminal Console -->
  <div
    bind:this={logContainerEl}
    onscroll={handleScroll}
    class="flex-1 overflow-y-auto p-4 font-mono text-xs leading-relaxed bg-[rgb(var(--console-bg))] text-[rgb(var(--console-fg))] select-text"
  >
    {#if logsStore.filtered.length === 0}
      <div class="flex flex-col items-center justify-center h-full text-center text-muted-foreground/60 py-16">
        <Icon name="terminal" size={44} strokeWidth={1.2} class="mb-3 opacity-30" />
        <div class="text-sm font-semibold text-muted-foreground">No logs to display</div>
        <div class="text-xs mt-1 text-muted-foreground/80">
          {#if logsStore.items.length > 0}
            Try adjusting search or level filters.
          {:else}
            Backend logs and events will stream here live.
          {/if}
        </div>
      </div>
    {:else}
      <div class="space-y-1">
        {#each logsStore.filtered as entry (entry.id)}
          {@const meta = levelPill(entry.level)}
          <div
            class="flex items-start gap-2.5 py-1 px-2 rounded hover:bg-[rgb(var(--console-row-hover))] transition-colors border-l-2
                   {entry.level.toUpperCase() === 'ERROR' ? 'border-danger bg-danger/[0.06]' : ''}
                   {entry.level.toUpperCase() === 'WARN' ? 'border-warning bg-warning/[0.04]' : ''}
                   {entry.level.toUpperCase() === 'INFO' ? 'border-primary/40' : ''}
                   {entry.level.toUpperCase() === 'DEBUG' || entry.level.toUpperCase() === 'TRACE' ? 'border-transparent text-muted-foreground' : ''}"
          >
            <!-- Timestamp -->
            <span class="text-muted-foreground/70 tabular-nums shrink-0 select-none text-[11px]">
              {entry.timestamp}
            </span>

            <!-- Level Pill -->
            <span
              class="px-1.5 py-0.2 rounded text-[10px] font-bold uppercase tracking-wider shrink-0 border {meta.cls} select-none"
            >
              {meta.text}
            </span>

            <!-- Module / Target -->
            <span class="text-primary/90 font-semibold shrink-0 text-[11px] select-none">
              [{entry.target}]
            </span>

            <!-- Message with soft wrapping -->
            <span class="flex-1 break-all whitespace-pre-wrap text-foreground/95 selection:bg-primary/30">
              {entry.message}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Bottom Log Stats Bar -->
  <footer class="px-4 py-1.5 border-t border-border/80 bg-surface/50 text-[11px] font-mono flex items-center justify-between text-muted-foreground">
    <div class="flex items-center gap-3">
      <span>Total: <strong class="text-foreground">{logsStore.items.length}</strong></span>
      <span class="w-px h-3 bg-border"></span>
      <span>Showing: <strong class="text-foreground">{logsStore.filtered.length}</strong></span>
      {#if userScrolledUp}
        <span class="w-px h-3 bg-border"></span>
        <button
          class="text-primary hover:underline flex items-center gap-1 font-semibold"
          onclick={() => { userScrolledUp = false; scrollToBottom(true); }}
        >
          <Icon name="chevron-down" size={11} />
          Scroll to newest
        </button>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <span class="text-muted-foreground/70">Buffer: 1,000 max in-memory</span>
    </div>
  </footer>
</div>
