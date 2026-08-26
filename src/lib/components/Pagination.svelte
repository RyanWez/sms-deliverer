<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
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
  <div class="page-footer font-mono">
    <div class="flex-1 tabular-nums">
      {#if messagesStore.pageRows.length > 0}
        Showing {messagesStore.pageIndexStart + 1}–{messagesStore.pageIndexStart + messagesStore.pageRows.length} of {messagesStore.visible.length}
      {/if}
    </div>

    <nav class="flex items-center gap-1" aria-label="Pagination">
      <button
        class="btn-icon border border-border w-8 h-7"
        onclick={() => cur > 1 && messagesStore.goTo(cur - 1)}
        disabled={cur <= 1}
        title="Previous page"
        aria-label="Previous page"
      >
        <Icon name="chevron-left" size={14} />
      </button>

      {#each pageNumbers() as p (p)}
        <button
          class="w-8 h-7 rounded-md text-xs font-semibold border transition-colors duration-150
                 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70 focus-visible:ring-offset-1 focus-visible:ring-offset-background
                 disabled:cursor-default
                 {p === cur
                   ? 'bg-primary/15 border-primary/50 text-primary'
                   : 'border-border text-muted-foreground hover:bg-elevated hover:text-foreground'}"
          aria-current={p === cur ? 'page' : undefined}
          aria-label={`Page ${p}`}
          onclick={() => p !== cur && messagesStore.goTo(p)}
        >
          {p}
        </button>
      {/each}

      <button
        class="btn-icon border border-border w-8 h-7"
        onclick={() => cur < total && messagesStore.goTo(cur + 1)}
        disabled={cur >= total}
        title="Next page"
        aria-label="Next page"
      >
        <Icon name="chevron-right" size={14} />
      </button>
    </nav>

    <div class="flex-1 text-right tabular-nums">
      Page {cur} / {total}
    </div>
  </div>
{/if}
