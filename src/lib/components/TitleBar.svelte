<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  const appWindow = getCurrentWindow();
  let maximizing = $state(false);

  async function toggleMaximize() {
    if (maximizing) {
      await appWindow.unmaximize();
      maximizing = false;
    } else {
      await appWindow.maximize();
      maximizing = true;
    }
  }
</script>

<header
  data-tauri-drag-region
  class="h-9 relative flex items-center justify-end px-3 bg-elevated border-b border-border select-none shrink-0"
>
  <div class="absolute inset-0 flex items-center justify-center pointer-events-none">
    <div class="flex items-center gap-2 pointer-events-auto" data-tauri-drag-region>
      <div class="w-4 h-4 rounded bg-primary/20 flex items-center justify-center pointer-events-none">
        <div class="w-2 h-2 rounded-sm bg-primary pointer-events-none"></div>
      </div>
      <span class="text-xs font-bold text-foreground/80 tracking-widest pointer-events-none">SIM BANK SMS READER</span>
    </div>
  </div>

  <div class="flex items-center gap-0.5 self-stretch -mr-3 relative z-10">
    <button
      class="w-10 h-full flex items-center justify-center text-muted-foreground hover:bg-elevated hover:text-foreground transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70 focus-visible:ring-inset"
      onclick={() => appWindow.minimize()}
      title="Minimize"
    >
      <svg width="12" height="1" viewBox="0 0 12 1" fill="currentColor"><rect width="12" height="1"/></svg>
    </button>
    <button
      class="w-10 h-full flex items-center justify-center text-muted-foreground hover:bg-elevated hover:text-foreground transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70 focus-visible:ring-inset"
      onclick={toggleMaximize}
      title={maximizing ? 'Restore' : 'Maximize'}
    >
      {#if maximizing}
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1">
          <rect x="2" y="0" width="8" height="8" rx="1"/><rect x="0" y="2" width="8" height="8" rx="1"/>
        </svg>
      {:else}
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1">
          <rect x="0.5" y="0.5" width="9" height="9" rx="1.5"/>
        </svg>
      {/if}
    </button>
    <button
      class="w-10 h-full flex items-center justify-center text-muted-foreground hover:bg-danger hover:text-white transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-danger/70 focus-visible:ring-inset"
      onclick={() => appWindow.close()}
      title="Close"
    >
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.2">
        <line x1="1" y1="1" x2="9" y2="9"/><line x1="9" y1="1" x2="1" y2="9"/>
      </svg>
    </button>
  </div>
</header>
