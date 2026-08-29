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
  let rawQuery = $state('');
  let debouncedQuery = $state('');
  let statusFilter = $state<'all' | 'with_sim' | 'no_sim' | 'selected' | 'no_modem' | 'errors'>('all');

  function hasValidSim(p: PortInfo): boolean {
    return Boolean(
      p.sim_number &&
      p.sim_number !== '-' &&
      p.sim_number.trim() !== '' &&
      p.sim_number !== 'Unknown'
    );
  }

  /** Probed and silent — an empty SIM slot rather than a fault. */
  function isNoModem(p: PortInfo): boolean {
    return p.alive === false;
  }

  // debounce port search 150ms — trivial cost for 64 ports but avoids reactive churn
  let portSearchTimer: ReturnType<typeof setTimeout> | null = null;
  function onPortSearchInput(v: string) {
    rawQuery = v;
    if (portSearchTimer) clearTimeout(portSearchTimer);
    portSearchTimer = setTimeout(() => {
      portSearchTimer = null;
      debouncedQuery = v;
    }, 150);
  }
  function clearPortSearch() {
    if (portSearchTimer) clearTimeout(portSearchTimer);
    portSearchTimer = null;
    rawQuery = '';
    debouncedQuery = '';
  }

  $effect(() => {
    return () => {
      if (portSearchTimer) clearTimeout(portSearchTimer);
    };
  });

  const totalPorts = $derived(portsStore.items.length);
  const counts = $derived.by(() => {
    let checked = 0;
    let errors = 0;
    let withSim = 0;
    let noSim = 0;
    let alive = 0;
    let noModem = 0;
    let probed = 0;
    for (const p of portsStore.items) {
      if (p.checked) checked++;
      // A silent port is expected on a partly-filled bank, so it must not be
      // counted as an error or the error badge reads 57 on a healthy setup.
      if (p.live_error && !isNoModem(p)) errors++;
      if (hasValidSim(p)) withSim++;
      else noSim++;
      if (p.alive !== null) probed++;
      if (p.alive === true) alive++;
      if (isNoModem(p)) noModem++;
    }
    return { checked, errors, withSim, noSim, alive, noModem, probed };
  });
  const checkedCount = $derived(counts.checked);
  const errorCount = $derived(counts.errors);
  const withSimCount = $derived(counts.withSim);
  const noSimCount = $derived(counts.noSim);
  const aliveCount = $derived(counts.alive);
  const noModemCount = $derived(counts.noModem);
  const hasProbed = $derived(counts.probed > 0);

  const filteredPorts = $derived.by(() => {
    const q = debouncedQuery.trim().toLowerCase();
    return portsStore.items.filter(p => {
      if (statusFilter === 'with_sim' && !hasValidSim(p)) return false;
      if (statusFilter === 'no_sim' && hasValidSim(p)) return false;
      if (statusFilter === 'selected' && !p.checked) return false;
      if (statusFilter === 'no_modem' && !isNoModem(p)) return false;
      if (statusFilter === 'errors' && !(p.live_error && !isNoModem(p))) return false;
      if (!q) return true;
      const hay = `${portLabel(p.name)} ${p.name} ${p.sim_number}`.toLowerCase();
      return hay.includes(q);
    });
  });

  const isFiltering = $derived(statusFilter !== 'all' || debouncedQuery.trim().length > 0);

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

  function selectWithSim() {
    if (liveStore.on) return;
    const updates: Array<{ name: string; changes: { checked: boolean } }> = [];
    for (const p of portsStore.items) {
      const nextChecked = hasValidSim(p);
      if (p.checked !== nextChecked) {
        api.togglePortChecked(p.name, nextChecked);
        updates.push({ name: p.name, changes: { checked: nextChecked } });
      }
    }
    if (updates.length > 0) {
      portsStore.batchUpdatePorts(updates);
    }
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
    return messagesStore.getMessageCountForPort(port.name);
  }

  function portOtpCount(port: PortInfo): number {
    return messagesStore.getOtpCountForPort(port.name);
  }

  function formatSimNumber(num: string): string {
    if (num.length > 15) return num.slice(0, 15) + '…';
    return num;
  }

  let lastPortTrigger: HTMLElement | null = $state(null);
  let prevActivePort: string | null = $state(null);
  $effect(() => {
    if (activePortName !== null && prevActivePort === null) {
      lastPortTrigger = document.activeElement as HTMLElement | null;
    }
    prevActivePort = activePortName;
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && activePortName !== null) {
      activePortName = null;
      requestAnimationFrame(() => lastPortTrigger?.focus());
    }
  }

  function closePortDetail() {
    activePortName = null;
    requestAnimationFrame(() => lastPortTrigger?.focus());
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex-1 flex flex-col h-full overflow-hidden min-w-0" id="panel-ports">
  <header class="page-header">
    <div class="flex items-center gap-2 sm:gap-3 mr-auto min-w-0 flex-wrap gap-y-1">
      <h1 class="page-title shrink-0">Ports</h1>
      <span class="badge badge-primary font-mono tabular-nums shrink-0" title="Serial ports the OS reports">
        {totalPorts} port{totalPorts !== 1 ? 's' : ''}
      </span>
      {#if hasProbed}
        <span
          class="badge badge-success font-mono tabular-nums shrink-0"
          title="Ports that answered an AT probe"
        >
          {aliveCount} modem{aliveCount !== 1 ? 's' : ''}
        </span>
        {#if noModemCount > 0}
          <span
            class="badge badge-muted font-mono tabular-nums shrink-0"
            title="Ports with no modem answering — empty SIM slots"
          >
            {noModemCount} empty
          </span>
        {/if}
      {/if}
      {#if errorCount > 0}
        <span class="badge badge-danger font-mono tabular-nums shrink-0">{errorCount} error{errorCount !== 1 ? 's' : ''}</span>
      {/if}
    </div>
    <div class="flex items-center gap-1.5 sm:gap-2 flex-wrap w-full sm:w-auto">
      <button
        class="btn-secondary text-primary border-primary/30 hover:bg-primary/10 gap-1.5"
        onclick={() => api.detectPorts()}
        disabled={liveStore.on || liveStore.detectBusy || liveStore.ussdBusy || liveStore.scanBusy || totalPorts === 0}
        title="Send a quick AT probe to every port and keep only the ones with a modem selected"
      >
        {#if liveStore.detectBusy}
          <Icon name="loader" size={13} class="animate-spin" />
          <span>Detecting…</span>
        {:else}
          <Icon name="monitor" size={13} />
          <span>Detect Modems</span>
        {/if}
      </button>
      <button
        class="btn-secondary text-primary border-primary/30 hover:bg-primary/10 gap-1.5"
        onclick={selectWithSim}
        disabled={liveStore.on || totalPorts === 0 || withSimCount === 0}
        title="Check only ports with detected SIM numbers"
      >
        <Icon name="sim" size={13} />
        <span>Select with SIM ({withSimCount})</span>
      </button>
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
      <div class="hidden sm:block w-px h-5 bg-border mx-0.5" role="separator" aria-hidden="true"></div>
      <button
        class="btn-primary"
        onclick={() => api.getSimNumbers()}
        disabled={liveStore.on || liveStore.ussdBusy}
        title="Get SIM numbers for checked ports"
      >
        {#if liveStore.ussdBusy}
          <Icon name="loader" size={14} class="animate-spin" />
          <span>Getting SIMs...</span>
        {:else}
          <Icon name="sim" size={14} />
          <span>Get SIM Numbers</span>
        {/if}
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

  <div class="flex items-center gap-2 gap-y-2 px-3 sm:px-5 py-2 bg-surface border-b border-border shrink-0 flex-wrap">
    <div class="relative w-full sm:w-56 max-w-full min-w-[180px] flex-1 sm:flex-none">
      <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none" aria-hidden="true">
        <Icon name="search" size={13} />
      </span>
      <input
        type="text"
        class="input text-xs py-0 pl-8 pr-7"
        placeholder="Search name or SIM number…"
        value={rawQuery}
        oninput={(e) => { onPortSearchInput((e.target as HTMLInputElement).value); }}
        aria-label="Search ports"
      />
      {#if rawQuery}
        <button
          class="btn-icon absolute right-1 top-1/2 -translate-y-1/2 w-5 h-5"
          onclick={clearPortSearch}
          title="Clear search"
          aria-label="Clear search"
        >
          <Icon name="x" size={11} strokeWidth={2} />
        </button>
      {/if}
    </div>

    <div class="flex items-center gap-0.5 p-0.5 rounded-md bg-background border border-border" role="group" aria-label="Port status filter">
      <button
        class="inline-flex items-center gap-1.5 h-7 px-2 rounded-[5px] text-xs font-medium transition-colors duration-150
               focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
               {statusFilter === 'all' ? 'bg-elevated text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
        aria-pressed={statusFilter === 'all'}
        onclick={() => { statusFilter = 'all'; }}
      >
        All
        <span class="px-1.5 min-w-[18px] text-center rounded-full text-[10px] leading-4 tabular-nums bg-muted text-muted-foreground">{totalPorts}</span>
      </button>
      <button
        class="inline-flex items-center gap-1.5 h-7 px-2 rounded-[5px] text-xs font-medium transition-colors duration-150
               focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
               {statusFilter === 'with_sim' ? 'bg-elevated text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
        aria-pressed={statusFilter === 'with_sim'}
        onclick={() => { statusFilter = 'with_sim'; }}
      >
        With SIM
        <span class="px-1.5 min-w-[18px] text-center rounded-full text-[10px] leading-4 tabular-nums bg-success/20 text-success">{withSimCount}</span>
      </button>
      <button
        class="inline-flex items-center gap-1.5 h-7 px-2 rounded-[5px] text-xs font-medium transition-colors duration-150
               focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
               {statusFilter === 'no_sim' ? 'bg-elevated text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
        aria-pressed={statusFilter === 'no_sim'}
        onclick={() => { statusFilter = 'no_sim'; }}
      >
        No SIM
        <span class="px-1.5 min-w-[18px] text-center rounded-full text-[10px] leading-4 tabular-nums bg-muted text-muted-foreground">{noSimCount}</span>
      </button>
      <button
        class="inline-flex items-center gap-1.5 h-7 px-2 rounded-[5px] text-xs font-medium transition-colors duration-150
               focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
               {statusFilter === 'selected' ? 'bg-elevated text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
        aria-pressed={statusFilter === 'selected'}
        onclick={() => { statusFilter = 'selected'; }}
      >
        Selected
        <span class="px-1.5 min-w-[18px] text-center rounded-full text-[10px] leading-4 tabular-nums bg-primary/15 text-primary">{checkedCount}</span>
      </button>
      <button
        class="inline-flex items-center gap-1.5 h-7 px-2 rounded-[5px] text-xs font-medium transition-colors duration-150
               focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
               {statusFilter === 'no_modem' ? 'bg-elevated text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
        aria-pressed={statusFilter === 'no_modem'}
        onclick={() => { statusFilter = 'no_modem'; }}
        title="Ports where no modem answered the AT probe"
      >
        No Modem
        <span class="px-1.5 min-w-[18px] text-center rounded-full text-[10px] leading-4 tabular-nums bg-muted text-muted-foreground">{noModemCount}</span>
      </button>
      <button
        class="inline-flex items-center gap-1.5 h-7 px-2 rounded-[5px] text-xs font-medium transition-colors duration-150
               focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
               {statusFilter === 'errors' ? 'bg-elevated text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
        aria-pressed={statusFilter === 'errors'}
        onclick={() => { statusFilter = 'errors'; }}
      >
        Errors
        <span class="px-1.5 min-w-[18px] text-center rounded-full text-[10px] leading-4 tabular-nums {errorCount > 0 ? 'bg-danger/15 text-danger' : 'bg-muted text-muted-foreground'}">{errorCount}</span>
      </button>
    </div>

    <div class="hidden lg:block flex-1 min-w-[12px]"></div>

    <span class="text-xs text-muted-foreground font-mono tabular-nums whitespace-nowrap shrink-0">
      {isFiltering ? `${filteredPorts.length} of ${totalPorts} shown` : ''}
    </span>
  </div>

  <div class="flex-1 flex overflow-hidden min-h-0 min-w-0 relative">
    <div class="flex-1 overflow-auto p-3 sm:p-5 bg-background min-w-0 min-h-0">
      {#if refreshing && !portsStore.hasLoaded}
        <div class="grid gap-3 port-grid" style="grid-template-columns: repeat(auto-fill, minmax(260px, 1fr))" aria-label="Loading ports">
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
            onclick={() => { clearPortSearch(); statusFilter = 'all'; }}
          >
            Clear Filters
          </button>
        </div>
      {:else}
        <div class="grid gap-3 port-grid" style="grid-template-columns: repeat(auto-fill, minmax(260px, 1fr))">
          {#each filteredPorts as port, index (port.name)}
            {@const st = portStatus(port, liveStore.on)}
            <div
              role="button"
              tabindex="0"
              data-port={port.name}
              class="group card p-4 text-left transition-colors duration-150 cursor-pointer hover:border-muted-foreground/40 waterfall-row
                     focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
                     {activePortName === port.name ? 'border-primary ring-1 ring-primary/40' : ''}
                     {st.key === 'error' ? 'bg-danger/[0.04]' : ''}"
              style="animation-delay: {Math.min(index * 24, 380)}ms;"
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
                        <span class="w-1.5 h-1.5 rounded-full shrink-0 aspect-square bg-current {st.key === 'connecting' ? 'animate-pulse-dot' : ''}" aria-hidden="true"></span>
                      {/if}
                      {st.label}
                    </span>
                  </div>
                  <div class="flex items-center gap-3 text-xs mb-2">
                    {#if hasValidSim(port)}
                      <span class="flex items-center gap-1.5 font-mono text-foreground font-medium truncate" title={`SIM Number: ${port.sim_number}`}>
                        <Icon name="sim" size={12} class="text-success shrink-0" />
                        <span class="truncate">{formatSimNumber(port.sim_number)}</span>
                      </span>
                    {:else}
                      <span class="flex items-center gap-1.5 font-mono text-muted-foreground/60 truncate" title="No SIM number detected">
                        <Icon name="sim" size={12} class="opacity-40 shrink-0" />
                        <span class="truncate italic text-[11px]">No SIM</span>
                      </span>
                    {/if}
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

              {#if isNoModem(port)}
                <div class="mt-3 p-2.5 rounded-md bg-elevated border border-border text-xs text-muted-foreground animate-fade-in">
                  <div class="flex items-center gap-1.5 font-medium">
                    <Icon name="info" size={13} strokeWidth={2} />
                    No modem on this port
                  </div>
                  <div class="mt-1 ml-[22px] text-[10px] opacity-80">
                    Nothing answered the AT probe — most likely an empty SIM slot. Skipped by scan and live.
                  </div>
                </div>
              {:else if port.live_error}
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
        onclose={closePortDetail}
      />
    {/if}
  </div>

  <footer class="page-footer font-mono flex-wrap">
    <div class="flex-1 flex items-center gap-4 min-w-0">
      <span class="tabular-nums whitespace-nowrap shrink-0">Checked: {checkedCount} / {totalPorts}</span>
      {#if liveStore.statusText}
        <span class="text-primary truncate" title={liveStore.statusText}>
          {liveStore.statusText}
        </span>
      {/if}
    </div>
    <span class="flex items-center gap-2 sm:gap-3 tabular-nums shrink-0 whitespace-nowrap">
      {#if errorCount > 0}
        <span class="text-danger whitespace-nowrap">{errorCount} error{errorCount !== 1 ? 's' : ''}</span>
      {/if}
      <span class="flex items-center gap-1.5 whitespace-nowrap">
        <span
          class="w-1.5 h-1.5 rounded-full shrink-0 aspect-square {liveStore.on ? 'bg-success animate-pulse-dot' : 'bg-muted-foreground/50'}"
          aria-hidden="true"
        ></span>
        {liveStore.on ? `Live: ${liveStore.readyPorts.length} ready` : 'Live mode off'}
      </span>
    </span>
  </footer>
</div>
