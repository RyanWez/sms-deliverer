<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';
  import { portLabel } from '$lib/utils/port';
  import { fmtFullDateTime } from '$lib/utils/format';

  const item = $derived(messagesStore.activeItem);

  let copiedField = $state<'message' | 'otp' | null>(null);
  let resetTimer: ReturnType<typeof setTimeout> | undefined;

  function flashCopied(field: 'message' | 'otp') {
    copiedField = field;
    clearTimeout(resetTimer);
    resetTimer = setTimeout(() => (copiedField = null), 1500);
  }

  function copyMessage() {
    if (!item) return;
    messagesStore.copyMessage(item.message.text);
    flashCopied('message');
  }

  function copyOtp() {
    if (!item?.otp) return;
    messagesStore.copyOtp(item.otp);
    flashCopied('otp');
  }

  function simNumber(port: string): string {
    return portsStore.find(port)?.sim_number || 'Unknown';
  }
</script>

{#if item}
  <aside class="detail-panel w-[clamp(280px,26vw,400px)]" aria-label="Message details" aria-live="polite">
    <header class="detail-header">
      <span class="detail-title">Message Details</span>
      {#if item.otp}
        <span class="badge badge-otp"><Icon name="zap" size={11} /> OTP</span>
      {/if}
      <button
        class="btn-icon"
        onclick={() => messagesStore.setActive(null)}
        title="Close details (Esc)"
        aria-label="Close details"
      >
        <Icon name="x" size={14} strokeWidth={2} />
      </button>
    </header>

    <dl class="detail-meta">
      <dt class="meta-label">Port</dt>
      <dd>
        <span class="badge badge-primary font-mono" title={item.message.port}>{portLabel(item.message.port)}</span>
      </dd>

      <dt class="meta-label">SIM number</dt>
      <dd class="meta-value font-mono tabular-nums" title={simNumber(item.message.port)}>
        {simNumber(item.message.port)}
      </dd>

      <dt class="meta-label">Sender</dt>
      <dd class="meta-value font-mono">{item.message.from || 'Unknown'}</dd>

      <dt class="meta-label">Received</dt>
      <dd class="meta-value font-mono tabular-nums whitespace-nowrap">{fmtFullDateTime(item.message.received)}</dd>

      <dt class="meta-label">Status</dt>
      <dd><span class="badge badge-muted">{item.message.status || 'Unknown'}</span></dd>

      <dt class="meta-label">OTP</dt>
      <dd>
        {#if item.otp}
          <button
            class="badge-otp text-sm font-mono font-bold tracking-widest cursor-pointer hover:brightness-110 active:scale-[0.97] transition focus:outline-none focus-visible:ring-2 focus-visible:ring-otp/70"
            onclick={copyOtp}
            title="Copy OTP code"
          >
            {copiedField === 'otp' ? 'Copied!' : item.otp}
          </button>
        {:else}
          <span class="text-xs text-muted-foreground">Not detected</span>
        {/if}
      </dd>
    </dl>

    <div class="detail-body-wrap">
      <div class="flex items-center justify-between mb-2">
        <span class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Message content</span>
        <span class="text-[10px] text-muted-foreground/60 font-mono tabular-nums">{item.message.text.length} chars</span>
      </div>
      <p class="message-body">{item.message.text}</p>
    </div>

    <footer class="detail-actions">
      <button
        class="btn-secondary flex-1"
        onclick={copyMessage}
        title="Copy full message text"
      >
        {#if copiedField === 'message'}
          <Icon name="check" size={13} strokeWidth={2.25} />
          Copied
        {:else}
          <Icon name="copy" size={13} />
          Copy Message
        {/if}
      </button>
      {#if item.otp}
        <button
          class="btn-otp flex-1"
          onclick={copyOtp}
          title="Copy OTP code"
        >
          {#if copiedField === 'otp'}
            <Icon name="check" size={13} strokeWidth={2.25} />
            Copied
          {:else}
            Copy OTP
          {/if}
        </button>
      {/if}
    </footer>
  </aside>
{/if}
