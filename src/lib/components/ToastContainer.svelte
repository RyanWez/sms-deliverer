<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import type { IconName } from '$lib/icons';
  import { liveStore } from '$lib/stores/live.svelte';
  import type { ToastData } from '$lib/types';

  const toasts = $derived(liveStore.toasts);

  function kindMeta(kind: ToastData['kind']): { cls: string; icon: IconName } {
    switch (kind) {
      case 'Otp': return { cls: 'toast-otp', icon: 'zap' };
      case 'Success': return { cls: 'toast-success', icon: 'check-circle' };
      case 'Danger': return { cls: 'toast-danger', icon: 'alert-circle' };
      case 'Warning': return { cls: 'toast-danger', icon: 'alert-triangle' };
      default: return { cls: 'toast-info', icon: 'info' };
    }
  }
</script>

<div class="toast-container" role="status" aria-live="polite">
  {#each toasts as t (t.id)}
    {@const meta = kindMeta(t.kind)}
    <div class={meta.cls}>
      <span class="toast-icon mt-0.5 shrink-0"><Icon name={meta.icon} size={16} /></span>
      <div class="toast-body">
        <div class="toast-title">{t.title}</div>
        <div class="toast-text">{t.body}</div>
        {#if t.otp}
          <div class="toast-otp-value">{t.otp}</div>
        {/if}
      </div>
      <button
        class="btn-icon w-6 h-6 opacity-60 hover:opacity-100"
        onclick={() => liveStore.removeToast(t.id)}
        title="Dismiss"
        aria-label="Dismiss notification"
      >
        <Icon name="x" size={12} strokeWidth={2} />
      </button>
    </div>
  {/each}
</div>
