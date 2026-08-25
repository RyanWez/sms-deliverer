<script lang="ts">
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';
  import { liveStore } from '$lib/stores/live.svelte';
</script>

<div class="h-8 flex items-center gap-4 px-4 bg-surface border-t border-border shrink-0 text-[11px] font-mono text-muted-foreground">
  <span class="truncate flex-1">{messagesStore.items.length > 0 ? `Total: ${messagesStore.items.length}` : 'Ready'}</span>
  <span class="flex items-center gap-1.5">
    <span class="w-1.5 h-1.5 rounded-full bg-primary"></span>
    {messagesStore.visible.length} shown
  </span>
  <span class="flex items-center gap-1.5">
    <span class="w-1.5 h-1.5 rounded-full bg-otp"></span>
    {messagesStore.otpCount} OTP
  </span>
  <span class="flex items-center gap-1.5">
    <span class="w-1.5 h-1.5 rounded-full {liveStore.on ? 'bg-success' : 'bg-muted'}"></span>
    {liveStore.on ? `LIVE ${liveStore.readyPorts.length}/${liveStore.totalPorts}` : 'live off'}
  </span>
  <span class="flex items-center gap-1.5">
    <span class="w-1.5 h-1.5 rounded-full bg-success"></span>
    {portsStore.items.length} ports
  </span>
</div>
