import type { ToastData } from '$lib/types';
import { dismissToast, pushToast } from '$lib/utils/toast-queue';

export function createLiveStore() {
  let on = $state(false);
  let scanBusy = $state(false);
  let ussdBusy = $state(false);
  let detectBusy = $state(false);
  let readyPorts = $state<string[]>([]);
  let totalPorts = $state(0);
  let statusText = $state('');
  let toasts = $state<ToastData[]>([]);

  // Bounded and coalescing — see utils/toast-queue.ts for why. The 4 s timer
  // still keys on the id it was scheduled with: a coalesced card is a new id, so
  // the superseded timer finds nothing to remove and the merged card gets its own
  // full 4 s. A repeat therefore keeps the notice on screen, which is what you
  // want from a port that is still flapping.
  function addToast(t: ToastData) {
    toasts = pushToast(toasts, t);
    setTimeout(() => {
      toasts = dismissToast(toasts, t.id);
    }, 4000);
  }

  function removeToast(id: number) {
    toasts = dismissToast(toasts, id);
  }

  return {
    get on() { return on; },
    set on(v: boolean) { on = v; },
    get scanBusy() { return scanBusy; },
    set scanBusy(v: boolean) { scanBusy = v; },
    get ussdBusy() { return ussdBusy; },
    set ussdBusy(v: boolean) { ussdBusy = v; },
    get detectBusy() { return detectBusy; },
    set detectBusy(v: boolean) { detectBusy = v; },
    get readyPorts() { return readyPorts; },
    set readyPorts(v: string[]) { readyPorts = v; },
    get totalPorts() { return totalPorts; },
    set totalPorts(v: number) { totalPorts = v; },
    get statusText() { return statusText; },
    set statusText(v: string) { statusText = v; },
    get toasts() { return toasts; },
    addToast,
    removeToast,
  };
}

export const liveStore = createLiveStore();
