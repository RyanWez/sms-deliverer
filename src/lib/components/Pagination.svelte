<script lang="ts">
  import { messagesStore } from '$lib/stores/messages.svelte';

  const total = $derived(messagesStore.totalPages);
  const cur = $derived(messagesStore.page);

  function pageNumbers(): number[] {
    const win = 5;
    let start = Math.max(1, cur - Math.floor(win / 2));
    const end = Math.min(total, start + win - 1);
    start = Math.max(1, end - win + 1);
    const arr: number[] = [];
    for (let i = start; i <= end; i++) arr.push(i);
    return arr;
  }
</script>

{#if messagesStore.visible.length > 0}
  <div class="h-11 shrink-0 flex items-center px-4 bg-surface border-t border-border gap-4">
    <div class="flex-1 text-[11px] font-mono text-muted-foreground">
      {#if messagesStore.pageRows.length > 0}
        Showing {messagesStore.pageIndexStart + 1}–{messagesStore.pageIndexStart + messagesStore.pageRows.length} of {messagesStore.visible.length}
      {/if}
    </div>

    <div class="flex items-center gap-1.5">
      <button
        class="w-8 h-8 rounded-lg border border-border flex items-center justify-center text-muted-foreground transition-all duration-150
               {cur <= 1 ? 'opacity-30 cursor-default' : 'hover:bg-elevated hover:text-foreground'}"
        onclick={() => cur > 1 && messagesStore.goTo(cur - 1)}
        disabled={cur <= 1}
        title="Previous page"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="15 18 9 12 15 6"/>
        </svg>
      </button>

      {#each pageNumbers() as p (p)}
        <button
          class="w-8 h-8 rounded-lg text-xs font-mono font-semibold border transition-all duration-150
                 {p === cur
                   ? 'bg-primary/15 border-primary text-primary'
                   : 'border-border text-muted-foreground hover:bg-elevated hover:text-foreground'}"
          class:cursor-default={p === cur}
          onclick={() => p !== cur && messagesStore.goTo(p)}
        >
          {p}
        </button>
      {/each}

      <button
        class="w-8 h-8 rounded-lg border border-border flex items-center justify-center text-muted-foreground transition-all duration-150
               {cur >= total ? 'opacity-30 cursor-default' : 'hover:bg-elevated hover:text-foreground'}"
        onclick={() => cur < total && messagesStore.goTo(cur + 1)}
        disabled={cur >= total}
        title="Next page"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="9 18 15 12 9 6"/>
        </svg>
      </button>
    </div>

    <div class="flex-1 text-right text-[11px] font-mono text-muted-foreground">
      Page {cur} / {total}
    </div>
  </div>
{/if}
