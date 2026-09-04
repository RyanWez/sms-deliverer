<script lang="ts">
  /**
   * Changelog page: this project's own CHANGELOG.md, read inside the app.
   *
   * The file is imported at build time rather than fetched, so the history is
   * there on a bank with no internet and can never disagree with the binary it
   * shipped in — `tauri-build.yml` builds from the release tag, by which point
   * the release PR has already written that version's section. The trade is
   * that the newest entry an install can show is its own version.
   *
   * Nothing here renders markdown. Every line is parsed into plain text and
   * printed through normal interpolation, the same rule the update card's
   * release notes follow, and the repository references release-please appends
   * are dropped by the parser: `#28` and `e7da48b` lead nowhere from in here.
   */
  import { onMount } from "svelte";
  import changelogSource from "../../../CHANGELOG.md?raw";
  import Icon from "$lib/components/Icon.svelte";
  import type { IconName } from "$lib/icons";
  import { navigationStore } from "$lib/stores/navigation.svelte";
  import {
    countByFilter,
    filterReleases,
    normalizeVersion,
    parseChangelog,
    type ChangeFilter,
  } from "$lib/utils/changelog";
  import { formatReleaseDate, type NoteKind } from "$lib/utils/release-notes";
  import { isTauri } from "$lib/utils/tauri";

  const releases = parseChangelog(changelogSource);
  const counts = countByFilter(releases);

  let filter = $state<ChangeFilter>("all");
  const shown = $derived(filterReleases(releases, filter));

  /**
   * The running version, so the entry the operator is actually on can be marked.
   *
   * Browser preview has no shell to ask, and there the newest entry in the
   * bundled file is the checkout's version by construction — release-please
   * writes both from the same commit.
   */
  let currentVersion = $state(normalizeVersion(releases[0]?.version));

  onMount(async () => {
    if (!isTauri()) return;
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      currentVersion = normalizeVersion(await getVersion());
    } catch {
      currentVersion = "";
    }
  });

  const currentRelease = $derived(
    releases.find((r) => r.version === currentVersion) ?? null,
  );

  const FILTERS: Array<{ id: ChangeFilter; label: string }> = [
    { id: "all", label: "All" },
    { id: "feature", label: "Features" },
    { id: "fix", label: "Fixes" },
    { id: "other", label: "Other" },
  ];

  const SECTION_ICON: Record<NoteKind, IconName> = {
    feature: "sparkles",
    fix: "bug",
    other: "wrench",
  };
  const SECTION_TONE: Record<NoteKind, string> = {
    feature: "text-primary",
    fix: "text-success",
    other: "text-muted-foreground",
  };

  function anchorId(version: string): string {
    return `release-${version.replace(/[^\w.-]/g, "-")}`;
  }

  function jumpTo(version: string) {
    document
      .getElementById(anchorId(version))
      ?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  function plural(n: number): string {
    return n === 1 ? "" : "s";
  }
</script>

<div class="flex-1 flex flex-col h-full overflow-hidden min-h-0" id="panel-changelog">
  <header class="page-header">
    <div class="min-w-0">
      <h1 class="page-title">Changelog</h1>
      <p class="page-subtitle">Every release of this app, newest first</p>
    </div>

    <div
      class="ml-auto flex items-center gap-1 p-0.5 rounded-lg bg-elevated border border-border"
      role="group"
      aria-label="Filter changes by kind"
    >
      {#each FILTERS as f (f.id)}
        {@const active = filter === f.id}
        <button
          type="button"
          class="inline-flex items-center gap-1.5 px-2.5 h-7 rounded-md text-[11px] font-medium
                 transition-colors duration-150
                 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
                 disabled:opacity-40 disabled:pointer-events-none
                 {active
                   ? 'bg-surface text-foreground shadow-sm'
                   : 'text-muted-foreground hover:text-foreground'}"
          aria-pressed={active}
          disabled={counts[f.id] === 0}
          onclick={() => (filter = f.id)}
        >
          {f.label}
          <span class="font-mono tabular-nums opacity-60">{counts[f.id]}</span>
        </button>
      {/each}
    </div>
  </header>

  <div class="flex-1 overflow-y-auto overflow-x-hidden min-h-0 bg-background">
    <div class="mx-auto max-w-[900px] px-4 sm:px-6 py-5 flex gap-6 lg:gap-8">
      <div class="flex-1 min-w-0">
        <!-- Which build this is. The "Check for updates" shortcut goes to the
             Updates panel rather than starting a check here, so the endpoint is
             still queried from one place with one cooldown. -->
        <section class="card overflow-hidden mb-6">
          <div
            class="bg-gradient-to-b from-primary/[0.07] to-transparent
                   px-4 py-4 flex items-start gap-3 flex-wrap sm:flex-nowrap"
          >
            <span
              class="w-10 h-10 rounded-xl bg-primary/15 text-primary border border-primary/25
                     flex items-center justify-center shrink-0 shadow-sm"
            >
              <Icon name="zap" size={19} strokeWidth={1.9} />
            </span>
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2 flex-wrap">
                <h2 class="text-sm font-semibold text-foreground leading-tight">
                  You are running
                </h2>
                <span class="badge-primary font-mono">v{currentVersion || "—"}</span>
              </div>
              <p class="text-[11px] text-muted-foreground mt-1 leading-relaxed">
                {#if currentRelease}
                  Released {formatReleaseDate(currentRelease.date) || "an unrecorded date"}
                  · {currentRelease.changeCount} change{plural(currentRelease.changeCount)}
                  in this version.
                {:else}
                  This build is not in the bundled changelog — the newest entry below is
                  the last release it was cut from.
                {/if}
              </p>
            </div>
            <button
              class="btn-secondary shrink-0"
              onclick={() => navigationStore.openSettings("updates")}
            >
              <Icon name="download" size={13} strokeWidth={2} />
              Check for updates
            </button>
          </div>
        </section>

        {#if shown.length === 0}
          <div class="empty-state">
            <Icon name="history" size={28} class="opacity-40" />
            <p class="empty-state-title mt-3">No release history</p>
            <p class="empty-state-hint">
              The changelog bundled with this build is empty.
            </p>
          </div>
        {:else}
          <ol class="relative" role="list">
            {#each shown as rel, i}
              {@const isCurrent = rel.version === currentVersion}
              <li id={anchorId(rel.version)} class="relative pl-8 pb-7 last:pb-1">
                <!-- Timeline rail: drawn per entry and omitted on the last one,
                     so the line ends at the oldest dot instead of running on. -->
                {#if i < shown.length - 1}
                  <span
                    class="absolute left-[7.5px] top-5 bottom-0 w-px bg-border"
                    aria-hidden="true"
                  ></span>
                {/if}
                <span
                  class="absolute left-0 top-[3px] w-4 h-4 rounded-full border-2
                         flex items-center justify-center
                         {isCurrent
                           ? 'border-primary bg-primary/20'
                           : 'border-border bg-surface'}"
                  aria-hidden="true"
                >
                  {#if isCurrent}
                    <span class="w-1.5 h-1.5 rounded-full bg-primary"></span>
                  {/if}
                </span>

                <div class="flex items-center gap-2 flex-wrap">
                  <h3
                    class="text-sm font-semibold font-mono leading-none
                           {isCurrent ? 'text-primary' : 'text-foreground'}"
                  >
                    v{rel.version}
                  </h3>
                  {#if isCurrent}
                    <span class="badge-primary text-[10px]">
                      <Icon name="check" size={10} strokeWidth={2.5} />
                      Current
                    </span>
                  {/if}
                  {#if rel.bump}
                    <span class="badge-muted text-[10px] uppercase tracking-wider">
                      {rel.bump}
                    </span>
                  {/if}
                  <span
                    class="text-[11px] text-muted-foreground font-mono inline-flex items-center gap-1"
                  >
                    <Icon name="clock" size={10} />
                    {formatReleaseDate(rel.date) || "undated"}
                  </span>
                  <span class="text-[11px] text-muted-foreground/60 font-mono">
                    {rel.changeCount} change{plural(rel.changeCount)}
                  </span>
                </div>

                {#if rel.sections.length === 0}
                  <p class="mt-2 text-[11px] text-muted-foreground italic">
                    This release shipped without notes.
                  </p>
                {:else}
                  <div class="card mt-2.5 overflow-hidden divide-y divide-border/50">
                    {#each rel.sections as section}
                      <div class="px-3.5 py-3">
                        <div
                          class="flex items-center gap-1.5 text-[10px] font-semibold
                                 uppercase tracking-wider {SECTION_TONE[section.kind]}"
                        >
                          <Icon name={SECTION_ICON[section.kind]} size={11} strokeWidth={2} />
                          {section.title}
                        </div>
                        <ul class="mt-2 space-y-1.5">
                          {#each section.items as item, j (`${section.title}-${j}`)}
                            <li class="flex items-start gap-2 text-xs leading-relaxed">
                              <span
                                class="mt-[7px] w-1 h-1 rounded-full bg-muted-foreground/60 shrink-0"
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
                {/if}

              </li>
            {/each}
          </ol>
        {/if}


      </div>

      <!-- Version index. Wide screens only: below that the timeline needs the
           whole column, and the list is a shortcut rather than the way in. -->
      <nav class="hidden lg:block w-[88px] shrink-0" aria-label="Jump to a release">
        <div class="sticky top-0">
          <div
            class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground px-2 pb-1.5"
          >
            Releases
          </div>
          <ul class="space-y-px" role="list">
            {#each shown as rel}
              <li>
                <button
                  type="button"
                  class="w-full text-left px-2 py-1 rounded text-[11px] font-mono
                         transition-colors duration-150
                         focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70
                         {rel.version === currentVersion
                           ? 'text-primary font-semibold bg-primary/10'
                           : 'text-muted-foreground hover:text-foreground hover:bg-elevated'}"
                  onclick={() => jumpTo(rel.version)}
                >
                  v{rel.version}
                </button>
              </li>
            {/each}
          </ul>
        </div>
      </nav>

    </div>
  </div>

  <footer class="page-footer">
    <span>
      {#if filter === "all"}
        {releases.length} release{plural(releases.length)} · {counts.all} change{plural(
          counts.all,
        )} · bundled with this build
      {:else}
        {shown.length} of {releases.length} release{plural(releases.length)} · {counts[
          filter
        ]} of {counts.all} change{plural(counts.all)}
      {/if}
    </span>
    <span class="font-mono">{currentVersion ? `v${currentVersion}` : ""}</span>
  </footer>
</div>

