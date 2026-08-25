<script lang="ts">
  import { onMount } from 'svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import NavRail from '$lib/components/NavRail.svelte';
  import Toolbar from '$lib/components/Toolbar.svelte';
  import FilterBar from '$lib/components/FilterBar.svelte';
  import MessageTable from '$lib/components/MessageTable.svelte';
  import StatsBar from '$lib/components/StatsBar.svelte';
  import ToastContainer from '$lib/components/ToastContainer.svelte';
  import { api } from '$lib/services/api';

  onMount(() => {
    api.init();
  });
</script>

<div class="h-full flex flex-col bg-background">
  <TitleBar />
  <div class="flex-1 flex overflow-hidden">
    <NavRail />
    <main class="flex-1 flex flex-col overflow-hidden">
      <Toolbar />
      <FilterBar />
      <div class="flex-1 overflow-hidden">
        <MessageTable />
      </div>
      <StatsBar />
    </main>
  </div>
  <ToastContainer />
</div>