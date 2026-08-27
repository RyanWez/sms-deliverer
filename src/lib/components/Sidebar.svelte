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
  class="flex flex-col bg-surface border-r border-border shrink-0 select-none overflow-hidden relative"
  style="width: {navigationStore.sidebarCollapsed ? '56px' : '220px'}; transition: width 300ms cubic-bezier(0.2, 0.0, 0, 1.0); will-change: width;"
>
  <!-- Header -->
  <div
    class="h-12 flex items-center justify-between px-3 border-b border-border relative overflow-hidden shrink-0"
  >
    <span
      class="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider whitespace-nowrap overflow-hidden transition-all duration-300 ease-[cubic-bezier(0.2,0,0,1)]"
      class:opacity-100={!navigationStore.sidebarCollapsed}
      class:max-w-[140px]={!navigationStore.sidebarCollapsed}
      class:opacity-0={navigationStore.sidebarCollapsed}
      class:max-w-0={navigationStore.sidebarCollapsed}
      class:pointer-events-none={navigationStore.sidebarCollapsed}
    >
      SMS Reader
    </span>
    <button
      class="btn-icon shrink-0 transition-transform duration-300 ease-[cubic-bezier(0.2,0,0,1)]"
      onclick={() => navigationStore.toggleSidebar()}
      title={navigationStore.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
      aria-label={navigationStore.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
    >
      <Icon
        name="chevrons-left"
        size={15}
        class="transition-transform duration-300 ease-[cubic-bezier(0.2,0,0,1)] {navigationStore.sidebarCollapsed ? 'rotate-180' : ''}"
      />
    </button>
  </div>

  <!-- Navigation items -->
  <nav class="flex-1 overflow-y-auto overflow-x-hidden py-2" aria-label="Main navigation">
    <ul class="space-y-1 px-2" role="list">
      {#each navItems as item (item.id)}
        <li>
          <button
            role="tab"
            aria-selected={navigationStore.currentSection === item.id}
            aria-controls={`panel-${item.id}`}
            aria-current={navigationStore.currentSection === item.id ? 'page' : undefined}
            class="relative w-full flex items-center px-2.5 h-9 rounded-md text-sm font-medium transition-colors duration-150 overflow-hidden
                   focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70 focus-visible:ring-offset-1 focus-visible:ring-offset-background
                   {navigationStore.currentSection === item.id
                     ? 'bg-primary/10 text-primary'
                     : 'text-muted-foreground hover:bg-elevated hover:text-foreground'}"
            onclick={() => navigationStore.navigate(item.id)}
            title={navigationStore.sidebarCollapsed ? item.label : ''}
          >
            {#if navigationStore.currentSection === item.id}
              <span
                class="absolute left-0 top-1/2 -translate-y-1/2 h-5 w-[3px] rounded-r-full bg-primary"
                aria-hidden="true"
              ></span>
            {/if}
            <div class="w-5 h-5 flex items-center justify-center shrink-0">
              <Icon name={item.icon} size={17} />
            </div>
            <span
              class="truncate whitespace-nowrap text-left transition-all duration-300 ease-[cubic-bezier(0.2,0,0,1)] overflow-hidden"
              class:opacity-100={!navigationStore.sidebarCollapsed}
              class:max-w-[140px]={!navigationStore.sidebarCollapsed}
              class:ml-3={!navigationStore.sidebarCollapsed}
              class:opacity-0={navigationStore.sidebarCollapsed}
              class:max-w-0={navigationStore.sidebarCollapsed}
              class:ml-0={navigationStore.sidebarCollapsed}
            >
              {item.label}
            </span>
          </button>
        </li>
      {/each}
    </ul>

    <!-- Quick Stats -->
    <div
      class="mx-2 mt-4 pt-3 border-t border-border overflow-hidden transition-all duration-300 ease-[cubic-bezier(0.2,0,0,1)] whitespace-nowrap"
      class:opacity-100={!navigationStore.sidebarCollapsed}
      class:max-h-40={!navigationStore.sidebarCollapsed}
      class:opacity-0={navigationStore.sidebarCollapsed}
      class:max-h-0={navigationStore.sidebarCollapsed}
      class:pt-0={navigationStore.sidebarCollapsed}
      class:mt-0={navigationStore.sidebarCollapsed}
      class:border-t-transparent={navigationStore.sidebarCollapsed}
      class:pointer-events-none={navigationStore.sidebarCollapsed}
    >
      <div class="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-2 px-1">
        Quick Stats
      </div>
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
  </nav>

  <!-- Footer -->
  <div
    class="p-3 border-t border-border overflow-hidden transition-all duration-300 ease-[cubic-bezier(0.2,0,0,1)] whitespace-nowrap shrink-0"
    class:opacity-100={!navigationStore.sidebarCollapsed}
    class:max-h-20={!navigationStore.sidebarCollapsed}
    class:opacity-0={navigationStore.sidebarCollapsed}
    class:max-h-0={navigationStore.sidebarCollapsed}
    class:p-0={navigationStore.sidebarCollapsed}
    class:border-t-transparent={navigationStore.sidebarCollapsed}
    class:pointer-events-none={navigationStore.sidebarCollapsed}
  >
    <div class="flex items-center gap-2 text-[11px] font-medium text-muted-foreground">
      <div class="w-5 h-5 rounded-md bg-primary/15 flex items-center justify-center shrink-0">
        <div class="w-2 h-2 rounded-sm bg-primary"></div>
      </div>
      <span>SIM Bank SMS Reader</span>
    </div>
    <div class="mt-1 pl-7 text-[10px] text-muted-foreground/60">v2.0.0</div>
  </div>
</aside>
