<script lang="ts">
  import Toolbar from '$lib/components/Toolbar.svelte';
  import FilterBar from '$lib/components/FilterBar.svelte';
  import MessageTable from '$lib/components/MessageTable.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import StatsBar from '$lib/components/StatsBar.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
</script>

<div class="flex-1 flex flex-col overflow-hidden h-full">
  <Toolbar />
  <FilterBar />
  <div class="flex-1 overflow-hidden min-h-0">
    <MessageTable />
  </div>
  <Pagination />
  {#if settingsStore.appearance.compactMode === false}
    <StatsBar />
  {/if}
</div>