import type { NavSection } from '$lib/types';

export function createNavigationStore() {
  let currentSection = $state<NavSection>('inbox');
  let sidebarCollapsed = $state(false);

  return {
    get currentSection() { return currentSection; },
    set currentSection(v: NavSection) { currentSection = v; },
    get sidebarCollapsed() { return sidebarCollapsed; },
    set sidebarCollapsed(v: boolean) { sidebarCollapsed = v; },
    toggleSidebar() { sidebarCollapsed = !sidebarCollapsed; },
    navigate(section: NavSection) { currentSection = section; },
  };
}

export const navigationStore = createNavigationStore();