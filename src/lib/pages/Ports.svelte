<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { portsStore } from "$lib/stores/ports.svelte";
  import { messagesStore } from "$lib/stores/messages.svelte";
  import { liveStore } from "$lib/stores/live.svelte";
  import { api } from "$lib/services/api";
  import { portLabel } from "$lib/utils/port";
  import type { PortInfo } from "$lib/types";

  let refreshing = $state(false);

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
    if (refreshing || liveStore.on) return;
    refreshing = true;
    try {
      await api.refreshPorts();
    } finally {
      refreshing = false;
    }
  }

  function togglePort(port: PortInfo) {
    api.togglePortChecked(port.name, !port.checked);
    portsStore.updatePort(port.name, { checked: !port.checked });
  }

  function getSimNumber(port: PortInfo) {
    if (port.sim_number) return port.sim_number;
    return "Unknown";
  }

  function getMessageCount(port: PortInfo) {
    return portMessageCounts.get(port.name) ?? 0;
  }

  function getOtpCount(port: PortInfo) {
    return portOtpCounts.get(port.name) ?? 0;
  }

  function formatSimNumber(num: string): string {
    if (num.length > 15) return num.slice(0, 15) + "…";
    return num;
  }
</script>

<div class="flex-1 flex flex-col h-full overflow-hidden" id="panel-ports">
  <header class="page-header">
    <div class="flex items-center gap-3 mr-auto">
      <h1 class="page-title">Ports</h1>
      <span class="badge badge-primary font-mono tabular-nums">
        {portsStore.items.length} port{portsStore.items.length !== 1 ? "s" : ""}
      </span>
    </div>
    <div class="flex items-center gap-2">
      <button
        class="btn-secondary"
        onclick={refreshPorts}
        disabled={liveStore.on || refreshing}
        title="Refresh port list"
      >
        <Icon
          name={refreshing ? "loader" : "refresh"}
          size={14}
          class={refreshing ? "animate-spin" : ""}
        />
        {refreshing ? "Refreshing…" : "Refresh"}
      </button>
      <button
        class="btn-primary"
        onclick={() => api.getSimNumbers()}
        disabled={liveStore.on}
        title="Get SIM numbers for checked ports"
      >
        <Icon name="sim" size={14} />
        Get SIM Numbers
      </button>
    </div>
  </header>

  <div class="flex-1 overflow-auto p-5 bg-background">
    {#if portsStore.items.length === 0}
      <div class="empty-state">
        <Icon
          name="ports"
          size={48}
          strokeWidth={1.25}
          class="mb-4 opacity-30"
        />
        <div class="empty-state-title">No ports detected</div>
        <div class="empty-state-hint">
          Connect a GSM modem or SIM bank device and click Refresh
        </div>
        <button class="btn-primary mt-4" onclick={refreshPorts}
          >Refresh Ports</button
        >
      </div>
    {:else}
      <div
        class="grid gap-3"
        style="grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));"
      >
        {#each portsStore.items as port (port.name)}
          <article
            class="card p-4 transition-colors duration-150 hover:border-muted-foreground/40
                   {port.live_error ? 'border-danger/50 bg-danger/[0.04]' : ''}
                   {port.live_ready && !port.live_error
              ? 'border-success/30'
              : ''}
                   {port.checked ? '' : 'opacity-60'}"
          >
            <div class="flex items-start gap-3">
              <div
                class="w-8 h-8 rounded-md flex items-center justify-center shrink-0
                       {port.live_error
                  ? 'bg-danger/10 text-danger'
                  : port.live_ready
                    ? 'bg-success/10 text-success'
                    : 'bg-primary/10 text-primary'}"
                aria-hidden="true"
              >
                <Icon name="sim" size={17} />
              </div>
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 mb-1 min-w-0">
                  <label
                    class="flex items-center gap-2 cursor-pointer group min-w-0"
                  >
                    <input
                      type="checkbox"
                      class="checkbox w-4 h-4"
                      checked={port.checked}
                      onchange={() => togglePort(port)}
                      aria-label={`Include ${portLabel(port.name)} in scan`}
                    />
                    <span
                      class="font-mono text-sm font-semibold text-foreground truncate group-hover:text-primary transition-colors"
                      >{portLabel(port.name)}</span
                    >
                  </label>
                  {#if port.live_ready && !port.live_error}
                    <span class="badge badge-success"
                      ><span
                        class="w-1.5 h-1.5 rounded-full bg-current"
                        aria-hidden="true"
                      ></span>LIVE</span
                    >
                  {:else if port.live_error}
                    <span class="badge badge-danger">ERROR</span>
                  {:else if liveStore.on && port.checked}
                    <span class="badge badge-warning"
                      ><span
                        class="w-1.5 h-1.5 rounded-full bg-current animate-pulse-dot"
                        aria-hidden="true"
                      ></span>CONNECTING</span
                    >
                  {:else if port.checked}
                    <span class="badge badge-muted">READY</span>
                  {:else}
                    <span class="badge badge-muted">DISABLED</span>
                  {/if}
                </div>
                <div
                  class="flex items-center gap-3 text-xs text-muted-foreground mb-2"
                >
                  <span
                    class="flex items-center gap-1.5 font-mono truncate"
                    title={port.sim_number || "Unknown"}
                  >
                    <Icon name="sim" size={12} />
                    {formatSimNumber(getSimNumber(port))}
                  </span>
                </div>
              </div>
            </div>

            <div class="mt-3 pt-3 border-t border-border/50">
              <div class="flex items-center justify-between gap-2">
                <div
                  class="flex items-center gap-4 text-[11px] font-mono text-muted-foreground tabular-nums"
                >
                  <span class="flex items-center gap-1.5">
                    <span
                      class="w-1.5 h-1.5 rounded-full bg-primary"
                      aria-hidden="true"
                    ></span>
                    {getMessageCount(port)} msg
                  </span>
                  {#if getOtpCount(port) > 0}
                    <span class="flex items-center gap-1.5">
                      <span
                        class="w-1.5 h-1.5 rounded-full bg-otp"
                        aria-hidden="true"
                      ></span>
                      {getOtpCount(port)} OTP
                    </span>
                  {/if}
                </div>
              </div>
            </div>

            {#if port.live_error}
              <div
                class="mt-3 p-2.5 rounded-md bg-danger/10 border border-danger/25 text-xs text-danger animate-fade-in"
              >
                <div class="flex items-center gap-1.5 font-medium">
                  <Icon name="alert-circle" size={13} strokeWidth={2} />
                  Connection Error
                </div>
                <div
                  class="mt-1 ml-[22px] text-[10px] opacity-80 font-mono break-all"
                >
                  {port.live_error}
                </div>
              </div>
            {/if}
          </article>
        {/each}
      </div>
    {/if}
  </div>

  <footer class="page-footer font-mono">
    <span class="tabular-nums"
      >Checked: {portsStore.items.filter((p) => p.checked).length} / {portsStore
        .items.length}</span
    >
    <span class="flex items-center gap-1.5 tabular-nums">
      <span
        class="w-1.5 h-1.5 rounded-full {liveStore.on
          ? 'bg-success animate-pulse-dot'
          : 'bg-muted-foreground/50'}"
        aria-hidden="true"
      ></span>
      {liveStore.on
        ? `Live: ${liveStore.readyPorts.length} ready`
        : "Live mode off"}
    </span>
  </footer>
</div>
