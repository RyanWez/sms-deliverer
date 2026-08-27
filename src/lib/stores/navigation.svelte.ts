import type { NavSection } from '$lib/types';

export function createNavigationStore() {
  let currentSection = $state<NavSection>('inbox');
  let sidebarCollapsed = $state(false);
  let sidebarAnimating = $state(false);
  let animTimer: ReturnType<typeof setTimeout> | null = null;

  function setAnimating() {
    if (typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    sidebarAnimating = true;
    if (animTimer) clearTimeout(animTimer);
    // match sidebar width transition duration (240ms) + small buffer
    animTimer = setTimeout(() => {
      sidebarAnimating = false;
      animTimer = null;
    }, 280);
  }

  return {
    get currentSection() { return currentSection; },
    set currentSection(v: NavSection) { currentSection = v; },
    get sidebarCollapsed() { return sidebarCollapsed; },
    set sidebarCollapsed(v: boolean) { sidebarCollapsed = v; },
    get isAnimating() { return sidebarAnimating; },
    toggleSidebar() {
      sidebarCollapsed = !sidebarCollapsed;
      setAnimating();
    },
    navigate(section: NavSection) { currentSection = section; },
  };
}

export const navigationStore = createNavigationStore();