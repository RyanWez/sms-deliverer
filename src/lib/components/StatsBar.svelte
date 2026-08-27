<script lang="ts">
  import { messagesStore } from '$lib/stores/messages.svelte';
  import { portsStore } from '$lib/stores/ports.svelte';
  import { liveStore } from '$lib/stores/live.svelte';
</script>

<div class="page-footer font-mono">
  <span class="truncate flex-1">{liveStore.statusText || (messagesStore.items.length > 0 ? `Total: ${messagesStore.items.length}` : 'Ready')}</span>
  <span class="w-px h-3.5 bg-border" aria-hidden="true"></span>
  <span class="flex items-center gap-1.5 whitespace-nowrap tabular-nums">
    <span class="w-1.5 h-1.5 rounded-full bg-primary" aria-hidden="true"></span>
    {messagesStore.visible.length} shown
  </span>
  <span class="w-px h-3.5 bg-border" aria-hidden="true"></span>
  <span class="flex items-center gap-1.5 whitespace-nowrap tabular-nums">
    <span class="w-1.5 h-1.5 rounded-full bg-otp" aria-hidden="true"></span>
    {messagesStore.otpCount} OTP
  </span>
  <span class="w-px h-3.5 bg-border" aria-hidden="true"></span>
  <span class="flex items-center gap-1.5 whitespace-nowrap tabular-nums">
    <span
      class="w-1.5 h-1.5 rounded-full shrink-0 aspect-square {liveStore.on ? 'bg-success animate-pulse-dot' : 'bg-muted-foreground/50'}"
      aria-hidden="true"
    ></span>
    {liveStore.on ? `LIVE ${liveStore.readyPorts.length}/${liveStore.totalPorts}` : 'live off'}
  </span>
  <span class="w-px h-3.5 bg-border" aria-hidden="true"></span>
  <span class="flex items-center gap-1.5 whitespace-nowrap tabular-nums">
    <span class="w-1.5 h-1.5 rounded-full bg-success" aria-hidden="true"></span>
    {portsStore.items.length} ports
  </span>
</div>
