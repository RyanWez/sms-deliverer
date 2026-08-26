<script lang="ts">
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';
  import { portLabel } from '$lib/utils/port';

  const allSelected = $derived.by(() =>
    messagesStore.visible.length > 0 && messagesStore.visible.every(m => messagesStore.isSelected(m.id))
  );

  function toggleAll() {
    if (allSelected) {
      messagesStore.clearSelection();
    } else {
      messagesStore.selectAll(messagesStore.visible.map(m => m.id));
    }
  }

  function fmtTime(received: string): string {
    if (!received || received === '1970-01-01T00:00:00Z') return '';
    const d = new Date(received);
    const pad = (n: number) => n.toString().padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function simNumber(port: string): string {
    const p = portsStore.find(port);
    return p?.sim_number || '-';
  }

  let hoveredRow = $state<number | null>(null);
  let containerEl: HTMLElement | undefined = $state();
  let theadEl: HTMLElement | undefined = $state();
  let ro: ResizeObserver | null = null;

  function updateAvail() {
    if (!containerEl) return;
    let h = containerEl.clientHeight;
    if (messagesStore.viewMode === 'Table' && theadEl) h -= theadEl.offsetHeight;
    else h -= 24;
    messagesStore.setAvail(h);
  }

  $effect(() => {
    void messagesStore.viewMode;
    if (!containerEl) return;
    updateAvail();
    ro = new ResizeObserver(() => updateAvail());
    ro.observe(containerEl);
    return () => {
      ro?.disconnect();
      ro = null;
    };
  });

  $effect(() => {
    void messagesStore.pageRows;
    if (!containerEl) return;
    const nodes = containerEl.querySelectorAll<HTMLElement>('[data-id]');
    if (nodes.length === 0) return;
    const entries = Array.from(nodes).map(n => ({
      id: Number(n.dataset.id),
      h: n.offsetHeight,
      expanded: messagesStore.isExpanded(Number(n.dataset.id)),
    }));
    messagesStore.reportHeights(entries);
  });

  function toggleText(id: number, e: Event) {
    e.stopPropagation();
    messagesStore.toggleExpanded(id);
  }
</script>

{#if messagesStore.viewMode === 'Table'}
  <div bind:this={containerEl} class="table-container h-full">
    <table class="table">
      <thead bind:this={theadEl}>
        <tr>
          <th class="!w-10">
            <input
              type="checkbox"
              class="w-3.5 h-3.5 rounded border-border bg-surface accent-primary cursor-pointer"
              checked={allSelected}
              onchange={toggleAll}
            />
          </th>
          <th class="!w-20">Port</th>
          <th class="!w-28">SIM</th>
          <th class="!w-24">From</th>
          <th class="!w-36">Received</th>
          <th class="!w-20">Status</th>
          <th class="!w-20">OTP</th>
          <th>Message</th>
        </tr>
      </thead>
      <tbody>
        {#each messagesStore.pageRows as item (item.id)}
          <tr
            data-id={item.id}
            class:selected={messagesStore.isSelected(item.id)}
            class:hovered={hoveredRow === item.id}
            onmouseenter={() => { hoveredRow = item.id; }}
            onmouseleave={() => { hoveredRow = null; }}
          >
            <td>
              <input
                type="checkbox"
                class="w-3.5 h-3.5 rounded border-border bg-surface accent-primary cursor-pointer"
                checked={messagesStore.isSelected(item.id)}
                onchange={() => messagesStore.toggleSelected(item.id)}
              />
            </td>
            <td class="font-mono text-xs font-semibold" title={item.message.port}>{portLabel(item.message.port)}</td>
            <td class="font-mono text-xs text-muted-foreground">{simNumber(item.message.port)}</td>
            <td class="text-xs truncate max-w-[120px]">{item.message.from || '-'}</td>
            <td class="font-mono text-xs text-muted-foreground">{fmtTime(item.message.received)}</td>
            <td>
              <span class="badge-muted text-[10px]">{item.message.status || '-'}</span>
            </td>
            <td>
              {#if item.otp}
                <button
                  class="badge-otp cursor-pointer hover:opacity-80 transition-opacity font-mono font-bold active:scale-95"
                  onclick={() => messagesStore.copyOtp(item.otp)}
                  title="Click to copy OTP"
                >
                  {item.otp}
                </button>
              {:else}
                <span class="text-xs text-muted-foreground">-</span>
              {/if}
            </td>
            <td
              class="text-xs text-muted-foreground max-w-xs cursor-pointer select-none"
              class:truncate={!messagesStore.isExpanded(item.id)}
              onclick={(e) => toggleText(item.id, e)}
              title={messagesStore.isExpanded(item.id) ? 'Click to collapse' : 'Click to expand'}
            >
              {#if item.is_new}
                <span class="w-1.5 h-1.5 rounded-full bg-primary inline-block mr-1.5"></span>
              {/if}
              {#if messagesStore.isExpanded(item.id)}
                <span class="whitespace-pre-wrap break-words text-foreground/90 leading-relaxed">{item.message.text}</span>
              {:else}
                {item.message.text.replace(/\n/g, ' ')}
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>

    {#if messagesStore.visible.length === 0}
      <div class="flex flex-col items-center justify-center h-full py-20 text-muted-foreground">
        <svg width="40" height="40" viewBox="0 0 40 40" fill="none" stroke="currentColor" stroke-width="1.5" class="mb-3 opacity-30">
          <rect x="4" y="8" width="32" height="24" rx="3"/>
          <polyline points="4,10 20,22 36,10"/>
        </svg>
        <div class="text-sm font-semibold">No messages yet</div>
        <div class="text-xs mt-1 opacity-60">Press Scan & Read All or turn on Live Mode</div>
      </div>
    {/if}
  </div>
{:else}
  <div bind:this={containerEl} class="p-3 overflow-y-auto h-full">
    {#if messagesStore.visible.length === 0}
      <div class="flex flex-col items-center justify-center h-full py-20 text-muted-foreground">
        <svg width="40" height="40" viewBox="0 0 40 40" fill="none" stroke="currentColor" stroke-width="1.5" class="mb-3 opacity-30">
          <rect x="4" y="8" width="32" height="24" rx="3"/>
          <polyline points="4,10 20,22 36,10"/>
        </svg>
        <div class="text-sm font-semibold">No messages yet</div>
        <div class="text-xs mt-1 opacity-60">Press Scan & Read All or turn on Live Mode</div>
      </div>
    {:else}
      <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));">
        {#each messagesStore.pageRows as item (item.id)}
          <button
            type="button"
            data-id={item.id}
            class="card p-3 hover:bg-elevated/50 transition-all cursor-pointer text-left"
            class:border-primary={messagesStore.isSelected(item.id)}
            class:border={messagesStore.isSelected(item.id)}
            onclick={() => messagesStore.toggleSelected(item.id)}
          >
            <div class="flex items-center gap-2 mb-2">
              <span class="font-mono text-[11px] font-bold text-primary" title={item.message.port}>{portLabel(item.message.port)}</span>
              <span class="font-mono text-[10px] text-muted-foreground">{simNumber(item.message.port)}</span>
              {#if item.is_new}
                <span class="w-1.5 h-1.5 rounded-full bg-primary"></span>
              {/if}
              <span class="flex-1"></span>
              <span class="font-mono text-[10px] text-muted-foreground">{fmtTime(item.message.received)}</span>
            </div>
            {#if item.otp}
              <span
                role="button"
                tabindex="0"
                class="badge-otp text-sm font-mono font-bold tracking-wider mb-2 cursor-pointer hover:opacity-80 inline-block active:scale-95 transition-transform"
                onclick={(e) => { e.stopPropagation(); messagesStore.copyOtp(item.otp); }}
                onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); messagesStore.copyOtp(item.otp); } }}
              >
                {item.otp}
              </span>
            {/if}
            <div class="text-xs text-muted-foreground leading-relaxed line-clamp-2">
              {item.message.text.replace(/\n/g, ' ')}
            </div>
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}
