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
  import { isTauri } from "$lib/utils/tauri";
  import { portRefreshPeriodMs } from "$lib/utils/port-refresh";

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

  // Port hotplug sweep: the operator physically plugs and pulls sticks, so the
  // list is re-enumerated on a timer instead of only on mount and on Refresh.
  // Lives here rather than in Ports.svelte so a stick plugged in while the
  // operator is reading the Inbox is still picked up.
  let portTimer: ReturnType<typeof setInterval> | undefined;

  function stopPortRefresh() {
    clearInterval(portTimer);
    portTimer = undefined;
  }

  function restartPortRefresh() {
    stopPortRefresh();
    // Nothing hotplugs in browser preview, and the synthetic generator would
    // just reshuffle the demo bank every tick — same reasoning as
    // restartAutoUpdater(), which also only arms inside the desktop shell.
    if (!isTauri()) return;
    const period = portRefreshPeriodMs(settingsStore.general.portRefreshInterval);
    if (period === null) return; // 0 / junk value = turned off
    portTimer = setInterval(() => {
      // refresh_ports rebuilds the shared port state, so it must not land in the
      // middle of an operation that owns the ports. portsBusy() covers live,
      // scan, USSD and delete; detectBusy is not part of it (the SIM sweep above
      // relies on the backend answering "Busy" instead), so it is checked
      // explicitly here rather than widening portsBusy() for every caller.
      if (portsBusy() || liveStore.detectBusy) return;
      void api.refreshPorts("auto");
    }, period);
  }

  // Re-arm whenever the interval changes so editing the setting takes effect
  // immediately, and without stacking a second timer.
  $effect(() => {
    void settingsStore.general.portRefreshInterval;
    restartPortRefresh();
    return stopPortRefresh;
  });

  // The Logs page only exists while Developer Mode is on. Turning it off hides
  // the sidebar entry but would otherwise leave the page mounted — and its
  // 1s log poll running — so send the user back to the Inbox. This lives here
  // rather than in the settings setter so it also covers Reset to Defaults,
  // which flips developer.enabled back off without going through setDeveloper.
  $effect(() => {
    if (
      !settingsStore.developer.enabled &&
      navigationStore.currentSection === "logs"
    ) {
      navigationStore.navigate("inbox");
    }
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
