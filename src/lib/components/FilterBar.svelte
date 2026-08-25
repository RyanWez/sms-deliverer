<script lang="ts">
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';
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

<div class="flex items-center gap-2 px-4 py-2 bg-surface border-b border-border shrink-0">
  {#each filters as f}
    <button
      class="px-3 py-1 rounded-full text-xs font-medium transition-all duration-150 border"
      class:bg-primary={messagesStore.quickFilter === f.value}
      class:text-primary-foreground={messagesStore.quickFilter === f.value}
      class:border-primary={messagesStore.quickFilter === f.value}
      class:bg-transparent={messagesStore.quickFilter !== f.value}
      class:text-muted-foreground={messagesStore.quickFilter !== f.value}
      class:border-border={messagesStore.quickFilter !== f.value}
      class:hover:bg-elevated={messagesStore.quickFilter !== f.value}
      onclick={() => setFilter(f.value)}
    >
      {f.label}
      {#if f.badge}
        <span class="ml-1 px-1.5 py-0 rounded-full text-[10px] {messagesStore.quickFilter === f.value ? 'bg-primary/20' : 'bg-muted'}">
          {f.badge()}
        </span>
      {/if}
    </button>
  {/each}

  <select
    class="input w-36 h-7 text-xs !py-0 !px-2"
    value={messagesStore.portFilter ?? ''}
    onchange={(e) => {
      const v = (e.target as HTMLSelectElement).value;
      messagesStore.portFilter = v || null;
    }}
  >
    <option value="">Port: All</option>
    {#each portsStore.items as port}
      <option value={port.name}>{port.sim_number || port.name}</option>
    {/each}
  </select>

  <div class="flex-1"></div>

  <div class="relative w-52">
    <input
      type="text"
      class="input h-7 text-xs !pr-7"
      placeholder="Search..."
      value={messagesStore.query}
      oninput={(e) => { messagesStore.query = (e.target as HTMLInputElement).value; }}
    />
    {#if messagesStore.query}
      <button
        class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
        onclick={() => { messagesStore.query = ''; }}
        title="Clear search"
      >
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="3" y1="3" x2="9" y2="9"/><line x1="9" y1="3" x2="3" y2="9"/>
        </svg>
      </button>
    {/if}
  </div>

  <div class="flex border border-border rounded-md overflow-hidden h-7">
    <button
      class="w-8 h-full flex items-center justify-center transition-colors {messagesStore.viewMode === 'Table' ? 'bg-primary/10 text-primary' : 'text-muted-foreground'}"
      onclick={() => { messagesStore.viewMode = 'Table'; }}
      title="Table view"
    >
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
        <line x1="1" y1="3" x2="13" y2="3"/><line x1="1" y1="7" x2="13" y2="7"/><line x1="1" y1="11" x2="13" y2="11"/>
      </svg>
    </button>
    <button
      class="w-8 h-full flex items-center justify-center transition-colors {messagesStore.viewMode === 'Cards' ? 'bg-primary/10 text-primary' : 'text-muted-foreground'}"
      onclick={() => { messagesStore.viewMode = 'Cards'; }}
      title="Card view"
    >
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
        <rect x="1" y="1" width="5" height="5" rx="1"/><rect x="8" y="1" width="5" height="5" rx="1"/>
        <rect x="1" y="8" width="5" height="5" rx="1"/><rect x="8" y="8" width="5" height="5" rx="1"/>
      </svg>
    </button>
  </div>

  <span class="text-[11px] text-muted-foreground font-mono">
    {messagesStore.visible.length} msgs
  </span>
</div>
