<script lang="ts">
  import { icons, type IconName } from '$lib/icons';

  interface Props {
    name: IconName;
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let { name, size = 16, strokeWidth = 1.75, class: cls = '' }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  class="shrink-0 {cls}"
  aria-hidden="true"
>
  {@html icons[name]}
</svg>
