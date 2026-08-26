import type { ToastData } from '$lib/types';

export function createLiveStore() {
  let on = $state(false);
  let scanBusy = $state(false);
  let ussdBusy = $state(false);
  let readyPorts = $state<string[]>([]);
  let totalPorts = $state(0);
  let statusText = $state('');
  let toasts = $state<ToastData[]>([]);

  function addToast(t: ToastData) {
    toasts = [...toasts, t];
    setTimeout(() => {
      toasts = toasts.filter(x => x.id !== t.id);
    }, 4000);
  }

  function removeToast(id: number) {
    toasts = toasts.filter(t => t.id !== id);
  }

  return {
    get on() { return on; },
    set on(v: boolean) { on = v; },
    get scanBusy() { return scanBusy; },
    set scanBusy(v: boolean) { scanBusy = v; },
    get ussdBusy() { return ussdBusy; },
    set ussdBusy(v: boolean) { ussdBusy = v; },
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
