<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import type { IconName } from '$lib/icons';
  import { navigationStore } from '$lib/stores/navigation.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';

  interface NavItem {
    id: 'inbox' | 'ports' | 'settings';
    label: string;
    icon: IconName;
  }

  const navItems: NavItem[] = [
    { id: 'inbox', label: 'Inbox', icon: 'inbox' },
    { id: 'ports', label: 'Ports', icon: 'ports' },
    { id: 'settings', label: 'Settings', icon: 'settings' },
  ];
</script>

<aside
  class="flex flex-col bg-surface border-r border-border shrink-0 transition-all duration-200 relative"
  style="width: {navigationStore.sidebarCollapsed ? '56px' : '220px'}"
>
  <div
    class="flex items-center {navigationStore.sidebarCollapsed
      ? 'justify-center'
      : 'justify-between'} px-3 py-3 border-b border-border"
  >
    {#if !navigationStore.sidebarCollapsed}
      <span class="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">SMS Reader</span>
    {/if}
    <button
      class="btn-icon"
      onclick={() => navigationStore.toggleSidebar()}
      title={navigationStore.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
      aria-label={navigationStore.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
    >
      <Icon name="chevrons-left" size={15} class={navigationStore.sidebarCollapsed ? 'rotate-180 transition-transform' : 'transition-transform'} />
    </button>
  </div>

  <nav class="flex-1 overflow-y-auto py-2" aria-label="Main navigation">
    <ul class="space-y-0.5 px-2" role="list">
      {#each navItems as item (item.id)}
        <li>
          <button
            role="tab"
            aria-selected={navigationStore.currentSection === item.id}
            aria-controls={`panel-${item.id}`}
            aria-current={navigationStore.currentSection === item.id ? 'page' : undefined}
            class="relative w-full flex items-center gap-3 px-3 h-9 rounded-md text-sm font-medium transition-colors duration-150
                   focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70 focus-visible:ring-offset-1 focus-visible:ring-offset-background
                   {navigationStore.currentSection === item.id
                     ? 'bg-primary/10 text-primary'
                     : 'text-muted-foreground hover:bg-elevated hover:text-foreground'}
                   {navigationStore.sidebarCollapsed ? 'justify-center px-0' : ''}"
            onclick={() => navigationStore.navigate(item.id)}
            title={navigationStore.sidebarCollapsed ? item.label : ''}
          >
            {#if navigationStore.currentSection === item.id}
              <span
                class="absolute -left-2 top-1/2 -translate-y-1/2 h-5 w-[3px] rounded-full bg-primary"
                aria-hidden="true"></span>
            {/if}
            <Icon name={item.icon} size={17} />
            {#if !navigationStore.sidebarCollapsed}
              <span class="truncate">{item.label}</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>

    {#if !navigationStore.sidebarCollapsed}
      <div class="mx-2 mt-4 pt-3 border-t border-border">
        <div class="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-2 px-1">Quick Stats</div>
        <div class="space-y-1 text-[11px] text-muted-foreground font-mono">
          <div class="flex items-center justify-between px-1 py-0.5 rounded hover:bg-elevated/50">
            <span>Total Ports</span>
            <span class="text-foreground tabular-nums">{portsStore.items.length}</span>
          </div>
          <div class="flex items-center justify-between px-1 py-0.5 rounded hover:bg-elevated/50">
            <span>Checked</span>
            <span class="text-foreground tabular-nums">{portsStore.items.filter((p) => p.checked).length}</span>
          </div>
        </div>
      </div>
    {/if}
  </nav>

  {#if !navigationStore.sidebarCollapsed}
    <div class="p-3 border-t border-border">
      <div class="flex items-center gap-2 text-[11px] font-medium text-muted-foreground">
        <div class="w-5 h-5 rounded-md bg-primary/15 flex items-center justify-center shrink-0">
          <div class="w-2 h-2 rounded-sm bg-primary"></div>
        </div>
        <span>SIM Bank SMS Reader</span>
      </div>
      <div class="mt-1 pl-7 text-[10px] text-muted-foreground/60">v2.0.0</div>
    </div>
  {/if}
</aside>
