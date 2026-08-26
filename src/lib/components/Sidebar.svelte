<script lang="ts">
  import { navigationStore } from '$lib/stores/navigation.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';

  interface NavItem {
    id: 'inbox' | 'ports' | 'settings';
    label: string;
    icon?: string;
  }

  const navItems: NavItem[] = [
    { id: 'inbox', label: 'Inbox', icon: 'https://img.icons8.com/fluency-systems-regular/96/FFFFFF/mailing.png' },
    { id: 'ports', label: 'Ports', icon: 'https://img.icons8.com/fluency-systems-regular/96/FFFFFF/internet-hub.png' },
    { id: 'settings', label: 'Settings' },
  ];
</script>

<aside
  class="flex flex-col bg-surface border-r border-border shrink-0 transition-all duration-200 relative"
  style="width: {navigationStore.sidebarCollapsed ? '56px' : '220px'}"
>
  <div class="flex items-center justify-between px-3 py-3.5 border-b border-border">
    {#if !navigationStore.sidebarCollapsed}
      <span class="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">SMS READER</span>
    {/if}
    <button
      class="w-7 h-7 flex items-center justify-center rounded hover:bg-elevated text-muted-foreground transition-colors"
      onclick={() => navigationStore.toggleSidebar()}
      title={navigationStore.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
      aria-label={navigationStore.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class={navigationStore.sidebarCollapsed ? 'rotate-180' : ''}>
        <polyline points="9 18 15 12 9 6"></polyline>
      </svg>
    </button>
  </div>

  <nav class="flex-1 overflow-y-auto py-2" aria-label="Main navigation">
    <ul class="space-y-0.5 px-2" role="list">
      {#each navItems as item}
        <li>
          <button
            role="tab"
            aria-selected={navigationStore.currentSection === item.id}
            aria-controls={`panel-${item.id}`}
            class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-150
                   {navigationStore.currentSection === item.id
                     ? 'bg-primary/15 text-primary border border-primary/30'
                     : 'text-muted-foreground hover:bg-elevated/50 hover:text-foreground'}
                   {navigationStore.sidebarCollapsed ? 'justify-center' : 'justify-start'}"
            onclick={() => navigationStore.navigate(item.id)}
            title={navigationStore.sidebarCollapsed ? item.label : ''}
          >
            <span class="shrink-0" aria-hidden="true">
              {#if item.icon}
                <img src={item.icon} alt="" width="18" height="18" class="brightness-0 invert" />
              {:else if item.id === 'settings'}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="12" cy="12" r="3"></circle>
                  <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
                </svg>
              {/if}
            </span>
            {#if !navigationStore.sidebarCollapsed}
              <span class="truncate">{item.label}</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>

    {#if !navigationStore.sidebarCollapsed}
      <div class="px-3 pt-4 border-t border-border">
        <div class="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-2">Quick Stats</div>
        <div class="space-y-1.5 text-[11px] text-muted-foreground font-mono">
          <div class="flex items-center justify-between">
            <span>Total Ports</span>
            <span class="text-foreground">{portsStore.items.length}</span>
          </div>
          <div class="flex items-center justify-between">
            <span>Checked</span>
            <span class="text-foreground">{portsStore.items.filter(p => p.checked).length}</span>
          </div>
        </div>
      </div>
    {/if}
  </nav>

  {#if !navigationStore.sidebarCollapsed}
    <div class="p-3 border-t border-border">
      <div class="flex items-center gap-2 text-[10px] text-muted-foreground">
        <div class="w-5 h-5 rounded bg-primary/20 flex items-center justify-center">
          <div class="w-2.5 h-2.5 rounded-sm bg-primary"></div>
        </div>
        <span>SIM Bank SMS Reader</span>
      </div>
      <div class="mt-1 text-[10px] text-muted-foreground/60">v2.0.0</div>
    </div>
  {/if}
</aside>