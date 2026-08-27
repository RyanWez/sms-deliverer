<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { liveStore } from '$lib/stores/live.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { api } from '$lib/services/api';
  import { confirmDialog } from '$lib/services/dialog';

  const busy = $derived(
    liveStore.on || messagesStore.deleteBusy || liveStore.scanBusy || liveStore.ussdBusy
  );
  let exportOpen = $state(false);
</script>

<header class="page-header toolbar">
  <div class="flex items-center gap-2 sm:gap-3 mr-auto min-w-0 flex-wrap gap-y-1.5">
    <h1 class="page-title shrink-0">Inbox</h1>
    <span class="badge badge-primary font-mono tabular-nums shrink-0 min-w-[82px] justify-center">
      {messagesStore.items.length} message{messagesStore.items.length !== 1 ? 's' : ''}
    </span>
    <span
      class="badge {liveStore.on ? 'badge-success' : 'badge-muted'} shrink-0 min-w-[96px] justify-center font-mono tabular-nums"
      title={liveStore.on ? 'Live monitoring active' : 'Live monitoring off'}
    >
      <span
        class="w-1.5 h-1.5 rounded-full shrink-0 aspect-square {liveStore.on ? 'bg-success animate-pulse-dot' : 'bg-muted-foreground/50'}"
        aria-hidden="true"></span>
      <span>{liveStore.on ? `Live ${liveStore.readyPorts.length}/${liveStore.totalPorts}` : 'Live off'}</span>
    </span>
  </div>

  <div class="flex items-center gap-1.5 sm:gap-2 flex-wrap w-full sm:w-auto">
    <button
      class="btn-primary min-w-[130px] justify-center"
      disabled={busy}
      onclick={() => api.startScan()}
      title="Read SMS from all checked ports"
    >
      {#if liveStore.scanBusy}
        <Icon name="loader" size={14} class="animate-spin" />
        <span>Scanning...</span>
      {:else}
        <span>Scan &amp; Read All</span>
      {/if}
    </button>

    <button
      class="{liveStore.on ? 'btn-danger' : 'btn-success'} min-w-[104px] justify-center gap-2"
      disabled={messagesStore.deleteBusy || liveStore.scanBusy || liveStore.ussdBusy}
      onclick={() => (liveStore.on ? api.stopLive() : api.startLive())}
      title={liveStore.on ? 'Stop live monitoring' : 'Start live monitoring on checked ports'}
    >
      <span
        class="w-2 h-2 rounded-full bg-current shrink-0 aspect-square {liveStore.on ? 'animate-pulse-dot' : ''}"
        aria-hidden="true"></span>
      <span>{liveStore.on ? 'Stop Live' : 'Live Mode'}</span>
    </button>

    <button
      class="btn-secondary"
      disabled={busy}
      onclick={() => api.getSimNumbers()}
      title="Get SIM numbers for checked ports"
    >
      {#if liveStore.ussdBusy}
        <Icon name="loader" size={14} class="animate-spin" />
        <span>Getting SIMs...</span>
      {:else}
        <Icon name="sim" size={14} />
        <span>Get SIM Numbers</span>
      {/if}
    </button>

    <div class="hidden sm:block w-px h-5 bg-border mx-0.5" role="separator" aria-hidden="true"></div>

    <button
      class="btn-danger"
      disabled={messagesStore.selectedCount === 0 || busy}
      onclick={async () => {
        if (
          !settingsStore.general.confirmDelete ||
          (await confirmDialog(`Delete ${messagesStore.selectedCount} message(s)?`, {
            title: 'Delete selected',
            kind: 'warning',
            okLabel: 'Delete',
          }))
        ) {
          api.deleteSelected([...messagesStore.selected]);
        }
      }}
      title="Delete selected messages"
    >
      {#if messagesStore.deleteBusy}
        <Icon name="loader" size={14} class="animate-spin" />
        <span>Deleting...</span>
      {:else}
        <Icon name="trash" size={14} />
        <span class="hidden sm:inline">Delete Selected</span><span class="sm:hidden">Delete</span> ({messagesStore.selectedCount})
      {/if}
    </button>

    <button
      class="btn-danger-quiet"
      disabled={busy}
      onclick={async () => {
        if (
          !settingsStore.general.confirmDelete ||
          (await confirmDialog('Delete ALL messages from checked ports?', {
            title: 'Clear all messages',
            kind: 'warning',
            okLabel: 'Clear all',
          }))
        ) {
          api.clearAll();
        }
      }}
      title="Clear all messages"
    >
      Clear All
    </button>

    <div class="relative">
      <button
        class="btn-secondary"
        disabled={messagesStore.visible.length === 0}
        onclick={() => (exportOpen = !exportOpen)}
        title="Export the current filtered messages"
        aria-haspopup="menu"
        aria-expanded={exportOpen}
      >
        <Icon name="download" size={14} />
        <span class="hidden sm:inline">Export</span>
      </button>
      {#if exportOpen}
        <div
          class="absolute right-0 top-full mt-1 z-30 min-w-[9rem] overflow-hidden rounded-lg border border-border bg-popover shadow-xl"
          role="menu"
        >
          <button
            class="block w-full px-3 py-2 text-left text-xs text-foreground hover:bg-accent"
            role="menuitem"
            onclick={() => {
              exportOpen = false;
              api.exportMessages('csv');
            }}
          >
            <Icon name="download" size={12} class="mr-2 text-muted-foreground" />CSV (.csv)
          </button>
          <button
            class="block w-full px-3 py-2 text-left text-xs text-foreground hover:bg-accent"
            role="menuitem"
            onclick={() => {
              exportOpen = false;
              api.exportMessages('json');
            }}
          >
            <Icon name="download" size={12} class="mr-2 text-muted-foreground" />JSON (.json)
          </button>
        </div>
      {/if}
    </div>
  </div>
</header>
