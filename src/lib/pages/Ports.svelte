<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PortDetail from '$lib/components/PortDetail.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { liveStore } from '$lib/stores/live.svelte';
  import { api } from '$lib/services/api';
  import { portLabel, portStatus } from '$lib/utils/port';
  import type { PortInfo } from '$lib/types';

  let refreshing = $state(false);
  let activePortName = $state<string | null>(null);
  let query = $state('');
  let statusFilter = $state<'all' | 'selected' | 'errors'>('all');

  const totalPorts = $derived(portsStore.items.length);
  const checkedCount = $derived(portsStore.items.filter(p => p.checked).length);
  const errorCount = $derived(portsStore.items.filter(p => p.live_error).length);

  const filteredPorts = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return portsStore.items.filter(p => {
      if (statusFilter === 'selected' && !p.checked) return false;
      if (statusFilter === 'errors' && !p.live_error) return false;
      if (!q) return true;
      const hay = `${portLabel(p.name)} ${p.name} ${p.sim_number}`.toLowerCase();
      return hay.includes(q);
    });
  });

  const isFiltering = $derived(statusFilter !== 'all' || query.trim().length > 0);

  onMount(() => {
    if (!portsStore.hasLoaded) void refreshPorts();
  });

  async function refreshPorts() {
    if (refreshing || liveStore.on) return;
    refreshing = true;
    try {
      await api.refreshPorts();
    } finally {
      refreshing = false;
    }
  }

  function setAllChecked(checked: boolean) {
    api.setAllPortsChecked(checked);
    portsStore.setCheckedAll(checked);
  }

  function togglePort(port: PortInfo) {
    api.togglePortChecked(port.name, !port.checked);
    portsStore.updatePort(port.name, { checked: !port.checked });
  }

  function inspect(port: PortInfo) {
    activePortName = port.name;
  }

  function cardKeydown(port: PortInfo, e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      inspect(port);
    }
  }

  function portMessageCount(port: PortInfo): number {
    let n = 0;
    for (const item of messagesStore.items) {
      if (item.message.port === port.name) n++;
    }
    return n;
  }

  function portOtpCount(port: PortInfo): number {
    let n = 0;
    for (const item of messagesStore.items) {
      if (item.message.port === port.name && item.otp) n++;
    }
    return n;
  }

  function formatSimNumber(num: string): string {
    if (num.length > 15) return num.slice(0, 15) + '…';
    return num;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && activePortName !== null) {
      activePortName = null;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex-1 flex flex-col h-full overflow-hidden" id="panel-ports">
  <header class="page-header">
    <div class="flex items-center gap-3 mr-auto min-w-0">
      <h1 class="page-title">Ports</h1>
      <span class="badge badge-primary font-mono tabular-nums" title="Detected ports">
        {totalPorts} port{totalPorts !== 1 ? 's' : ''}
      </span>
      {#if errorCount > 0}
        <span class="badge badge-danger font-mono tabular-nums">{errorCount} error{errorCount !== 1 ? 's' : ''}</span>
      {/if}
    </div>
    <div class="flex items-center gap-2 flex-wrap">
      <button
        class="btn-secondary"
        onclick={() => setAllChecked(true)}
        disabled={liveStore.on || totalPorts === 0 || checkedCount === totalPorts}
        title="Include all ports in scan"
      >
        Select All
      </button>
      <button
        class="btn-secondary"
        onclick={() => setAllChecked(false)}
        disabled={liveStore.on || checkedCount === 0}
        title="Exclude all ports from scan"
      >
        Deselect All
      </button>
      <div class="w-px h-5 bg-border mx-0.5" role="separator"></div>
      <button
        class="btn-primary"
        onclick={() => api.getSimNumbers()}
        disabled={liveStore.on || liveStore.ussdBusy}
        title="Get SIM numbers for checked ports"
      >
        {#if liveStore.ussdBusy}
          <Icon name="loader" size={14} class="animate-spin" />
        {:else}
          <Icon name="sim" size={14} />
        {/if}
        Get SIM Numbers
      </button>
      <button
        class="btn-secondary"
        onclick={refreshPorts}
        disabled={liveStore.on || refreshing}
        title="Refresh port list"
      >
        <Icon name={refreshing ? 'loader' : 'refresh'} size={14} class={refreshing ? 'animate-spin' : ''} />
        {refreshing ? 'Refreshing…' : 'Refresh'}
      </button>
    </div>
  </header>

  <div class="flex items-center gap-2 px-5 py-2 bg-surface border-b border-border shrink-0 flex-wrap">
    <div class="relative w-56 max-w-full">
      <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none" aria-hidden="true">
        <Icon name="search" size={13} />
      </span>
      <input
        type="text"
        class="input text-xs py-0 pl-8 pr-7"
        placeholder="Search name or SIM number…"
        value={query}
        oninput={(e) => { query = (e.target as HTMLInputElement).value; }}
        aria-label="Search ports"
      />
      {#if query}
        <button
          class="btn-icon absolute right-1 top-1/2 -translate-y-1/2 w-5 h-5"
          onclick={() => { query = ''; }}
          title="Clear search"
          aria-label="Clear search"
        >
          <Icon name="x" size={11} strokeWidth={2} />
        </button>
      {/if}
    </div>

    <div class="flex items-center gap-0.5 p-0.5 rounded-md bg-background border border-border" role="group" aria-label="Port status filter">
      <button
        class="inline-flex items-center gap-1.5 h-7 px-2.5 rounded-[5px] text-xs font-medium transition-colors duration-150
               focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
               {statusFilter === 'all' ? 'bg-elevated text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
        aria-pressed={statusFilter === 'all'}
        onclick={() => { statusFilter = 'all'; }}
      >
        All
        <span class="px-1.5 min-w-[18px] text-center rounded-full text-[10px] leading-4 tabular-nums bg-muted text-muted-foreground">{totalPorts}</span>
      </button>
      <button
        class="inline-flex items-center gap-1.5 h-7 px-2.5 rounded-[5px] text-xs font-medium transition-colors duration-150
               focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
               {statusFilter === 'selected' ? 'bg-elevated text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
        aria-pressed={statusFilter === 'selected'}
        onclick={() => { statusFilter = 'selected'; }}
      >
        Selected
        <span class="px-1.5 min-w-[18px] text-center rounded-full text-[10px] leading-4 tabular-nums bg-primary/15 text-primary">{checkedCount}</span>
      </button>
      <button
        class="inline-flex items-center gap-1.5 h-7 px-2.5 rounded-[5px] text-xs font-medium transition-colors duration-150
               focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
               {statusFilter === 'errors' ? 'bg-elevated text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
        aria-pressed={statusFilter === 'errors'}
        onclick={() => { statusFilter = 'errors'; }}
      >
        Errors
        <span class="px-1.5 min-w-[18px] text-center rounded-full text-[10px] leading-4 tabular-nums {errorCount > 0 ? 'bg-danger/15 text-danger' : 'bg-muted text-muted-foreground'}">{errorCount}</span>
      </button>
    </div>

    <div class="flex-1"></div>

    <span class="text-xs text-muted-foreground font-mono tabular-nums whitespace-nowrap">
      {isFiltering ? `${filteredPorts.length} of ${totalPorts} shown` : ''}
    </span>
  </div>

  <div class="flex-1 flex overflow-hidden min-h-0">
    <div class="flex-1 overflow-auto p-5 bg-background min-w-0">
      {#if refreshing && !portsStore.hasLoaded}
        <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));" aria-label="Loading ports">
          {#each Array(6) as _, i (i)}
            <div class="card p-4 animate-pulse">
              <div class="flex items-start gap-3">
                <div class="w-8 h-8 rounded-md bg-muted/60"></div>
                <div class="flex-1 space-y-2">
                  <div class="h-3.5 w-24 rounded bg-muted/60"></div>
                  <div class="h-3 w-32 rounded bg-muted/40"></div>
                </div>
              </div>
              <div class="mt-4 pt-3 border-t border-border/50 flex gap-4">
                <div class="h-3 w-14 rounded bg-muted/40"></div>
                <div class="h-3 w-14 rounded bg-muted/40"></div>
              </div>
            </div>
          {/each}
        </div>
      {:else if portsStore.hasLoaded && totalPorts === 0}
        <div class="empty-state">
          <Icon name="ports" size={48} strokeWidth={1.25} class="mb-4 opacity-30" />
          <div class="empty-state-title">No ports detected</div>
          <div class="empty-state-hint">Connect a GSM modem or SIM bank device and click Refresh</div>
          <button class="btn-primary mt-4" onclick={refreshPorts}>Refresh Ports</button>
        </div>
      {:else if filteredPorts.length === 0}
        <div class="empty-state">
          <Icon name="search" size={40} strokeWidth={1.25} class="mb-3 opacity-30" />
          <div class="empty-state-title">No ports match</div>
          <div class="empty-state-hint">No ports match the current search or status filter</div>
          <button
            class="btn-secondary mt-4"
            onclick={() => { query = ''; statusFilter = 'all'; }}
          >
            Clear Filters
          </button>
        </div>
      {:else}
        <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));">
          {#each filteredPorts as port (port.name)}
            {@const st = portStatus(port, liveStore.on)}
            <div
              role="button"
              tabindex="0"
              data-port={port.name}
              class="group card p-4 text-left transition-colors duration-150 cursor-pointer hover:border-muted-foreground/40
                     focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
                     {activePortName === port.name ? 'border-primary ring-1 ring-primary/40' : ''}
                     {st.key === 'error' ? 'bg-danger/[0.04]' : ''}"
              onclick={(e) => {
                if ((e.target as HTMLElement).closest('input, label')) return;
                inspect(port);
              }}
              onkeydown={(e) => cardKeydown(port, e)}
              title="View port details"
            >
              <div class="flex items-start gap-3">
                <div class={`w-8 h-8 rounded-md flex items-center justify-center shrink-0 ${st.tile}`} aria-hidden="true">
                  <Icon name="sim" size={17} />
                </div>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-1 min-w-0">
                    <input
                      id={`port-check-${port.name}`}
                      type="checkbox"
                      class="checkbox w-4 h-4"
                      checked={port.checked}
                      onchange={() => togglePort(port)}
                      aria-label={`Include ${portLabel(port.name)} in scan`}
                    />
                    <label
                      for={`port-check-${port.name}`}
                      class="font-mono text-sm font-semibold text-foreground truncate group-hover:text-primary transition-colors cursor-pointer"
                    >
                      {portLabel(port.name)}
                    </label>
                    <span class="flex-1"></span>
                    <span class="badge {st.badge}">
                      {#if st.key === 'live' || st.key === 'connecting'}
                        <span class="w-1.5 h-1.5 rounded-full bg-current {st.key === 'connecting' ? 'animate-pulse-dot' : ''}" aria-hidden="true"></span>
                      {/if}
                      {st.label}
                    </span>
                  </div>
                  <div class="flex items-center gap-3 text-xs text-muted-foreground mb-2">
                    <span class="flex items-center gap-1.5 font-mono truncate" title={port.sim_number || 'Unknown'}>
                      <Icon name="sim" size={12} />
                      {formatSimNumber(port.sim_number || 'Unknown')}
                    </span>
                  </div>
                </div>
                <Icon
                  name="chevron-right"
                  size={15}
                  class="text-muted-foreground/50 mt-1"
                />
              </div>

              <div class="mt-3 pt-3 border-t border-border/50 flex items-center justify-between gap-2">
                <div class="flex items-center gap-4 text-[11px] font-mono text-muted-foreground tabular-nums min-w-0">
                  <span class="flex items-center gap-1.5 whitespace-nowrap">
                    <span class="w-1.5 h-1.5 rounded-full bg-primary" aria-hidden="true"></span>
                    {portMessageCount(port)} msg
                  </span>
                  {#if portOtpCount(port) > 0}
                    <span class="flex items-center gap-1.5 whitespace-nowrap">
                      <span class="w-1.5 h-1.5 rounded-full bg-otp" aria-hidden="true"></span>
                      {portOtpCount(port)} OTP
                    </span>
                  {/if}
                </div>
                <span class="font-mono text-[10px] text-muted-foreground/60 truncate" title={port.name}>{port.name}</span>
              </div>

              {#if port.live_error}
                <div class="mt-3 p-2.5 rounded-md bg-danger/10 border border-danger/25 text-xs text-danger animate-fade-in">
                  <div class="flex items-center gap-1.5 font-medium">
                    <Icon name="alert-circle" size={13} strokeWidth={2} />
                    Connection Error
                  </div>
                  <div class="mt-1 ml-[22px] text-[10px] opacity-80 font-mono break-all">{port.live_error}</div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>

    {#if activePortName !== null && portsStore.find(activePortName)}
      <PortDetail
        name={activePortName}
        ontoggle={() => {
          const p = portsStore.find(activePortName!);
          if (p) togglePort(p);
        }}
        onclose={() => { activePortName = null; }}
      />
    {/if}
  </div>

  <footer class="page-footer font-mono">
    <div class="flex-1 flex items-center gap-4 min-w-0">
      <span class="tabular-nums shrink-0">Checked: {checkedCount} / {totalPorts}</span>
      {#if liveStore.statusText}
        <span class="text-primary truncate" title={liveStore.statusText}>
          {liveStore.statusText}
        </span>
      {/if}
    </div>
    <span class="flex items-center gap-3 tabular-nums shrink-0">
      {#if errorCount > 0}
        <span class="text-danger">{errorCount} error{errorCount !== 1 ? 's' : ''}</span>
      {/if}
      <span class="flex items-center gap-1.5">
        <span
          class="w-1.5 h-1.5 rounded-full {liveStore.on ? 'bg-success animate-pulse-dot' : 'bg-muted-foreground/50'}"
          aria-hidden="true"
        ></span>
        {liveStore.on ? `Live: ${liveStore.readyPorts.length} ready` : 'Live mode off'}
      </span>
    </span>
  </footer>
</div>
