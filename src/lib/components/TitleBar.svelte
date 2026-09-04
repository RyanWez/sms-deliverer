<script lang="ts">
  import { isTauri } from '$lib/utils/tauri';

  let maximizing = $state(false);

  async function toggleMaximize() {
    if (!isTauri()) return;
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const appWindow = getCurrentWindow();
      if (maximizing) {
        await appWindow.unmaximize();
        maximizing = false;
      } else {
        await appWindow.maximize();
        maximizing = true;
      }
    } catch (e) {
      console.warn('Window maximize unavailable:', e);
    }
  }

  async function minimize() {
    if (!isTauri()) return;
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().minimize();
    } catch (e) {
      console.warn('Window minimize unavailable:', e);
    }
  }

  async function closeWindow() {
    if (!isTauri()) return;
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    } catch (e) {
      console.warn('Window close unavailable:', e);
    }
  }
</script>

<!--
  The bar carries nothing but the window controls. The app's name was in the
  middle of it and is gone: the window title, the taskbar entry and the sidebar
  footer already say what this program is, and a caption repeating it cost a
  36px strip across the top of every screen.

  `data-tauri-drag-region` stays on the header itself, so the whole empty strip
  is still the drag handle it was when the caption held that attribute.
-->
<header
  data-tauri-drag-region
  class="h-9 relative flex items-center justify-end px-3 bg-elevated border-b border-border select-none shrink-0"
>
  <div class="flex items-center gap-0.5 self-stretch -mr-3 relative z-10">
    <button
      class="w-10 h-full flex items-center justify-center text-muted-foreground hover:bg-border hover:text-foreground transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70 focus-visible:ring-inset"
      onclick={minimize}
      title="Minimize"
      aria-label="Minimize window"
    >
      <svg width="12" height="1" viewBox="0 0 12 1" fill="currentColor" aria-hidden="true"><rect width="12" height="1"/></svg>
    </button>
    <button
      class="w-10 h-full flex items-center justify-center text-muted-foreground hover:bg-border hover:text-foreground transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70 focus-visible:ring-inset"
      onclick={toggleMaximize}
      title={maximizing ? 'Restore' : 'Maximize'}
      aria-label={maximizing ? 'Restore window' : 'Maximize window'}
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
      onclick={closeWindow}
      title="Close"
      aria-label="Close window"
    >
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.2" aria-hidden="true">
        <line x1="1" y1="1" x2="9" y2="9"/><line x1="9" y1="1" x2="1" y2="9"/>
      </svg>
    </button>
  </div>
</header>
