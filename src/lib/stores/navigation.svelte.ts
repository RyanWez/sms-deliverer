import type { NavSection } from '$lib/types';

export function createNavigationStore() {
  let currentSection = $state<NavSection>('inbox');
  let sidebarCollapsed = $state(false);
  /**
   * Settings category to land on, consumed once by the Settings page.
   *
   * Set by callers that mean a specific panel — the update card's "What's new"
   * link, for instance — rather than the General tab the page opens on.
   */
  let pendingSettingsGroup = $state<string | null>(null);

  return {
    get currentSection() { return currentSection; },
    set currentSection(v: NavSection) { currentSection = v; },
    get sidebarCollapsed() { return sidebarCollapsed; },
    set sidebarCollapsed(v: boolean) { sidebarCollapsed = v; },
    get isAnimating() { return false; },
    toggleSidebar() {
      sidebarCollapsed = !sidebarCollapsed;
    },
    navigate(section: NavSection) { currentSection = section; },
    openSettings(group: string) {
      pendingSettingsGroup = group;
      currentSection = 'settings';
    },
    /** Read and clear the requested category. */
    takeSettingsGroup(): string | null {
      const g = pendingSettingsGroup;
      pendingSettingsGroup = null;
      return g;
    },
  };
}

export const navigationStore = createNavigationStore();