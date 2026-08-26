<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { liveStore } from '$lib/stores/live.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { api } from '$lib/services/api';

  const busy = $derived(
    liveStore.on || messagesStore.deleteBusy || liveStore.scanBusy || liveStore.ussdBusy
  );
</script>

<header class="page-header">
  <div class="flex items-center gap-2 sm:gap-3 mr-auto min-w-0 flex-wrap gap-y-1.5">
    <h1 class="page-title shrink-0">Inbox</h1>
    <span class="badge badge-primary font-mono tabular-nums shrink-0">
      {messagesStore.items.length} message{messagesStore.items.length !== 1 ? 's' : ''}
    </span>
    <span
      class="badge {liveStore.on ? 'badge-success' : 'badge-muted'} shrink-0"
      title={liveStore.on ? 'Live monitoring active' : 'Live monitoring off'}
    >
      <span
        class="w-1.5 h-1.5 rounded-full {liveStore.on ? 'bg-success animate-pulse-dot' : 'bg-muted-foreground/50'}"
        aria-hidden="true"></span>
      {liveStore.on ? `Live ${liveStore.readyPorts.length}/${liveStore.totalPorts}` : 'Live off'}
    </span>
  </div>

  <div class="flex items-center gap-1.5 sm:gap-2 flex-wrap w-full sm:w-auto">
    <button
      class="btn-primary"
      disabled={busy}
      onclick={() => api.startScan()}
      title="Read SMS from all checked ports"
    >
      {#if liveStore.scanBusy}
        <Icon name="loader" size={14} class="animate-spin" />
      {/if}
      Scan &amp; Read All
    </button>

    <button
      class="{liveStore.on ? 'btn-danger' : 'btn-success'}"
      disabled={messagesStore.deleteBusy || liveStore.scanBusy || liveStore.ussdBusy}
      onclick={() => (liveStore.on ? api.stopLive() : api.startLive())}
      title={liveStore.on ? 'Stop live monitoring' : 'Start live monitoring on checked ports'}
    >
      <span
        class="w-2 h-2 rounded-full bg-current {liveStore.on ? 'animate-pulse-dot' : ''}"
        aria-hidden="true"></span>
      {liveStore.on ? 'Stop Live' : 'Live Mode'}
    </button>

    <button
      class="btn-secondary"
      disabled={busy}
      onclick={() => api.getSimNumbers()}
      title="Get SIM numbers for checked ports"
    >
      {#if liveStore.ussdBusy}
        <Icon name="loader" size={14} class="animate-spin" />
      {:else}
        <Icon name="sim" size={14} />
      {/if}
      Get SIM Numbers
    </button>

    <div class="hidden sm:block w-px h-5 bg-border mx-0.5" role="separator" aria-hidden="true"></div>

    <button
      class="btn-danger"
      disabled={messagesStore.selectedCount === 0 || busy}
      onclick={() => {
        if (!settingsStore.general.confirmDelete || confirm(`Delete ${messagesStore.selectedCount} message(s)?`)) {
          api.deleteSelected([...messagesStore.selected]);
        }
      }}
      title="Delete selected messages"
    >
      {#if messagesStore.deleteBusy}
        <Icon name="loader" size={14} class="animate-spin" />
      {:else}
        <Icon name="trash" size={14} />
      {/if}
      <span class="hidden sm:inline">Delete Selected</span><span class="sm:hidden">Delete</span> ({messagesStore.selectedCount})
    </button>

    <button
      class="btn-danger-quiet"
      disabled={busy}
      onclick={() => {
        if (!settingsStore.general.confirmDelete || confirm('Delete ALL messages from checked ports?')) {
          api.clearAll();
        }
      }}
      title="Clear all messages"
    >
      Clear All
    </button>
  </div>
</header>
