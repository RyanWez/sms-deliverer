<script lang="ts">
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { api } from '$lib/services/api';

  const settingsGroups = [
    {
      id: 'general',
      label: 'General',
      icon: 'general',
      description: 'Application behavior and startup options',
      fields: [
        {
          key: 'autoStartLive',
          label: 'Auto-start Live Mode',
          description: 'Automatically start live monitoring on application launch',
          type: 'checkbox' as const,
          bind: 'general',
        },
        {
          key: 'minimizeToTray',
          label: 'Minimize to System Tray',
          description: 'Keep the app running in the background when minimized',
          type: 'checkbox' as const,
          bind: 'general',
        },
        {
          key: 'confirmDelete',
          label: 'Confirm Before Deleting',
          description: 'Show confirmation dialog when deleting messages',
          type: 'checkbox' as const,
          bind: 'general',
        },
        {
          key: 'portRefreshInterval',
          label: 'Port Refresh Interval (seconds)',
          description: 'How often to automatically refresh the port list',
          type: 'number' as const,
          bind: 'general',
          min: 10,
          max: 300,
          step: 5,
        },
      ],
    },
    {
      id: 'notifications',
      label: 'Notifications',
      icon: 'notifications',
      description: 'Configure how and when you receive alerts',
      fields: [
        {
          key: 'enabled',
          label: 'Enable Notifications',
          description: 'Show desktop notifications for new messages',
          type: 'checkbox' as const,
          bind: 'notifications',
        },
        {
          key: 'soundEnabled',
          label: 'Play Sound',
          description: 'Play a notification sound when a new message arrives',
          type: 'checkbox' as const,
          bind: 'notifications',
        },
        {
          key: 'desktopNotifications',
          label: 'Desktop Notifications',
          description: 'Show native OS notifications for new SMS',
          type: 'checkbox' as const,
          bind: 'notifications',
        },
        {
          key: 'otpOnlyNotifications',
          label: 'OTP Messages Only',
          description: 'Only notify for messages containing OTP codes',
          type: 'checkbox' as const,
          bind: 'notifications',
        },
      ],
    },
    {
      id: 'otp',
      label: 'OTP Settings',
      icon: 'otp',
      description: 'One-time password detection and handling',
      fields: [
        {
          key: 'autoCopy',
          label: 'Auto-copy OTP to Clipboard',
          description: 'Automatically copy detected OTP codes to clipboard',
          type: 'checkbox' as const,
          bind: 'otp',
        },
        {
          key: 'showInTable',
          label: 'Show OTP Column',
          description: 'Display OTP codes in the message table',
          type: 'checkbox' as const,
          bind: 'otp',
        },
        {
          key: 'highlightNewOtp',
          label: 'Highlight New OTP',
          description: 'Visually highlight newly received OTP messages',
          type: 'checkbox' as const,
          bind: 'otp',
        },
        {
          key: 'otpPattern',
          label: 'OTP Detection Pattern (Regex)',
          description: 'Regular expression used to detect OTP codes in messages',
          type: 'text' as const,
          bind: 'otp',
          placeholder: '\\b(\\d{4,8})\\b',
        },
      ],
    },
    {
      id: 'appearance',
      label: 'Appearance',
      icon: 'appearance',
      description: 'Customize the look and feel of the application',
      fields: [
        {
          key: 'theme',
          label: 'Theme',
          description: 'Choose the color theme for the application',
          type: 'select' as const,
          bind: 'appearance',
          options: [
            { value: 'system', label: 'System' },
            { value: 'dark', label: 'Dark' },
            { value: 'light', label: 'Light' },
          ],
        },
        {
          key: 'compactMode',
          label: 'Compact Mode',
          description: 'Reduce spacing for more content on screen',
          type: 'checkbox' as const,
          bind: 'appearance',
        },
        {
          key: 'showSIMColumn',
          label: 'Show SIM Column',
          description: 'Display SIM number column in message table',
          type: 'checkbox' as const,
          bind: 'appearance',
        },
        {
          key: 'showPortColumn',
          label: 'Show Port Column',
          description: 'Display port name column in message table',
          type: 'checkbox' as const,
          bind: 'appearance',
        },
      ],
    },
    {
      id: 'updates',
      label: 'Updates',
      icon: 'https://img.icons8.com/fluency-systems-filled/96/FFFFFF/uninstalling-updates.png',
      description: 'Application update preferences',
      fields: [
        {
          key: 'autoCheck',
          label: 'Automatically Check for Updates',
          description: 'Check for new versions on startup',
          type: 'checkbox' as const,
          bind: 'updates',
        },
        {
          key: 'checkInterval',
          label: 'Check Interval (hours)',
          description: 'How often to check for updates in the background',
          type: 'number' as const,
          bind: 'updates',
          min: 1,
          max: 168,
          step: 1,
        },
      ],
    },
    {
      id: 'advanced',
      label: 'Advanced',
      icon: 'https://img.icons8.com/fluency-systems-regular/96/FFFFFF/advanced-lighting-panel.png',
      description: 'Advanced options and data management',
      fields: [
        {
          key: 'resetSettings',
          label: 'Reset All Settings to Defaults',
          description: 'Restore all settings to their default values',
          type: 'button' as const,
          action: 'reset',
          variant: 'danger',
        },
        {
          key: 'clearMessages',
          label: 'Clear All Messages',
          description: 'Permanently delete all stored SMS messages',
          type: 'button' as const,
          action: 'clearMessages',
          variant: 'danger',
        },
      ],
    },
  ] as const;

  let expandedGroups = $state<Set<string>>(new Set(['general']));

  function toggleGroup(id: string) {
    const next = new Set(expandedGroups);
    if (next.has(id)) next.delete(id); else next.add(id);
    expandedGroups = next;
  }

  async function handleAction(action: string) {
    if (action === 'reset') {
      if (confirm('Reset all settings to defaults? This cannot be undone.')) {
        settingsStore.resetToDefaults();
      }
    } else if (action === 'clearMessages') {
      if (confirm('Permanently delete ALL messages? This cannot be undone.')) {
        await api.clearAll();
      }
    }
  }

  function getNestedValue(obj: any, path: string): any {
    return path.split('.').reduce((o, k) => o?.[k], obj);
  }

  function setNestedValue(obj: any, path: string, value: any) {
    const keys = path.split('.');
    const last = keys.pop()!;
    const target = keys.reduce((o, k) => o[k], obj);
    target[last] = value;
  }
</script>

<div class="flex-1 flex flex-col h-full">
  <header class="px-5 py-4 bg-surface border-b border-border shrink-0">
    <h1 class="text-lg font-semibold text-foreground">Settings</h1>
    <p class="text-xs text-muted-foreground mt-0.5">Configure application preferences and behavior</p>
  </header>

  <div class="flex-1 overflow-auto p-5">
    <div class="max-w-3xl mx-auto space-y-4">
      {#each settingsGroups as group}
        <section class="card overflow-hidden" aria-labelledby={`heading-${group.id}`}>
          <div
            class="flex items-center gap-3 px-4 py-3.5 border-b border-border/50 cursor-pointer hover:bg-elevated/30 transition-colors"
            onclick={() => toggleGroup(group.id)}
            role="button"
            tabindex={0}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleGroup(group.id); } }}
            aria-expanded={expandedGroups.has(group.id)}
            aria-controls={`content-${group.id}`}
            id={`heading-${group.id}`}
          >
            <span class="w-8 h-8 rounded-lg bg-elevated flex items-center justify-center text-muted-foreground shrink-0">
              {#if group.icon.startsWith('http')}
                <img src={group.icon} alt="" width="18" height="18" class="brightness-0 invert" />
              {:else if group.icon === 'general'}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
                  <line x1="8" y1="21" x2="16" y2="21"></line>
                  <line x1="12" y1="17" x2="12" y2="21"></line>
                </svg>
              {:else if group.icon === 'notifications'}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"></path>
                  <path d="M13.73 21a2 2 0 0 1-3.46 0"></path>
                </svg>
              {:else if group.icon === 'otp'}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="2" y="3" width="20" height="14" rx="2"></rect>
                  <path d="M8 21h8"></path>
                  <path d="M12 17v4"></path>
                </svg>
              {:else if group.icon === 'appearance'}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="12" cy="12" r="5"></circle>
                  <line x1="12" y1="1" x2="12" y2="3"></line>
                  <line x1="12" y1="21" x2="12" y2="23"></line>
                  <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line>
                  <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line>
                  <line x1="1" y1="12" x2="3" y2="12"></line>
                  <line x1="21" y1="12" x2="23" y2="12"></line>
                  <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line>
                  <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
                </svg>
              {:else}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <polygon points="12 2 22 8.5 22 15.5 12 22 2 15.5 2 8.5 12 2"></polygon>
                  <path d="M12 22V12"></path>
                  <path d="M22 8.5V15.5"></path>
                </svg>
              {/if}
            </span>
            <div class="flex-1 min-w-0">
              <h2 class="font-medium text-foreground truncate">{group.label}</h2>
              <p class="text-[11px] text-muted-foreground truncate">{group.description}</p>
            </div>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class={expandedGroups.has(group.id) ? 'rotate-90' : ''} aria-hidden="true">
              <polyline points="9 18 15 12 9 6"></polyline>
            </svg>
          </div>

          {#if expandedGroups.has(group.id)}
            <div id={`content-${group.id}`} role="region" class="p-4 space-y-4 animate-slide-down">
              {#each group.fields as field}
                <div class="space-y-1.5">
                  <label class="flex items-start gap-2 cursor-pointer">
                    <div class="flex-1 min-w-0">
                      <span class="text-sm font-medium text-foreground">{field.label}</span>
                      <p class="text-[11px] text-muted-foreground mt-0.5">{field.description}</p>
                    </div>
                    {#if field.type === 'checkbox'}
                      <input
                        type="checkbox"
                        class="w-4 h-4 mt-0.5 rounded border-border bg-surface accent-primary cursor-pointer shrink-0"
                        checked={getNestedValue(settingsStore, `${field.bind}.${field.key}`)}
                        onchange={(e) => {
                          const target = e.target as HTMLInputElement;
                          setNestedValue(settingsStore, `${field.bind}.${field.key}`, target.checked);
                          const setter = (settingsStore as any)[`set${field.bind.charAt(0).toUpperCase() + field.bind.slice(1)}`];
                          if (setter) setter({ [field.key]: target.checked });
                        }}
                      />
                    {:else if field.type === 'select'}
                      <select
                        class="input w-auto min-w-[180px] text-sm shrink-0"
                        value={getNestedValue(settingsStore, `${field.bind}.${field.key}`)}
                        onchange={(e) => {
                          const target = e.target as HTMLSelectElement;
                          setNestedValue(settingsStore, `${field.bind}.${field.key}`, target.value);
                          const setter = (settingsStore as any)[`set${field.bind.charAt(0).toUpperCase() + field.bind.slice(1)}`];
                          if (setter) setter({ [field.key]: target.value });
                        }}
                      >
                        {#each field.options as opt}
                          <option value={opt.value}>{opt.label}</option>
                        {/each}
                      </select>
                    {:else if field.type === 'number'}
                      <input
                        type="number"
                        class="input w-auto min-w-[100px] text-sm shrink-0"
                        value={getNestedValue(settingsStore, `${field.bind}.${field.key}`)}
                        min={field.min}
                        max={field.max}
                        step={field.step}
                        onchange={(e) => {
                          const target = e.target as HTMLInputElement;
                          const value = parseInt(target.value, 10);
                          setNestedValue(settingsStore, `${field.bind}.${field.key}`, value);
                          const setter = (settingsStore as any)[`set${field.bind.charAt(0).toUpperCase() + field.bind.slice(1)}`];
                          if (setter) setter({ [field.key]: value });
                        }}
                      />
                    {:else if field.type === 'text'}
                      <input
                        type="text"
                        class="input w-auto min-w-[240px] max-w-[400px] text-sm shrink-0 font-mono text-xs"
                        value={getNestedValue(settingsStore, `${field.bind}.${field.key}`)}
                        placeholder={field.placeholder}
                        onchange={(e) => {
                          const target = e.target as HTMLInputElement;
                          setNestedValue(settingsStore, `${field.bind}.${field.key}`, target.value);
                          const setter = (settingsStore as any)[`set${field.bind.charAt(0).toUpperCase() + field.bind.slice(1)}`];
                          if (setter) setter({ [field.key]: target.value });
                        }}
                      />
                    {:else if field.type === 'button'}
                      <button
                        class="btn-danger text-xs h-8 shrink-0"
                        onclick={() => handleAction(field.action)}
                      >
                        {field.label}
                      </button>
                    {/if}
                  </label>
                </div>
              {/each}
            </div>
          {/if}
        </section>
      {/each}
    </div>
  </div>

  <footer class="px-5 py-3 bg-surface border-t border-border shrink-0">
    <div class="flex items-center justify-between text-[11px] text-muted-foreground">
      <span>Settings are saved automatically</span>
      <span class="font-mono">v2.0.0</span>
    </div>
  </footer>
</div>

<style>
  @keyframes slide-down {
    from { opacity: 0; transform: translateY(-8px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .animate-slide-down { animation: slide-down 0.2s ease-out; }
</style>