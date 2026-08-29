<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { liveStore } from '$lib/stores/live.svelte';
  import { portLabel, portStatus } from '$lib/utils/port';

  let { name, ontoggle, onclose }: { name: string; ontoggle: () => void; onclose: () => void } = $props();

  const port = $derived(portsStore.find(name) ?? null);
  const status = $derived(port ? portStatus(port, liveStore.on) : null);

  const msgCount = $derived.by(() => {
    if (!port) return 0;
    return messagesStore.getMessageCountForPort(port.name);
  });
  const otpCount = $derived.by(() => {
    if (!port) return 0;
    return messagesStore.getOtpCountForPort(port.name);
  });

  type CopyField = 'name' | 'sim' | 'iccid';

  let copiedField = $state<CopyField | null>(null);
  let resetTimer: ReturnType<typeof setTimeout> | undefined;

  function flashCopied(field: CopyField) {
    copiedField = field;
    clearTimeout(resetTimer);
    resetTimer = setTimeout(() => (copiedField = null), 1500);
  }

  function copyValue(field: CopyField, value: string) {
    navigator.clipboard.writeText(value);
    flashCopied(field);
  }
</script>

{#if port && status}
  <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
  <aside
    class="detail-panel w-[clamp(300px,32%,400px)] max-w-[48%] min-w-[300px]"
    aria-label={`Port details for ${portLabel(port.name)}`}
  >
    <header class="detail-header">
      <span class="font-mono text-sm font-semibold text-foreground">{portLabel(port.name)}</span>
      <span class="badge {status.badge}">{status.label}</span>
      <span class="flex-1"></span>
      <button
        class="btn-icon"
        onclick={onclose}
        aria-label="Close port details"
        title="Close details (Esc)"
      >
        <Icon name="x" size={14} strokeWidth={2} />
      </button>
    </header>

    <dl class="detail-meta">
      <dt class="meta-label">Device path</dt>
      <dd class="meta-value flex items-center justify-end gap-1 min-w-0">
        <span class="font-mono truncate" title={port.name}>{port.name}</span>
        <button
          class="btn-icon w-6 h-6"
          onclick={() => copyValue('name', port.name)}
          title="Copy device path"
          aria-label="Copy device path"
        >
          <Icon name={copiedField === 'name' ? 'check' : 'copy'} size={12} strokeWidth={2} />
        </button>
      </dd>

      <dt class="meta-label">SIM number</dt>
      <dd class="meta-value flex items-center justify-end gap-1 min-w-0">
        <span class="font-mono tabular-nums truncate" title={port.sim_number || 'Unknown'}>
          {port.sim_number || 'Unknown'}
        </span>
        {#if port.sim_number}
          <button
            class="btn-icon w-6 h-6"
            onclick={() => copyValue('sim', port.sim_number)}
            title="Copy SIM number"
            aria-label="Copy SIM number"
          >
            <Icon name={copiedField === 'sim' ? 'check' : 'copy'} size={12} strokeWidth={2} />
          </button>
        {/if}
      </dd>

      <dt class="meta-label">SIM card (ICCID)</dt>
      <dd class="meta-value flex items-center justify-end gap-1 min-w-0">
        {#if port.iccid}
          <span class="font-mono truncate" title={port.iccid}>{port.iccid}</span>
          <button
            class="btn-icon w-6 h-6"
            onclick={() => copyValue('iccid', port.iccid ?? '')}
            title="Copy ICCID"
            aria-label="Copy ICCID"
          >
            <Icon name={copiedField === 'iccid' ? 'check' : 'copy'} size={12} strokeWidth={2} />
          </button>
        {:else}
          <span class="text-muted-foreground/60 italic">Not read yet</span>
        {/if}
      </dd>

      <dt class="meta-label">Modem</dt>
      <dd class="meta-value">
        {#if port.alive === true}
          <span class="text-success">Answering</span>
        {:else if port.alive === false}
          <span class="text-muted-foreground">No modem (empty slot)</span>
        {:else}
          <span class="text-muted-foreground/60 italic">Not probed yet</span>
        {/if}
      </dd>

      <dt class="meta-label">Messages</dt>
      <dd class="meta-value font-mono tabular-nums">{msgCount}</dd>

      <dt class="meta-label">OTP codes</dt>
      <dd class="meta-value font-mono tabular-nums">{otpCount}</dd>
    </dl>

    {#if port.alive === false}
      <div class="mx-4 mt-3 p-2.5 rounded-md bg-elevated border border-border text-xs text-muted-foreground animate-fade-in">
        <div class="flex items-center gap-1.5 font-medium">
          <Icon name="info" size={13} strokeWidth={2} />
          No modem on this port
        </div>
        <div class="mt-1 ml-[22px] text-[10px] opacity-80">
          Nothing answered the AT probe. Scan and live mode skip it, so it costs no time.
        </div>
      </div>
    {:else if port.live_error}
      <div class="mx-4 mt-3 p-2.5 rounded-md bg-danger/10 border border-danger/25 text-xs text-danger animate-fade-in">
        <div class="flex items-center gap-1.5 font-medium">
          <Icon name="alert-circle" size={13} strokeWidth={2} />
          Connection Error
        </div>
        <div class="mt-1 ml-[22px] text-[10px] opacity-80 font-mono break-all">{port.live_error}</div>
      </div>
    {/if}

    <div class="flex-1"></div>

    <footer class="detail-actions">
      <button
        class="{port.checked ? 'btn-primary' : 'btn-secondary'} w-full"
        onclick={ontoggle}
        title={port.checked ? 'Exclude this port from scan and live mode' : 'Include this port in scan and live mode'}
      >
        {#if port.checked}
          <Icon name="check" size={13} strokeWidth={2.25} />
          Included in scan
        {:else}
          Include in scan
        {/if}
      </button>
    </footer>
  </aside>
{/if}
