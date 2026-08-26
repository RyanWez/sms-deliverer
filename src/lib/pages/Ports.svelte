<script lang="ts">
  import { portsStore } from '$lib/stores/ports.svelte';
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { liveStore } from '$lib/stores/live.svelte';
  import { api } from '$lib/services/api';
  import { portLabel } from '$lib/utils/port';
  import type { PortInfo } from '$lib/types';

  const portMessageCounts = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const item of messagesStore.items) {
      counts.set(item.message.port, (counts.get(item.message.port) ?? 0) + 1);
    }
    return counts;
  });

  const portOtpCounts = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const item of messagesStore.items) {
      if (item.otp) {
        counts.set(item.message.port, (counts.get(item.message.port) ?? 0) + 1);
      }
    }
    return counts;
  });

  async function refreshPorts() {
    await api.refreshPorts();
  }

  function togglePort(port: PortInfo) {
    api.togglePortChecked(port.name, !port.checked);
    portsStore.updatePort(port.name, { checked: !port.checked });
  }

  function getSimNumber(port: PortInfo) {
    if (port.sim_number) return port.sim_number;
    return 'Unknown';
  }

  function getMessageCount(port: PortInfo) {
    return portMessageCounts.get(port.name) ?? 0;
  }

  function getOtpCount(port: PortInfo) {
    return portOtpCounts.get(port.name) ?? 0;
  }

  function formatSimNumber(num: string): string {
    if (num.length > 15) return num.slice(0, 15) + '…';
    return num;
  }
</script>

<div class="flex-1 flex flex-col h-full">
  <header class="flex items-center justify-between px-5 py-4 bg-surface border-b border-border shrink-0 flex-wrap gap-3">
    <div class="flex items-center gap-3">
      <h1 class="text-lg font-semibold text-foreground">Ports</h1>
      <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-medium bg-primary/15 text-primary border border-primary/30">
        {portsStore.items.length} port{portsStore.items.length !== 1 ? 's' : ''}
      </span>
    </div>
    <div class="flex items-center gap-2">
      <button
        class="btn-ghost text-xs h-8"
        onclick={refreshPorts}
        disabled={liveStore.on}
        title="Refresh port list"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" class="animate-spin" style="animation-play-state: {liveStore.on ? 'running' : 'paused'}">
          <polyline points="23 4 23 10 17 10"></polyline>
          <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
        </svg>
        <span>Refresh</span>
      </button>
      <button
        class="btn-primary text-xs h-8"
        onclick={() => api.getSimNumbers()}
        disabled={liveStore.on}
        title="Get SIM numbers for checked ports"
      >
        Get SIM Numbers
      </button>
    </div>
  </header>

  <div class="flex-1 overflow-auto p-5">
    {#if portsStore.items.length === 0}
      <div class="flex flex-col items-center justify-center h-full py-20 text-muted-foreground">
        <svg width="48" height="48" viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1.5" class="mb-4 opacity-30">
          <path d="M6 12h36M6 12v24M6 36h36M42 12H6"/>
          <path d="M18 12v12M30 12v12"/>
        </svg>
        <div class="text-sm font-semibold">No ports detected</div>
        <div class="text-xs mt-1 opacity-60 max-w-xs text-center">
          Connect a GSM modem or SIM bank device and click Refresh
        </div>
        <button class="btn-primary mt-4" onclick={refreshPorts}>Refresh Ports</button>
      </div>
    {:else}
      <div class="grid gap-4" style="grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));">
        {#each portsStore.items as port (port.name)}
          <article
            class="card p-4 transition-all duration-200 hover:border-border/80
                   {port.live_error ? 'border-danger/50 bg-danger/5' : ''}
                   {port.live_ready ? 'border-success/30' : ''}
                   {port.checked ? '' : 'opacity-60'}"
          >
            <div class="flex items-start gap-3">
              <div class="w-8 h-8 rounded-lg bg-primary/15 flex items-center justify-center shrink-0">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class="text-primary">
                  <path d="M21 12V7H5V6H3v6h2v6h18v-6h2V7h-4"></path>
                  <path d="M9 12h6"></path>
                  <path d="M9 18h6"></path>
                </svg>
              </div>
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 mb-1">
                  <label class="flex items-center gap-1.5 cursor-pointer">
                    <input
                      type="checkbox"
                      class="w-4 h-4 rounded border-border bg-surface accent-primary cursor-pointer"
                      checked={port.checked}
                      onchange={() => togglePort(port)}
                    />
                    <span class="font-mono text-sm font-semibold text-foreground truncate">{portLabel(port.name)}</span>
                  </label>
                  {#if port.live_ready}
                    <span class="badge badge-success text-[10px]">LIVE</span>
                  {:else if port.live_error}
                    <span class="badge badge-danger text-[10px]">ERROR</span>
                  {:else if liveStore.on && port.checked}
                    <span class="badge badge-warning text-[10px]">CONNECTING</span>
                  {:else if port.checked}
                    <span class="badge badge-muted text-[10px]">READY</span>
                  {:else}
                    <span class="badge badge-muted text-[10px]">DISABLED</span>
                  {/if}
                </div>
                <div class="flex items-center gap-3 text-xs text-muted-foreground mb-2">
                  <span class="flex items-center gap-1 font-mono" title={port.sim_number || 'Unknown'}>
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                      <rect x="2" y="4" width="20" height="16" rx="2"></rect>
                      <line x1="6" y1="8" x2="18" y2="8"></line>
                      <line x1="6" y1="16" x2="18" y2="16"></line>
                    </svg>
                    {formatSimNumber(getSimNumber(port))}
                  </span>
                </div>
              </div>
            </div>

            <div class="mt-3 pt-3 border-t border-border/50">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-4 text-[11px] text-muted-foreground">
                  <span class="flex items-center gap-1 font-mono">
                    <span class="w-1.5 h-1.5 rounded-full bg-primary"></span>
                    {getMessageCount(port)} msg
                  </span>
                  {#if getOtpCount(port) > 0}
                    <span class="flex items-center gap-1 font-mono">
                      <span class="w-1.5 h-1.5 rounded-full bg-otp"></span>
                      {getOtpCount(port)} OTP
                    </span>
                  {/if}
                </div>
                <div class="flex items-center gap-1.5">
                  {#if port.live_error}
                    <span class="badge badge-danger text-[10px]">{port.live_error}</span>
                  {/if}
                </div>
              </div>
            </div>

            {#if port.live_error}
              <div class="mt-3 p-2.5 rounded bg-danger/10 border border-danger/20 text-xs text-danger">
                <div class="flex items-center gap-1.5">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="10"></circle>
                    <line x1="12" y1="8" x2="12" y2="12"></line>
                    <line x1="12" y1="16" x2="12.01" y2="16"></line>
                  </svg>
                  <span class="font-medium">Connection Error</span>
                </div>
                <div class="mt-1 ml-3.5 text-[10px] opacity-80 font-mono">{port.live_error}</div>
              </div>
            {/if}
          </article>
        {/each}
      </div>
    {/if}
  </div>

  <footer class="px-5 py-3 bg-surface border-t border-border shrink-0">
    <div class="flex items-center justify-between text-[11px] font-mono text-muted-foreground">
      <span>Checked: {portsStore.items.filter(p => p.checked).length} / {portsStore.items.length}</span>
      <span>{liveStore.on ? `Live: ${liveStore.readyPorts.length} ready` : 'Live mode off'}</span>
    </div>
  </footer>
</div>