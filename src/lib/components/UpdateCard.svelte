<script lang="ts">
  /**
   * Settings → Updates panel body: the release-notes box the user asked for,
   * plus the download / restart flow.
   *
   * Release notes come from a remote endpoint, so every line is rendered as
   * escaped text through normal Svelte interpolation. Nothing here uses
   * `{@html}` — a release body must never be able to inject markup.
   */
  import { onMount } from "svelte";
  import Icon from "$lib/components/Icon.svelte";
  import type { IconName } from "$lib/icons";
  import { updaterStore } from "$lib/stores/updater.svelte";
  import {
    runUpdateCheck,
    downloadUpdate,
    restartNow,
    dismissUpdate,
  } from "$lib/services/updater";
  import { formatReleaseDate, countNoteItems, type NoteKind } from "$lib/utils/release-notes";
  import {
    cooldownRemaining,
    cooldownSeconds,
    formatBytes,
    downloadPercent,
    MANUAL_COOLDOWN_MS,
  } from "$lib/utils/update-policy";

  const s = updaterStore;
  const releasedOn = $derived(formatReleaseDate(updaterStore.releaseDate));
  const changeCount = $derived(countNoteItems(updaterStore.notes));
  const percent = $derived(
    downloadPercent(updaterStore.downloadedBytes, updaterStore.totalBytes),
  );

  // `now` only ticks while a cooldown is actually running, so the settings page
  // is not re-rendering on a timer for the rest of the session. The effect keys
  // off the check timestamp rather than the remaining time, or it would tear
  // down and rebuild its own interval on every tick.
  let now = $state(Date.now());
  const coolMs = $derived(cooldownRemaining(updaterStore.lastCheckedAt, now));
  const coolLeft = $derived(cooldownSeconds(coolMs));

  $effect(() => {
    const at = updaterStore.lastCheckedAt;
    if (!at) return;
    const deadline = at + MANUAL_COOLDOWN_MS;
    if (Date.now() >= deadline) return;
    const timer = setInterval(() => {
      now = Date.now();
      if (now >= deadline) clearInterval(timer);
    }, 500);
    return () => clearInterval(timer);
  });

  // Tells the floating progress card to stand down while this panel is open.
  onMount(() => {
    updaterStore.panelOpen = true;
    return () => {
      updaterStore.panelOpen = false;
    };
  });

  const SECTION_ICON: Record<NoteKind, IconName> = {
    feature: "sparkles",
    fix: "bug",
    other: "info",
  };
  const SECTION_TONE: Record<NoteKind, string> = {
    feature: "text-primary",
    fix: "text-success",
    other: "text-muted-foreground",
  };

  const busy = $derived(
    s.stage === "checking" || s.stage === "downloading" || s.stage === "installing",
  );
</script>

<div class="divide-y divide-border/50 border-t border-border/50">
  <!-- Check row: hidden once a specific version is waiting, so the only
       button on screen is the one that moves the update forward. -->
  {#if !s.pending}
    <div class="px-4 py-3.5 flex items-center justify-between gap-4 flex-wrap sm:flex-nowrap">
      <div class="min-w-0 flex-1">
        <span class="block text-sm font-medium text-foreground">Check for Updates Now</span>
        <p class="text-[11px] text-muted-foreground mt-0.5 leading-relaxed">
          {#if s.stage === "uptodate"}
            You are on the latest version{s.currentVersion ? ` (v${s.currentVersion})` : ""}.
          {:else if coolMs > 0}
            Just checked — the endpoint is queried at most once a minute.
          {:else}
            Query the release endpoint for a newer version
          {/if}
        </p>
      </div>
      <div class="flex items-center gap-2 shrink-0">
        {#if s.stage === "uptodate"}
          <span class="badge-success" aria-live="polite">
            <Icon name="check" size={11} strokeWidth={2.5} />
            Up to date
          </span>
        {/if}
        <button
          class="btn-primary shrink-0 min-w-[92px] tabular-nums"
          disabled={busy || coolMs > 0}
          onclick={() => void runUpdateCheck(true)}
        >
          {#if s.stage === "checking"}
            <span
              class="inline-block w-3.5 h-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"
              aria-hidden="true"
            ></span>
            Checking…
          {:else if coolMs > 0}
            {coolLeft}s
          {:else}
            Check
          {/if}
        </button>
      </div>
    </div>
  {/if}

  {#if s.stage === "error" && !s.pending}
    <div class="px-4 py-3 flex items-start gap-2.5 bg-danger/[0.05]">
      <Icon name="alert-circle" size={14} class="text-danger mt-0.5" />
      <div class="min-w-0">
        <div class="text-xs font-semibold text-danger">Update check failed</div>
        <p class="text-[11px] text-muted-foreground mt-0.5 break-words">{s.error}</p>
      </div>
    </div>
  {/if}

  {#if s.pending}
    <!-- ── Update available box ── -->
    <div class="relative overflow-hidden animate-fade-in">
      <div
        class="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-primary/60 to-transparent"
        aria-hidden="true"
      ></div>
      <div class="bg-gradient-to-b from-primary/[0.07] to-transparent">
        <!-- Header: version jump + release date -->
        <div class="flex items-start gap-3 px-4 pt-4 pb-3">
          <span
            class="w-10 h-10 rounded-xl bg-primary/15 text-primary border border-primary/25
                   flex items-center justify-center shrink-0 shadow-sm"
          >
            <Icon name={s.stage === "ready" ? "rocket" : "sparkles"} size={19} strokeWidth={1.9} />
          </span>
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2 flex-wrap">
              <h4 class="text-sm font-semibold text-foreground leading-tight">
                {s.stage === "ready" ? "Ready to install" : "Update available"}
              </h4>
              <span class="badge-primary font-mono">v{s.version}</span>
            </div>
            <div
              class="flex items-center gap-1.5 mt-1 text-[11px] text-muted-foreground font-mono flex-wrap"
            >
              {#if s.currentVersion}
                <span>from v{s.currentVersion}</span>
              {/if}
              {#if releasedOn}
                <span class="opacity-50">·</span>
                <span class="inline-flex items-center gap-1">
                  <Icon name="clock" size={10} />
                  {releasedOn}
                </span>
              {/if}
              {#if changeCount > 0}
                <span class="opacity-50">·</span>
                <span>{changeCount} change{changeCount === 1 ? "" : "s"}</span>
              {/if}
            </div>
          </div>
          {#if s.stage === "available"}
            <button
              class="btn-icon shrink-0"
              title="Dismiss until the next check"
              aria-label="Dismiss update"
              onclick={() => void dismissUpdate()}
            >
              <Icon name="x" size={14} />
            </button>
          {/if}
        </div>

        <!-- Release notes -->
        {#if s.notes.length > 0}
          <div
            class="mx-4 mb-3 rounded-lg border border-border/70 bg-background/60 overflow-hidden"
          >
            <div class="max-h-56 overflow-y-auto px-3.5 py-3 space-y-3">
              {#each s.notes as section (section.title)}
                <div>
                  <div
                    class="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider {SECTION_TONE[
                      section.kind
                    ]}"
                  >
                    <Icon name={SECTION_ICON[section.kind]} size={11} strokeWidth={2} />
                    {section.title}
                  </div>
                  <ul class="mt-1.5 space-y-1.5">
                    {#each section.items as item, i (`${section.title}-${i}`)}
                      <li class="flex items-start gap-2 text-xs leading-relaxed">
                        <span
                          class="mt-[8px] w-1 h-1 rounded-full bg-muted-foreground/60 shrink-0"
                          aria-hidden="true"
                        ></span>
                        <span class="min-w-0 text-foreground/85 break-words">
                          {#if item.scope}
                            <span
                              class="font-mono text-[10px] px-1 py-px mr-1 rounded bg-elevated
                                     border border-border/60 text-muted-foreground align-[1px]"
                              >{item.scope}</span
                            >
                          {/if}{item.text}
                        </span>
                      </li>
                    {/each}
                  </ul>
                </div>
              {/each}
            </div>
          </div>
        {:else}
          <p class="mx-4 mb-3 text-[11px] text-muted-foreground italic">
            This release shipped without notes.
          </p>
        {/if}

        <!-- Progress / actions -->
        {#if s.stage === "downloading" || s.stage === "installing"}
          <div class="px-4 pb-4">
            <div class="flex items-center justify-between gap-2 mb-1.5 text-[11px]">
              <span class="font-medium text-foreground inline-flex items-center gap-1.5">
                <span
                  class="inline-block w-3 h-3 border-2 border-primary border-t-transparent rounded-full animate-spin"
                  aria-hidden="true"
                ></span>
                {s.stage === "installing" ? "Installing…" : "Downloading…"}
              </span>
              <span class="font-mono text-muted-foreground tabular-nums" aria-live="polite">
                {#if s.stage === "installing"}
                  please wait
                {:else if percent !== null}
                  {percent}% · {formatBytes(s.downloadedBytes)} / {formatBytes(s.totalBytes)}
                {:else}
                  {formatBytes(s.downloadedBytes) || "starting…"}
                {/if}
              </span>
            </div>
            <div
              class="h-1.5 w-full overflow-hidden rounded-full bg-border/70"
              role="progressbar"
              aria-valuemin="0"
              aria-valuemax="100"
              aria-valuenow={percent ?? undefined}
              aria-label="Update download progress"
            >
              {#if s.stage === "installing" || percent === null}
                <div class="h-full w-1/3 rounded-full bg-primary animate-indeterminate"></div>
              {:else}
                <div
                  class="h-full rounded-full bg-primary transition-[width] duration-200"
                  style:width="{percent}%"
                ></div>
              {/if}
            </div>
          </div>
        {:else if s.stage === "ready"}
          <div class="px-4 pb-4 flex items-center justify-between gap-3 flex-wrap">
            <p class="text-[11px] text-muted-foreground min-w-0 flex items-start gap-1.5">
              <Icon name="check-circle" size={13} class="text-success mt-px" />
              <span>
                Downloaded{formatBytes(s.totalBytes) ? ` (${formatBytes(s.totalBytes)})` : ""} —
                installing needs a restart, so pick a moment when no ports are busy.
              </span>
            </p>
            <button class="btn-success min-w-[112px] shrink-0" onclick={() => void restartNow()}>
              <Icon name="power" size={13} strokeWidth={2} />
              Restart Now
            </button>
          </div>
        {:else}
          <div class="px-4 pb-4 flex items-center justify-between gap-3 flex-wrap">
            {#if s.error}
              <p class="text-[11px] text-danger min-w-0 flex items-start gap-1.5">
                <Icon name="alert-circle" size={13} class="mt-px" />
                <span class="break-words">{s.error}</span>
              </p>
            {:else}
              <p class="text-[11px] text-muted-foreground min-w-0">
                Downloads in the background — you choose when to restart.
              </p>
            {/if}
            <button class="btn-primary min-w-[112px] shrink-0" onclick={() => void downloadUpdate()}>
              <Icon name="download" size={13} strokeWidth={2} />
              {s.error ? "Retry" : "Update Now"}
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
