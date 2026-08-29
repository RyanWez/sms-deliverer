<script lang="ts">
  /**
   * Floating update dock, visible from any page.
   *
   * A background check can find a release while the user is in the inbox, and a
   * four-second toast is easy to miss — this stays until it is acted on. It
   * stands down while Settings → Updates is open, because that panel shows the
   * same release and the same buttons in full.
   */
  import Icon from "$lib/components/Icon.svelte";
  import { updaterStore } from "$lib/stores/updater.svelte";
  import { restartNow, downloadUpdate } from "$lib/services/updater";
  import { navigationStore } from "$lib/stores/navigation.svelte";
  import { formatBytes, downloadPercent } from "$lib/utils/update-policy";

  const stage = $derived(updaterStore.stage);
  const visible = $derived(
    updaterStore.pending && !updaterStore.panelOpen && !updaterStore.snoozed,
  );
  const percent = $derived(
    downloadPercent(updaterStore.downloadedBytes, updaterStore.totalBytes),
  );

  function openPanel() {
    navigationStore.openSettings("updates");
  }
</script>

{#if visible}
  <div
    class="fixed bottom-20 right-4 z-50 w-[19.5rem] max-w-[calc(100vw-2rem)] rounded-xl
           border border-border bg-surface shadow-2xl overflow-hidden animate-slide-in"
    role="status"
    aria-live="polite"
  >
    <div class="h-0.5 bg-gradient-to-r from-primary/70 via-primary to-primary/70" aria-hidden="true"></div>
    <div class="p-3.5">
      <div class="flex items-center justify-between gap-2 mb-2">
        <span class="text-xs font-semibold text-foreground truncate inline-flex items-center gap-1.5">
          {#if stage === "ready"}
            <Icon name="check-circle" size={13} class="text-success" />
            Update v{updaterStore.version} ready
          {:else if stage === "available"}
            <Icon name="sparkles" size={13} class="text-primary" />
            Version {updaterStore.version} available
          {:else}
            <Icon name="download" size={13} class="text-primary" />
            Updating to v{updaterStore.version}
          {/if}
        </span>
        <span class="text-[11px] font-mono text-muted-foreground shrink-0 tabular-nums">
          {#if stage === "installing"}
            Installing…
          {:else if stage === "ready"}
            {formatBytes(updaterStore.totalBytes)}
          {:else if stage === "downloading" && percent !== null}
            {percent}% · {formatBytes(updaterStore.downloadedBytes)}
          {:else if stage === "downloading"}
            Downloading…
          {/if}
        </span>
        {#if stage === "available" || stage === "ready"}
          <button
            class="btn-icon w-5 h-5 shrink-0"
            title="Hide — it stays in Settings → Updates"
            aria-label="Hide update notice"
            onclick={() => (updaterStore.snoozed = true)}
          >
            <Icon name="x" size={12} />
          </button>
        {/if}
      </div>

      {#if stage === "available"}
        <div class="flex items-center gap-2">
          <button class="btn-secondary flex-1" onclick={openPanel}>What's new</button>
          <button class="btn-primary flex-1" onclick={() => void downloadUpdate()}>
            <Icon name="download" size={12} strokeWidth={2} />
            Update
          </button>
        </div>
      {:else if stage === "ready"}
        <div class="flex items-center gap-2">
          <button class="btn-secondary flex-1" onclick={openPanel}>What's new</button>
          <button class="btn-success flex-1" onclick={() => void restartNow()}>
            <Icon name="power" size={12} strokeWidth={2} />
            Restart
          </button>
        </div>
      {:else}
        <div
          class="h-1.5 w-full overflow-hidden rounded-full bg-border"
          role="progressbar"
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow={percent ?? undefined}
          aria-label="Update download progress"
        >
          {#if stage === "installing" || percent === null}
            <div class="h-full w-1/3 rounded-full bg-primary animate-indeterminate"></div>
          {:else}
            <div
              class="h-full rounded-full bg-primary transition-[width] duration-150"
              style:width="{percent}%"
            ></div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}
