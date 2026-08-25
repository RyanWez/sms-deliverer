<script lang="ts">
  import { portsStore } from '$lib/stores/ports.svelte';
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { api } from '$lib/services/api';

  let collapsed = $state(false);

  const portCount = $derived(() => portsStore.items.length);
  const checkedCount = $derived(() => portsStore.items.filter(p => p.checked).length);

  function toggleCollapsed() { collapsed = !collapsed; }
</script>

<aside class="flex flex-col bg-surface border-r border-border shrink-0 transition-all duration-200"
  style="width: {collapsed ? '52px' : '180px'}">
  <div class="flex items-center justify-between px-3 py-3 border-b border-border">
    {#if !collapsed}
      <span class="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">Ports</span>
    {/if}
    <button
      class="w-6 h-6 flex items-center justify-center rounded hover:bg-elevated text-muted-foreground transition-colors"
      onclick={toggleCollapsed}
      title={collapsed ? 'Expand' : 'Collapse'}
    >
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
        {#if collapsed}
          <polyline points="4,2 8,6 4,10"/>
        {:else}
          <polyline points="8,2 4,6 8,10"/>
        {/if}
      </svg>
    </button>
  </div>

  <div class="flex-1 overflow-y-auto py-1">
    {#if !collapsed}
      {#each portsStore.items as port (port.name)}
        <label class="flex items-center gap-2 px-3 py-1.5 cursor-pointer hover:bg-elevated/50 group transition-colors">
          <input
            type="checkbox"
            class="w-3.5 h-3.5 rounded border-border bg-surface accent-primary cursor-pointer"
            checked={port.checked}
            onchange={(e) => {
              const target = e.target as HTMLInputElement;
              api.togglePortChecked(port.name, target.checked);
              portsStore.updatePort(port.name, { checked: target.checked });
            }}
          />
          <span class="text-xs text-foreground truncate font-mono group-hover:text-primary transition-colors">
            {port.sim_number || port.name}
          </span>
        </label>
      {/each}
    {:else}
      {#each portsStore.items as port (port.name)}
        <div class="flex justify-center py-1">
          <input
            type="checkbox"
            class="w-3.5 h-3.5 rounded border-border bg-surface accent-primary cursor-pointer"
            checked={port.checked}
            onchange={(e) => {
              const target = e.target as HTMLInputElement;
              api.togglePortChecked(port.name, target.checked);
              portsStore.updatePort(port.name, { checked: target.checked });
            }}
            title={port.name}
          />
        </div>
      {/each}
    {/if}
  </div>

  {#if !collapsed}
    <div class="px-3 py-2 border-t border-border text-[10px] text-muted-foreground font-mono">
      {checkedCount}/{portCount} selected
    </div>
  {/if}
</aside>
