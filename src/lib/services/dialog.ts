import { isTauri } from '$lib/utils/tauri';

/**
 * Native confirmation dialog helpers.
 *
 * Browsers' window.confirm()/alert() silently no-op inside Tauri webviews,
 * so every destructive action must go through @tauri-apps/plugin-dialog
 * (see Memory/03-troubleshooting.md case #7). Falls back to window.confirm()
 * only when running outside Tauri (plain browser dev preview).
 */

export type ConfirmKind = 'info' | 'warning' | 'error';

interface ConfirmOptions {
  title?: string;
  kind?: ConfirmKind;
  okLabel?: string;
  cancelLabel?: string;
}

/** Show a native confirmation dialog; resolves true when user confirms. */
export async function confirmDialog(text: string, opts: ConfirmOptions = {}): Promise<boolean> {
  if (!isTauri()) {
    // Browser dev preview — best-effort browser dialog.
    return window.confirm(text);
  }
  const { confirm } = await import('@tauri-apps/plugin-dialog');
  return confirm(text, {
    title: opts.title ?? 'Please confirm',
    kind: opts.kind ?? 'info',
    okLabel: opts.okLabel ?? 'OK',
    cancelLabel: opts.cancelLabel ?? 'Cancel',
  });
}
