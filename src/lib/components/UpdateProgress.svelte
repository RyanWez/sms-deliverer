<script lang="ts">
  import { liveStore } from '$lib/stores/live.svelte';

  const p = $derived(liveStore.updateProgress);

  function percent(): number {
    if (!p || p.totalBytes <= 0) return 0;
    return Math.min(100, Math.round((p.downloadedBytes / p.totalBytes) * 100));
  }

  function sizeLabel(bytes: number): string {
    if (bytes <= 0) return '';
    return bytes >= 1024 * 1024
      ? `${(bytes / 1024 / 1024).toFixed(1)} MB`
      : `${(bytes / 1024).toFixed(0)} KB`;
  }
</script>

{#if p}
  <div class="fixed bottom-20 right-4 z-50 w-72 max-w-[calc(100vw-2rem)] rounded-lg border border-border bg-card shadow-xl p-3.5">
    <div class="flex items-center justify-between gap-2 mb-2">
      <span class="text-xs font-semibold text-foreground truncate">
        Updating to v{p.version}
      </span>
      <span class="text-[11px] font-mono text-muted-foreground shrink-0">
        {#if p.phase === 'install'}
          Installing…
        {:else if p.totalBytes > 0}
          {percent()}% · {sizeLabel(p.downloadedBytes)} / {sizeLabel(p.totalBytes)}
        {:else}
          Downloading…
        {/if}
      </span>
    </div>
    <div class="h-1.5 w-full overflow-hidden rounded-full bg-border">
      <div
        class="h-full rounded-full bg-primary transition-[width] duration-150 {p.phase === 'install' ? 'animate-pulse' : ''}"
        style:width="{p.phase === 'install' ? '100%' : `${percent()}%`}"
      ></div>
    </div>
  </div>
{/if}
