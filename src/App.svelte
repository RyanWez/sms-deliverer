<script lang="ts">
  import { onMount } from "svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import ToastContainer from "$lib/components/ToastContainer.svelte";
  import UpdateDock from "$lib/components/UpdateDock.svelte";
  import Inbox from "$lib/pages/Inbox.svelte";
  import Ports from "$lib/pages/Ports.svelte";
  import Logs from "$lib/pages/Logs.svelte";
  import Settings from "$lib/pages/Settings.svelte";
  import { api } from "$lib/services/api";
  import { restartAutoUpdater } from "$lib/services/updater";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { navigationStore } from "$lib/stores/navigation.svelte";
  import { liveStore } from "$lib/stores/live.svelte";
  import { messagesStore } from "$lib/stores/messages.svelte";

  const SIM_CLEANUP_EVERY = 10 * 60_000;

  /** True while some operation owns the serial ports. */
  function portsBusy() {
    return (
      liveStore.on ||
      liveStore.scanBusy ||
      liveStore.ussdBusy ||
      messagesStore.deleteBusy
    );
  }

  onMount(() => {
    api.init();

    // Inbox sweep: retentionHours of 0 means "keep everything", which
    // purgeExpiredMessages treats as a no-op.
    const purgeTimer = setInterval(() => {
      void api.purgeExpiredMessages(settingsStore.general.retentionHours);
    }, 60_000);

    // SIM sweep: SIM memory holds only a few dozen messages and starts
    // rejecting new SMS once full. Live workers prune the ports they own, so
    // this covers the idle case only.
    const firstSweep = setTimeout(() => {
      if (!portsBusy()) void api.cleanupSimStorage(settingsStore.general.retentionHours);
    }, 30_000);
    const simTimer = setInterval(() => {
      if (portsBusy()) return;
      void api.cleanupSimStorage(settingsStore.general.retentionHours);
    }, SIM_CLEANUP_EVERY);

    return () => {
      clearInterval(purgeTimer);
      clearInterval(simTimer);
      clearTimeout(firstSweep);
    };
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
  <div class="flex-1 flex overflow-hidden min-h-0 app-shell">
    <Sidebar />
    <main class="flex-1 flex flex-col overflow-hidden min-w-0 min-h-0 main-content">
      {#if navigationStore.currentSection === "inbox"}
        <Inbox />
      {:else if navigationStore.currentSection === "ports"}
        <Ports />
      {:else if navigationStore.currentSection === "logs"}
        <Logs />
      {:else if navigationStore.currentSection === "settings"}
        <Settings />
      {/if}
    </main>
  </div>
  <ToastContainer />
  <UpdateDock />
</div>
