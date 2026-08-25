<script lang="ts">
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { liveStore } from '$lib/stores/live.svelte';
  import { api } from '$lib/services/api';

  const busy = $derived(
    liveStore.on || messagesStore.deleteBusy
  );
</script>

<div class="flex items-center gap-2 px-4 py-2.5 bg-surface border-b border-border shrink-0 flex-wrap">
  <button
    class="btn-primary text-xs h-8"
    disabled={busy}
    onclick={() => api.startScan()}
  >
    Scan & Read All
  </button>

  <button
    class="text-xs h-8 px-4 py-2 rounded-md font-medium transition-all duration-150 inline-flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed {liveStore.on ? 'bg-danger text-danger-foreground hover:bg-danger/90' : 'bg-success text-success-foreground hover:bg-success/90'}"
    disabled={messagesStore.deleteBusy}
    onclick={() => liveStore.on ? api.stopLive() : api.startLive()}
  >
    <span class="w-2 h-2 rounded-full {liveStore.on ? 'bg-white' : 'bg-white/60'}"></span>
    {liveStore.on ? 'Stop Live' : 'Live Mode'}
  </button>

  <button
    class="btn-warning text-xs h-8"
    disabled={busy}
    onclick={() => api.getSimNumbers()}
  >
    Get SIM Numbers
  </button>

  <div class="w-px h-5 bg-border mx-1"></div>

  <button
    class="btn-danger text-xs h-8"
    disabled={messagesStore.selectedCount === 0 || busy}
    onclick={() => {
      if (confirm(`Delete ${messagesStore.selectedCount} message(s)?`)) {
        api.deleteSelected([...messagesStore.selected]);
      }
    }}
  >
    Delete Selected ({messagesStore.selectedCount})
  </button>

  <button
    class="btn-danger text-xs h-8"
    disabled={busy}
    onclick={() => {
      if (confirm('Delete ALL messages from checked ports?')) {
        api.clearAll();
      }
    }}
  >
    Clear All
  </button>
</div>
