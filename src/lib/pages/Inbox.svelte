<script lang="ts">
  import Toolbar from '$lib/components/Toolbar.svelte';
  import FilterBar from '$lib/components/FilterBar.svelte';
  import MessageTable from '$lib/components/MessageTable.svelte';
  import MessageDetail from '$lib/components/MessageDetail.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import StatsBar from '$lib/components/StatsBar.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { messagesStore } from '$lib/stores/messages.svelte';

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && messagesStore.activeItem !== null) {
      messagesStore.setActive(null);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex-1 flex flex-col overflow-hidden h-full" id="panel-inbox">
  <Toolbar />
  <div class="flex-1 flex overflow-hidden min-h-0">
    <section class="flex-1 flex flex-col overflow-hidden min-w-0" aria-label="Message list">
      <FilterBar />
      <div class="flex-1 overflow-hidden min-h-0">
        <MessageTable />
      </div>
      <Pagination />
    </section>
    <MessageDetail />
  </div>
  {#if settingsStore.appearance.compactMode === false}
    <StatsBar />
  {/if}
</div>
