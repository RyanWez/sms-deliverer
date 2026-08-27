<script lang="ts">
  import Toolbar from '$lib/components/Toolbar.svelte';
  import FilterBar from '$lib/components/FilterBar.svelte';
  import MessageTable from '$lib/components/MessageTable.svelte';
  import MessageDetail from '$lib/components/MessageDetail.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import { messagesStore } from '$lib/stores/messages.svelte';

  let lastActiveTrigger: HTMLElement | null = $state(null);

  // Track active item to capture trigger for focus return
  let prevActiveId: number | null = $state(null);
  $effect(() => {
    const cur = messagesStore.activeItem?.id ?? null;
    if (cur !== null && prevActiveId === null) {
      lastActiveTrigger = document.activeElement as HTMLElement | null;
    }
    prevActiveId = cur;
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && messagesStore.activeItem !== null) {
      messagesStore.setActive(null);
      // return focus to last trigger next frame
      requestAnimationFrame(() => lastActiveTrigger?.focus());
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex-1 flex flex-col overflow-hidden h-full min-w-0" id="panel-inbox">
  <Toolbar />
  <div class="flex-1 flex overflow-hidden min-h-0 min-w-0 relative">
    <section class="flex-1 flex flex-col overflow-hidden min-w-0 min-h-0" aria-label="Message list">
      <FilterBar />
      <MessageTable />
      <Pagination />
    </section>
    <MessageDetail oncloseFocusReturn={() => lastActiveTrigger?.focus()} />
  </div>
</div>
