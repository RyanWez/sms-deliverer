<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';
  import { portLabel } from '$lib/utils/port';
  import type { QuickFilter } from '$lib/types';

  function setFilter(f: QuickFilter) {
    messagesStore.quickFilter = f;
  }

  const filters: { label: string; value: QuickFilter; badge?: () => number }[] = [
    { label: 'All', value: 'All' },
    { label: 'Has OTP', value: 'Otp', badge: () => messagesStore.otpCount },
    { label: 'Today', value: 'Today' },
  ];
</script>

<div class="flex items-center gap-2 px-5 py-2 bg-surface border-b border-border shrink-0 flex-wrap">
  <div
    class="flex items-center gap-0.5 p-0.5 rounded-md bg-background border border-border"
    role="group"
    aria-label="Quick filters"
  >
    {#each filters as f (f.value)}
      <button
        class="inline-flex items-center gap-1.5 h-7 px-2.5 rounded-[5px] text-xs font-medium transition-colors duration-150
               focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
               {messagesStore.quickFilter === f.value
                 ? 'bg-elevated text-foreground shadow-sm'
                 : 'text-muted-foreground hover:text-foreground'}"
        aria-pressed={messagesStore.quickFilter === f.value}
        onclick={() => setFilter(f.value)}
      >
        {f.label}
        {#if f.badge}
          <span
            class="px-1.5 min-w-[18px] text-center rounded-full text-[10px] leading-4 tabular-nums
                   {messagesStore.quickFilter === f.value ? 'bg-primary/15 text-primary' : 'bg-muted text-muted-foreground'}"
          >
            {f.badge()}
          </span>
        {/if}
      </button>
    {/each}
  </div>

  <select
    class="input w-40 text-xs py-0 pl-2.5 pr-6"
    value={messagesStore.portFilter ?? ''}
    onchange={(e) => {
      const v = (e.target as HTMLSelectElement).value;
      messagesStore.portFilter = v || null;
    }}
    aria-label="Filter by port"
  >
    <option value="">Port: All</option>
    {#each portsStore.items as port (port.name)}
      <option value={port.name}>{port.sim_number || portLabel(port.name)}</option>
    {/each}
  </select>

  <div class="flex-1"></div>

  <div class="relative w-56">
    <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none" aria-hidden="true">
      <Icon name="search" size={13} />
    </span>
    <input
      type="text"
      class="input h-7 text-xs py-0 pl-8 pr-7"
      placeholder="Search messages…"
      value={messagesStore.query}
      oninput={(e) => { messagesStore.query = (e.target as HTMLInputElement).value; }}
      aria-label="Search messages"
    />
    {#if messagesStore.query}
      <button
        class="btn-icon absolute right-1 top-1/2 -translate-y-1/2 w-5 h-5"
        onclick={() => { messagesStore.query = ''; }}
        title="Clear search"
        aria-label="Clear search"
      >
        <Icon name="x" size={11} strokeWidth={2} />
      </button>
    {/if}
  </div>

  <div
    class="flex items-center gap-0.5 p-0.5 rounded-md bg-background border border-border"
    role="group"
    aria-label="View mode"
  >
    <button
      class="w-7 h-6 flex items-center justify-center rounded-[4px] transition-colors duration-150
             focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
             {messagesStore.viewMode === 'Table'
               ? 'bg-elevated text-foreground shadow-sm'
               : 'text-muted-foreground hover:text-foreground'}"
      aria-pressed={messagesStore.viewMode === 'Table'}
      onclick={() => { messagesStore.viewMode = 'Table'; }}
      title="Table view"
    >
      <Icon name="table" size={14} />
    </button>
    <button
      class="w-7 h-6 flex items-center justify-center rounded-[4px] transition-colors duration-150
             focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
             {messagesStore.viewMode === 'Cards'
               ? 'bg-elevated text-foreground shadow-sm'
               : 'text-muted-foreground hover:text-foreground'}"
      aria-pressed={messagesStore.viewMode === 'Cards'}
      onclick={() => { messagesStore.viewMode = 'Cards'; }}
      title="Card view"
    >
      <Icon name="cards" size={14} />
    </button>
  </div>

  <span class="text-xs text-muted-foreground font-mono tabular-nums whitespace-nowrap">
    {messagesStore.visible.length} msgs
  </span>
</div>
