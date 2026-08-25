<script lang="ts">
  import { liveStore } from '$lib/stores/live.svelte';

  const toasts = $derived(() => liveStore.toasts);

  function kindClass(kind: string): string {
    switch (kind) {
      case 'Otp': return 'toast-otp';
      case 'Success': return 'toast-success';
      case 'Danger': return 'toast-danger';
      case 'Warning': return 'toast-danger';
      default: return 'toast-info';
    }
  }
</script>

<div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
  {#each toasts() as t (t.id)}
    <div class="{kindClass(t.kind)} animate-slide-in">
      <div class="flex-1 min-w-0">
        <div class="text-xs font-semibold">{t.title}</div>
        <div class="text-xs opacity-80 truncate">{t.body}</div>
        {#if t.otp}
          <div class="text-sm font-bold font-mono mt-1 tracking-wider">{t.otp}</div>
        {/if}
      </div>
      <button
        class="opacity-50 hover:opacity-100 transition-opacity shrink-0"
        onclick={() => liveStore.removeToast(t.id)}
        title="Dismiss"
      >
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="3" y1="3" x2="9" y2="9"/><line x1="9" y1="3" x2="3" y2="9"/>
        </svg>
      </button>
    </div>
  {/each}
</div>
