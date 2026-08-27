/**
 * Utility to check if the application is running inside a Tauri desktop container
 * or in a standard web browser (e.g. Vite dev preview).
 */
export function isTauri(): boolean {
  return (
    typeof window !== 'undefined' &&
    ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)
  );
}
