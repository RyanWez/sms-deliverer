<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import type { IconName } from "$lib/icons";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { api } from "$lib/services/api";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process"; // We will add plugin-process just in case, or catch error

  const settingsGroups = [
    {
      id: "general",
      label: "General",
      icon: "monitor" as IconName,
      description: "Application behavior and startup options",
      fields: [
        {
          key: "autoStartLive",
          label: "Auto-start Live Mode",
          description:
            "Automatically start live monitoring on application launch",
          type: "checkbox" as const,
          bind: "general",
        },
        {
          key: "minimizeToTray",
          label: "Minimize to System Tray",
          description: "Keep the app running in the background when minimized",
          type: "checkbox" as const,
          bind: "general",
        },
        {
          key: "confirmDelete",
          label: "Confirm Before Deleting",
          description: "Show confirmation dialog when deleting messages",
          type: "checkbox" as const,
          bind: "general",
        },
        {
          key: "portRefreshInterval",
          label: "Port Refresh Interval (seconds)",
          description: "How often to automatically refresh the port list",
          type: "number" as const,
          bind: "general",
          min: 10,
          max: 300,
          step: 5,
        },
      ],
    },
    {
      id: "notifications",
      label: "Notifications",
      icon: "bell" as IconName,
      description: "Configure how and when you receive alerts",
      fields: [
        {
          key: "enabled",
          label: "Enable Notifications",
          description: "Show desktop notifications for new messages",
          type: "checkbox" as const,
          bind: "notifications",
        },
        {
          key: "soundEnabled",
          label: "Play Sound",
          description: "Play a notification sound when a new message arrives",
          type: "checkbox" as const,
          bind: "notifications",
        },
        {
          key: "desktopNotifications",
          label: "Desktop Notifications",
          description: "Show native OS notifications for new SMS",
          type: "checkbox" as const,
          bind: "notifications",
        },
        {
          key: "otpOnlyNotifications",
          label: "OTP Messages Only",
          description: "Only notify for messages containing OTP codes",
          type: "checkbox" as const,
          bind: "notifications",
        },
      ],
    },
    {
      id: "otp",
      label: "OTP Settings",
      icon: "hash" as IconName,
      description: "One-time password detection and handling",
      fields: [
        {
          key: "autoCopy",
          label: "Auto-copy OTP to Clipboard",
          description: "Automatically copy detected OTP codes to clipboard",
          type: "checkbox" as const,
          bind: "otp",
        },
        {
          key: "showInTable",
          label: "Show OTP Column",
          description: "Display OTP codes in the message table",
          type: "checkbox" as const,
          bind: "otp",
        },
        {
          key: "highlightNewOtp",
          label: "Highlight New OTP",
          description: "Visually highlight newly received OTP messages",
          type: "checkbox" as const,
          bind: "otp",
        },
        {
          key: "otpPattern",
          label: "OTP Detection Pattern (Regex)",
          description:
            "Regular expression used to detect OTP codes in messages",
          type: "text" as const,
          bind: "otp",
          placeholder: "\\b(\\d{4,8})\\b",
        },
      ],
    },
    {
      id: "appearance",
      label: "Appearance",
      icon: "sun" as IconName,
      description: "Customize the look and feel of the application",
      fields: [
        {
          key: "theme",
          label: "Theme",
          description: "Choose the color theme for the application",
          type: "select" as const,
          bind: "appearance",
          options: [
            { value: "system", label: "System" },
            { value: "dark", label: "Dark" },
            { value: "light", label: "Light" },
          ],
        },
        {
          key: "compactMode",
          label: "Compact Mode",
          description: "Reduce spacing for more content on screen",
          type: "checkbox" as const,
          bind: "appearance",
        },
        {
          key: "showSIMColumn",
          label: "Show SIM Column",
          description: "Display SIM number column in message table",
          type: "checkbox" as const,
          bind: "appearance",
        },
        {
          key: "showPortColumn",
          label: "Show Port Column",
          description: "Display port name column in message table",
          type: "checkbox" as const,
          bind: "appearance",
        },
      ],
    },
    {
      id: "updates",
      label: "Updates",
      icon: "download" as IconName,
      description: "Application update preferences",
      fields: [
        {
          key: "autoCheck",
          label: "Automatically Check for Updates",
          description: "Check for new versions on startup",
          type: "checkbox" as const,
          bind: "updates",
        },
        {
          key: "checkInterval",
          label: "Check Interval (hours)",
          description: "How often to check for updates in the background",
          type: "number" as const,
          bind: "updates",
          min: 1,
          max: 168,
          step: 1,
        },
        {
          key: "checkNow",
          label: "Check for Updates Now",
          description: "Check if a new version is available for download",
          type: "button" as const,
          action: "checkUpdates",
          variant: "primary",
        },
      ],
    },
    {
      id: "advanced",
      label: "Advanced",
      icon: "wrench" as IconName,
      description: "Advanced options and data management",
      fields: [
        {
          key: "resetSettings",
          label: "Reset All Settings to Defaults",
          description: "Restore all settings to their default values",
          type: "button" as const,
          action: "reset",
          variant: "danger",
        },
        {
          key: "clearMessages",
          label: "Clear All Messages",
          description: "Permanently delete all stored SMS messages",
          type: "button" as const,
          action: "clearMessages",
          variant: "danger",
        },
      ],
    },
  ];

  let expandedGroups = $state<Set<string>>(new Set(["general"]));

  function toggleGroup(id: string) {
    const next = new Set(expandedGroups);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expandedGroups = next;
  }

  async function handleAction(action: string) {
    if (action === "reset") {
      if (confirm("Reset all settings to defaults? This cannot be undone.")) {
        settingsStore.resetToDefaults();
      }
    } else if (action === "clearMessages") {
      if (confirm("Permanently delete ALL messages? This cannot be undone.")) {
        await api.clearAll();
      }
    } else if (action === "checkUpdates") {
      try {
        const update = await check();
        if (update) {
          if (confirm(`Version ${update.version} is available. Do you want to download and install it now?`)) {
            await update.downloadAndInstall();
            alert("Update installed successfully. The application will now restart.");
            try {
              await relaunch();
            } catch (e) {
              alert("Please restart the app manually to apply the update.");
            }
          }
        } else {
          alert("You are already on the latest version.");
        }
      } catch (error) {
        alert("Failed to check for updates: " + error);
      }
    }
  }

  function getNestedValue(obj: any, path: string): any {
    return path.split(".").reduce((o, k) => o?.[k], obj);
  }

  function setNestedValue(obj: any, path: string, value: any) {
    const keys = path.split(".");
    const last = keys.pop()!;
    const target = keys.reduce((o, k) => o[k], obj);
    target[last] = value;
  }

  function fieldId(group: { id: string }, field: { key: string }): string {
    return `field-${group.id}-${field.key}`;
  }

  function setterFor(bind: string) {
    return (settingsStore as any)[
      `set${bind.charAt(0).toUpperCase() + bind.slice(1)}`
    ];
  }
</script>

<div class="flex-1 flex flex-col h-full overflow-hidden min-h-0" id="panel-settings">
  <header class="page-header">
    <div class="min-w-0">
      <h1 class="page-title">Settings</h1>
      <p class="page-subtitle">
        Configure application preferences and behavior
      </p>
    </div>
  </header>

  <div class="flex-1 overflow-auto p-5 bg-background min-h-0">
    <div class="max-w-3xl mx-auto space-y-3">
      {#each settingsGroups as group (group.id)}
        <section
          class="card overflow-hidden"
          aria-labelledby={`heading-${group.id}`}
        >
          <h2 class="sr-only">{group.label}</h2>
          <button
            type="button"
            class="w-full flex items-center gap-3 px-4 py-3 border-b border-border/50 text-left transition-colors duration-150
                   hover:bg-elevated/40 focus:outline-none focus-visible:bg-elevated/40 focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary/70"
            onclick={() => toggleGroup(group.id)}
            aria-expanded={expandedGroups.has(group.id)}
            aria-controls={`content-${group.id}`}
            id={`heading-${group.id}`}
          >
            <span
              class="w-8 h-8 rounded-md bg-elevated flex items-center justify-center text-muted-foreground shrink-0"
            >
              <Icon name={group.icon} size={17} />
            </span>
            <span class="flex-1 min-w-0">
              <span class="block text-sm font-medium text-foreground truncate"
                >{group.label}</span
              >
              <span class="block text-[11px] text-muted-foreground truncate"
                >{group.description}</span
              >
            </span>
            <Icon
              name="chevron-right"
              size={16}
              class={`text-muted-foreground transition-transform duration-150 ${expandedGroups.has(group.id) ? "rotate-90" : ""}`}
            />
          </button>

          {#if expandedGroups.has(group.id)}
            <div
              id={`content-${group.id}`}
              role="region"
              aria-labelledby={`heading-${group.id}`}
              class="px-4 animate-fade-in"
            >
              <div class="divide-y divide-border/50">
                {#each group.fields as field (field.key)}
                  <div
                    class="py-3.5 first:pt-1 last:pb-1 flex items-center justify-between gap-6"
                  >
                    {#if field.type === "button"}
                      <div class="min-w-0">
                        <span class="block text-sm font-medium text-danger"
                          >{field.label}</span
                        >
                        <p class="text-[11px] text-muted-foreground mt-0.5">
                          {field.description}
                        </p>
                      </div>
                      <button
                        class={field.variant === "danger"
                          ? (field.action === "reset" ? "btn-danger-quiet shrink-0" : "btn-danger shrink-0")
                          : "btn-primary shrink-0"}
                        onclick={() => handleAction(field.action)}
                      >
                        {field.action === "reset" ? "Reset" : field.action === "checkUpdates" ? "Check" : "Clear"}
                      </button>
                    {:else}
                      <label
                        class="flex-1 min-w-0 cursor-pointer"
                        for={fieldId(group, field)}
                      >
                        <span class="block text-sm font-medium text-foreground"
                          >{field.label}</span
                        >
                        <span
                          class="block text-[11px] text-muted-foreground mt-0.5"
                          >{field.description}</span
                        >
                      </label>
                      {#if field.type === "checkbox"}
                        <input
                          id={fieldId(group, field)}
                          type="checkbox"
                          class="checkbox w-4 h-4"
                          checked={getNestedValue(
                            settingsStore,
                            `${field.bind}.${field.key}`,
                          )}
                          onchange={(e) => {
                            const target = e.target as HTMLInputElement;
                            setNestedValue(
                              settingsStore,
                              `${field.bind}.${field.key}`,
                              target.checked,
                            );
                            const setter = setterFor(field.bind);
                            if (setter) setter({ [field.key]: target.checked });
                          }}
                        />
                      {:else if field.type === "select"}
                        <select
                          id={fieldId(group, field)}
                          class="input w-auto min-w-[180px] max-w-[220px] text-sm shrink-0"
                          value={getNestedValue(
                            settingsStore,
                            `${field.bind}.${field.key}`,
                          )}
                          onchange={(e) => {
                            const target = e.target as HTMLSelectElement;
                            setNestedValue(
                              settingsStore,
                              `${field.bind}.${field.key}`,
                              target.value,
                            );
                            const setter = setterFor(field.bind);
                            if (setter) setter({ [field.key]: target.value });
                          }}
                        >
                          {#each field.options as opt (opt.value)}
                            <option value={opt.value}>{opt.label}</option>
                          {/each}
                        </select>
                      {:else if field.type === "number"}
                        <input
                          id={fieldId(group, field)}
                          type="number"
                          class="input w-auto min-w-[110px] max-w-[140px] text-sm shrink-0 font-mono tabular-nums"
                          value={getNestedValue(
                            settingsStore,
                            `${field.bind}.${field.key}`,
                          )}
                          min={field.min}
                          max={field.max}
                          step={field.step}
                          onchange={(e) => {
                            const target = e.target as HTMLInputElement;
                            const value = parseInt(target.value, 10);
                            setNestedValue(
                              settingsStore,
                              `${field.bind}.${field.key}`,
                              value,
                            );
                            const setter = setterFor(field.bind);
                            if (setter) setter({ [field.key]: value });
                          }}
                        />
                      {:else if field.type === "text"}
                        <input
                          id={fieldId(group, field)}
                          type="text"
                          class="input w-auto min-w-[240px] max-w-[320px] text-xs shrink-0 font-mono"
                          value={getNestedValue(
                            settingsStore,
                            `${field.bind}.${field.key}`,
                          )}
                          placeholder={field.placeholder}
                          spellcheck="false"
                          onchange={(e) => {
                            const target = e.target as HTMLInputElement;
                            setNestedValue(
                              settingsStore,
                              `${field.bind}.${field.key}`,
                              target.value,
                            );
                            const setter = setterFor(field.bind);
                            if (setter) setter({ [field.key]: target.value });
                          }}
                        />
                      {/if}
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </section>
      {/each}
    </div>
  </div>

  <footer class="page-footer">
    <span>Settings are saved automatically</span>
    <span class="font-mono">v2.0.0</span>
  </footer>
</div>
