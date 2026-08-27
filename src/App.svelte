<script lang="ts">
  import { onMount } from "svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import ToastContainer from "$lib/components/ToastContainer.svelte";
  import Inbox from "$lib/pages/Inbox.svelte";
  import Ports from "$lib/pages/Ports.svelte";
  import Settings from "$lib/pages/Settings.svelte";
  import { api } from "$lib/services/api";
  import { restartAutoUpdater } from "$lib/services/updater";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { navigationStore } from "$lib/stores/navigation.svelte";

  onMount(() => {
    api.init();
  });

  // Schedule background update checks on launch and re-schedule them whenever
  // the user toggles autoCheck / checkInterval in Settings → Updates.
  $effect(() => {
    void settingsStore.updates.autoCheck;
    void settingsStore.updates.checkInterval;
    restartAutoUpdater();
  });
</script>

<div class="h-full flex flex-col bg-background">
  <TitleBar />
  <div class="flex-1 flex overflow-hidden min-h-0">
    <Sidebar />
    <main class="flex-1 flex flex-col overflow-hidden min-w-0 min-h-0">
      {#if navigationStore.currentSection === "inbox"}
        <Inbox />
      {:else if navigationStore.currentSection === "ports"}
        <Ports />
      {:else if navigationStore.currentSection === "settings"}
        <Settings />
      {/if}
    </main>
  </div>
  <ToastContainer />
</div>
