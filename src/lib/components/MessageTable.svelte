<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { portLabel } from '$lib/utils/port';
  import { fmtDateTime } from '$lib/utils/format';

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

  function inspect(id: number) {
    messagesStore.setActive(id);
  }

  function rowKeydown(id: number, e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      inspect(id);
    }
  }

  function simNumber(port: string): string {
    const p = portsStore.find(port);
    return p?.sim_number || '-';
  }

  let containerEl: HTMLElement | undefined = $state();
  let theadEl: HTMLElement | undefined = $state();
  let ro: ResizeObserver | null = null;
  let rafPending = false;
  let rafId: number | null = null;

  function updateAvail() {
    if (!containerEl) return;
    let h = containerEl.clientHeight;
    if (messagesStore.viewMode === 'Table' && theadEl) h -= theadEl.offsetHeight;
    else h -= 24;
    messagesStore.setAvail(h);
  }

  function scheduleUpdate() {
    if (rafPending) return;
    rafPending = true;
    rafId = requestAnimationFrame(() => {
      rafPending = false;
      rafId = null;
      updateAvail();
    });
  }

  $effect(() => {
    void messagesStore.viewMode;
    if (!containerEl) return;
    updateAvail();
    ro = new ResizeObserver(() => scheduleUpdate());
    ro.observe(containerEl);
    return () => {
      ro?.disconnect();
      ro = null;
      if (rafId !== null) cancelAnimationFrame(rafId);
      rafPending = false;
    };
  });

  $effect(() => {
    void messagesStore.pageRows;
    if (!containerEl) return;
    // Defer height measurement to next frame to avoid layout thrashing
    const id = requestAnimationFrame(() => {
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
    return () => cancelAnimationFrame(id);
  });
</script>

{#snippet emptyState()}
  <div class="empty-state">
    <Icon name="message" size={40} strokeWidth={1.25} class="mb-3 opacity-30" />
    {#if messagesStore.items.length > 0}
      <div class="empty-state-title">No messages match your filters</div>
      <div class="empty-state-hint">{messagesStore.items.length} message(s) loaded — try clearing search or filters</div>
    {:else}
      <div class="empty-state-title">No messages yet</div>
      <div class="empty-state-hint">Press Scan &amp; Read All or turn on Live Mode</div>
    {/if}
  </div>
{/snippet}

{#if messagesStore.viewMode === 'Table'}
  <div bind:this={containerEl} class="table-container flex-1 overflow-auto min-h-0 border-0 rounded-none">
    <table class="table min-w-[720px]">
      <thead bind:this={theadEl}>
        <tr>
          <th class="!w-10">
            <input
              type="checkbox"
              class="checkbox w-3.5 h-3.5"
              checked={allSelected}
              onchange={toggleAll}
              aria-label="Select all visible messages"
            />
          </th>
          {#if settingsStore.appearance.showPortColumn}
            <th class="!w-20">Port</th>
          {/if}
          {#if settingsStore.appearance.showSIMColumn}
            <th class="!w-28">SIM</th>
          {/if}
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
            tabindex="0"
            class:selected={messagesStore.isSelected(item.id)}
            class:active={messagesStore.isActive(item.id)}
            onclick={() => inspect(item.id)}
            onkeydown={(e) => rowKeydown(item.id, e)}
            title="View message details"
          >
            <td onclick={(e) => e.stopPropagation()}>
              <input
                type="checkbox"
                class="checkbox w-3.5 h-3.5"
                checked={messagesStore.isSelected(item.id)}
                onchange={() => messagesStore.toggleSelected(item.id)}
                aria-label="Select message for deletion"
              />
            </td>
            {#if settingsStore.appearance.showPortColumn}
              <td class="font-mono text-xs font-semibold truncate max-w-[110px]" title={item.message.port}>{portLabel(item.message.port)}</td>
            {/if}
            {#if settingsStore.appearance.showSIMColumn}
              <td class="font-mono text-xs text-muted-foreground truncate max-w-[130px]" title={simNumber(item.message.port)}>{simNumber(item.message.port)}</td>
            {/if}
            <td class="text-xs truncate max-w-[120px]" title={item.message.from}>{item.message.from || '-'}</td>
            <td class="font-mono text-xs text-muted-foreground whitespace-nowrap">{fmtDateTime(item.message.received)}</td>
            <td>
              <span class="badge badge-muted truncate max-w-[90px] inline-flex">{item.message.status || '-'}</span>
            </td>
            <td onclick={(e) => e.stopPropagation()}>
              {#if item.otp}
                <button
                  class="badge-otp cursor-pointer font-mono font-bold tracking-wider hover:brightness-110 active:scale-[0.97] transition focus:outline-none focus-visible:ring-2 focus-visible:ring-otp/70 truncate max-w-[110px]"
                  onclick={() => messagesStore.copyOtp(item.otp)}
                  title="Click to copy OTP: {item.otp}"
                >
                  {item.otp}
                </button>
              {:else}
                <span class="text-xs text-muted-foreground">-</span>
              {/if}
            </td>
            <td class="text-xs text-muted-foreground max-w-[280px] lg:max-w-xs" class:truncate={!messagesStore.isExpanded(item.id)} title={item.message.text}>
              {#if item.is_new}
                <span class="w-1.5 h-1.5 rounded-full bg-primary inline-block mr-1.5 animate-pulse-dot shrink-0" title="Unread"></span>
              {/if}
              <span class="break-words">{item.message.text.replace(/\n/g, ' ')}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>

    {#if messagesStore.visible.length === 0}
      {@render emptyState()}
    {/if}
  </div>
{:else}
  <div bind:this={containerEl} class="p-4 overflow-y-auto flex-1 min-h-0 bg-background">
    {#if messagesStore.visible.length === 0}
      {@render emptyState()}
    {:else}
      <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));">
        {#each messagesStore.pageRows as item (item.id)}
          <div
            role="button"
            tabindex="0"
            data-id={item.id}
            class="card p-3.5 text-left transition-colors duration-150 cursor-pointer
                   hover:bg-elevated/50 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
                   {messagesStore.isActive(item.id)
                     ? 'border-primary bg-primary/5'
                     : messagesStore.isSelected(item.id) ? 'border-primary/40' : ''}"
            onclick={() => inspect(item.id)}
            onkeydown={(e) => rowKeydown(item.id, e)}
            title="View message details"
          >
            <div class="flex items-center gap-2 mb-2">
              <input
                type="checkbox"
                class="checkbox w-3.5 h-3.5"
                checked={messagesStore.isSelected(item.id)}
                onclick={(e) => e.stopPropagation()}
                onchange={() => messagesStore.toggleSelected(item.id)}
                onkeydown={(e) => e.stopPropagation()}
                aria-label="Select message for deletion"
              />
              {#if settingsStore.appearance.showPortColumn}
                <span class="font-mono text-[11px] font-bold text-primary" title={item.message.port}>{portLabel(item.message.port)}</span>
              {/if}
              {#if settingsStore.appearance.showSIMColumn}
                <span class="font-mono text-[10px] text-muted-foreground">{simNumber(item.message.port)}</span>
              {/if}
              {#if item.is_new}
                <span class="w-1.5 h-1.5 rounded-full bg-primary animate-pulse-dot" title="Unread"></span>
              {/if}
              <span class="flex-1"></span>
              <span class="font-mono text-[10px] text-muted-foreground tabular-nums">{fmtDateTime(item.message.received)}</span>
            </div>
            {#if item.otp}
              <span
                role="button"
                tabindex="0"
                class="badge-otp text-sm font-mono font-bold tracking-wider mb-2 cursor-pointer hover:brightness-110 inline-block active:scale-[0.97] transition focus:outline-none focus-visible:ring-2 focus-visible:ring-otp/70"
                onclick={(e) => { e.stopPropagation(); messagesStore.copyOtp(item.otp); }}
                onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); messagesStore.copyOtp(item.otp); } }}
                title="Click to copy OTP"
              >
                {item.otp}
              </span>
            {/if}
            <div class="text-xs text-muted-foreground leading-relaxed line-clamp-2">
              {item.message.text.replace(/\n/g, ' ')}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}
