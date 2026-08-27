<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import type { IconName } from "$lib/icons";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { api } from "$lib/services/api";
  import { runUpdateCheck } from "$lib/services/updater";
  import { confirm as nativeConfirm } from "@tauri-apps/plugin-dialog";
  import { getVersion } from "@tauri-apps/api/app";
  import { onMount } from "svelte";

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
        {
          key: "autoDeleteExpired",
          label: "Auto-Delete Expired Messages",
          description:
            "Automatically remove messages older than the retention period in the background",
          type: "checkbox" as const,
          bind: "general",
        },
        {
          key: "retentionHours",
          label: "Message Retention Period",
          description:
            "How long to keep received SMS before automatic background deletion",
          type: "select" as const,
          bind: "general",
          options: [
            { value: 1, label: "1 Hour" },
            { value: 2, label: "2 Hours (Default)" },
            { value: 4, label: "4 Hours" },
            { value: 8, label: "8 Hours" },
            { value: 24, label: "24 Hours (1 Day)" },
            { value: 168, label: "7 Days" },
          ],
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
          buttonText: "Check",
          variant: "primary",
        },
      ],
    },
    {
      id: "developer",
      label: "Developer",
      icon: "terminal" as IconName,
      description: "Diagnostics, real-time logging, and developer tools",
      fields: [
        {
          key: "enabled",
          label: "Developer Mode",
          description: "Enable Developer Mode to view live logs and backend diagnostics in the sidebar",
          type: "checkbox" as const,
          bind: "developer",
        },
        {
          key: "logLevel",
          label: "Capture Log Level",
          description: "Minimum severity level for capturing system logs",
          type: "select" as const,
          bind: "developer",
          options: [
            { value: "ALL", label: "All (Info, Warn, Error)" },
            { value: "INFO", label: "Info, Warn & Error" },
            { value: "WARN", label: "Warn & Error Only" },
            { value: "ERROR", label: "Error Only" },
          ],
        },
        {
          key: "autoScroll",
          label: "Auto-scroll Logs by Default",
          description: "Automatically scroll to bottom when new logs arrive in the console",
          type: "checkbox" as const,
          bind: "developer",
        },
        {
          key: "openLogFolder",
          label: "Open Log Directory",
          description: "Open the folder containing persistent application log files",
          type: "button" as const,
          action: "openLogFolder",
          buttonText: "Open Folder",
          variant: "secondary",
        },
        {
          key: "clearLogs",
          label: "Clear In-Memory Logs",
          description: "Clear all currently captured logs from memory",
          type: "button" as const,
          action: "clearLogs",
          buttonText: "Clear Logs",
          variant: "secondary",
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
          buttonText: "Reset",
          variant: "danger",
        },
        {
          key: "clearMessages",
          label: "Clear All Messages",
          description: "Permanently delete all stored SMS messages",
          type: "button" as const,
          action: "clearMessages",
          buttonText: "Delete All",
          variant: "danger",
        },
      ],
    },
  ];

  let selectedId = $state<string>("general");
  let updateChecking = $state(false);
  let appVersion = $state<string | null>(null);

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch {
      appVersion = null;
    }
  });

  const selectedGroup = $derived(
    settingsGroups.find((g) => g.id === selectedId) ?? settingsGroups[0]
  );

  async function handleAction(action: string) {
    if (action === "reset") {
      if (
        await nativeConfirm("Reset all settings to defaults? This cannot be undone.", {
          title: "Reset settings",
          kind: "warning",
          okLabel: "Reset",
          cancelLabel: "Cancel",
        })
      ) {
        settingsStore.resetToDefaults();
      }
    } else if (action === "clearMessages") {
      if (
        !settingsStore.general.confirmDelete ||
        (await nativeConfirm("Permanently delete ALL messages? This cannot be undone.", {
          title: "Clear all messages",
          kind: "warning",
          okLabel: "Delete all",
          cancelLabel: "Cancel",
        }))
      ) {
        await api.clearAll();
      }
    } else if (action === "checkUpdates") {
      if (updateChecking) return;
      updateChecking = true;
      try {
        await runUpdateCheck(true);
      } finally {
        updateChecking = false;
      }
    } else if (action === "openLogFolder") {
      await api.openLogFolder();
    } else if (action === "clearLogs") {
      await api.clearLogs();
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

  function onCategoryKeydown(e: KeyboardEvent) {
    const idx = settingsGroups.findIndex((g) => g.id === selectedId);
    if (e.key === "ArrowDown" || e.key === "ArrowRight") {
      e.preventDefault();
      const next = (idx + 1) % settingsGroups.length;
      selectedId = settingsGroups[next].id;
      // move focus to newly selected button
      queueMicrotask(() => {
        const el = document.querySelector<HTMLButtonElement>(`[data-cat="${selectedId}"]`);
        el?.focus();
      });
    } else if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
      e.preventDefault();
      const prev = (idx - 1 + settingsGroups.length) % settingsGroups.length;
      selectedId = settingsGroups[prev].id;
      queueMicrotask(() => {
        const el = document.querySelector<HTMLButtonElement>(`[data-cat="${selectedId}"]`);
        el?.focus();
      });
    } else if (e.key === "Home") {
      e.preventDefault();
      selectedId = settingsGroups[0].id;
    } else if (e.key === "End") {
      e.preventDefault();
      selectedId = settingsGroups[settingsGroups.length - 1].id;
    }
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

  <!-- Two-pane layout: left nav + right content -->
  <div class="flex-1 flex overflow-hidden min-h-0 bg-background
              flex-col sm:flex-row">
    <!-- Category navigation -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
    <nav
      class="shrink-0 flex flex-row sm:flex-col gap-1 p-2 sm:p-2 sm:py-3
             border-b sm:border-b-0 sm:border-r border-border bg-surface
             overflow-x-auto sm:overflow-y-auto sm:overflow-x-hidden
             w-full sm:w-[220px] lg:w-[240px]
             scrollbar-thin"
      aria-label="Settings categories"
      role="tablist"
      aria-orientation="vertical"
      onkeydown={onCategoryKeydown}
    >
      {#each settingsGroups as group (group.id)}
        {@const active = selectedId === group.id}
        <button
          type="button"
          data-cat={group.id}
          class="flex items-center gap-3 text-left rounded-md px-2.5 py-2.5 transition-colors duration-150
                 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/70 focus-visible:ring-offset-1 focus-visible:ring-offset-surface
                 min-w-[160px] sm:min-w-0 sm:w-full
                 {active
                   ? 'bg-primary/10 text-primary border border-primary/20 shadow-sm'
                   : 'border border-transparent text-muted-foreground hover:bg-elevated hover:text-foreground hover:border-border'}"
          aria-current={active ? 'page' : undefined}
          aria-selected={active}
          role="tab"
          tabindex={active ? 0 : -1}
          onclick={() => (selectedId = group.id)}
        >
          <span
            class="w-8 h-8 rounded-md flex items-center justify-center shrink-0
                   {active ? 'bg-primary/15 text-primary' : 'bg-elevated text-muted-foreground'}"
            aria-hidden="true"
          >
            <Icon name={group.icon} size={16} />
          </span>
          <span class="flex-1 min-w-0 hidden sm:block">
            <span class="block text-xs font-semibold leading-tight truncate {active ? 'text-primary' : 'text-foreground'}"
              >{group.label}</span
            >
            <span class="block text-[11px] leading-tight truncate {active ? 'text-primary/70' : 'text-muted-foreground'}"
              >{group.description}</span
            >
          </span>
          <!-- compact label for very narrow (mobile) horizontal strip: show label only -->
          <span class="sm:hidden text-xs font-medium truncate {active ? 'text-primary' : 'text-foreground'}">{group.label}</span>
          {#if active}
            <Icon name="chevron-right" size={14} class="hidden sm:block text-primary/60 shrink-0" />
          {/if}
        </button>
      {/each}
    </nav>

    <!-- Selected category content -->
    <div class="flex-1 overflow-auto min-w-0 min-h-0 bg-background">
      <div class="max-w-[720px] mx-auto p-4 sm:p-5 lg:p-6">
        <!-- Category header -->
        <div class="flex items-start gap-3 mb-4">
          <span class="w-9 h-9 rounded-lg bg-elevated border border-border flex items-center justify-center text-muted-foreground shrink-0">
            <Icon name={selectedGroup.icon} size={18} />
          </span>
          <div class="min-w-0">
            <h2 class="text-sm font-semibold text-foreground leading-tight">{selectedGroup.label}</h2>
            <p class="text-xs text-muted-foreground mt-0.5 leading-relaxed">{selectedGroup.description}</p>
          </div>
        </div>

        <section
          class="card overflow-hidden"
          aria-labelledby="settings-heading-{selectedGroup.id}"
        >
          <h3 id="settings-heading-{selectedGroup.id}" class="sr-only">{selectedGroup.label} settings</h3>

          {#if selectedGroup.id === "advanced"}
            <!-- Destructive zone: clearly separated -->
            <div class="p-3 bg-danger/[0.04] border-b border-danger/15 flex items-center gap-2">
              <span class="w-6 h-6 rounded-md bg-danger/10 text-danger flex items-center justify-center shrink-0">
                <Icon name="alert-triangle" size={13} strokeWidth={2} />
              </span>
              <div class="min-w-0">
                <div class="text-xs font-semibold text-danger leading-tight">Danger zone</div>
                <div class="text-[11px] text-muted-foreground leading-tight">Destructive actions — use with caution</div>
              </div>
            </div>
            <div class="divide-y divide-border/50">
              {#each selectedGroup.fields as field (field.key)}
                {@const f = field as any}
                <div class="px-4 py-3.5 flex items-center justify-between gap-6">
                  <div class="min-w-0 flex-1">
                    <span class="block text-sm font-medium text-danger">{field.label}</span>
                    <p class="text-[11px] text-muted-foreground mt-0.5 leading-relaxed">{field.description}</p>
                  </div>
                  <button
                    class={f.variant === "danger"
                      ? (f.action === "reset" ? "btn-danger-quiet shrink-0" : "btn-danger shrink-0")
                      : "btn-primary shrink-0"}
                    onclick={() => handleAction(f.action)}
                  >
                    {f.action === "reset" ? "Reset" : f.action === "clearMessages" ? "Clear" : "Action"}
                  </button>
                </div>
              {/each}
            </div>
          {:else}
            <div class="divide-y divide-border/50">
              {#each selectedGroup.fields as field (field.key)}
                <div class="px-4 py-3.5 flex items-center justify-between gap-4 sm:gap-6
                            flex-wrap sm:flex-nowrap">
                  {#if field.type === "button"}
                    <div class="min-w-0 flex-1">
                      <span class="block text-sm font-medium text-foreground">{field.label}</span>
                      <p class="text-[11px] text-muted-foreground mt-0.5 leading-relaxed">
                        {field.description}
                      </p>
                    </div>
                    {@const btnClass = field.variant === "danger"
                      ? "btn-danger"
                      : field.variant === "secondary"
                        ? "btn-secondary"
                        : "btn-primary"}
                    <button
                      class="{btnClass} shrink-0 inline-flex items-center justify-center gap-2 disabled:opacity-60 disabled:pointer-events-none text-xs sm:text-sm px-3.5 py-1.5 min-w-[76px]"
                      disabled={field.action === "checkUpdates" && updateChecking}
                      onclick={() => handleAction(field.action)}
                    >
                      {#if field.action === "checkUpdates" && updateChecking}
                        <span
                          class="inline-block w-3.5 h-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"
                          aria-hidden="true"></span>
                        Checking…
                      {:else}
                        {field.buttonText ?? 'Execute'}
                      {/if}
                    </button>
                  {:else if field.type === "checkbox" || field.type === "select" || field.type === "number" || field.type === "text"}
                    <label
                      class="flex-1 min-w-[200px] cursor-pointer"
                      for={fieldId(selectedGroup, field)}
                    >
                      <span class="block text-sm font-medium text-foreground leading-tight"
                        >{field.label}</span
                      >
                      <span
                        class="block text-[11px] text-muted-foreground mt-1 leading-relaxed"
                        >{field.description}</span
                      >
                    </label>
                    <div class="shrink-0 flex items-center">
                      {#if field.type === "checkbox"}
                        <input
                          id={fieldId(selectedGroup, field)}
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
                          id={fieldId(selectedGroup, field)}
                          class="input w-[180px] sm:w-[210px] max-w-full text-xs sm:text-sm shrink-0 truncate cursor-pointer pr-7"
                          value={getNestedValue(
                            settingsStore,
                            `${field.bind}.${field.key}`,
                          )}
                          onchange={(e) => {
                            const target = e.target as HTMLSelectElement;
                            const currentVal = getNestedValue(settingsStore, `${field.bind}.${field.key}`);
                            const parsedVal = typeof currentVal === 'number' && !Number.isNaN(Number(target.value))
                              ? Number(target.value)
                              : target.value;
                            setNestedValue(
                              settingsStore,
                              `${field.bind}.${field.key}`,
                              parsedVal,
                            );
                            const setter = setterFor(field.bind);
                            if (setter) setter({ [field.key]: parsedVal });
                          }}
                        >
                          {#each field.options as opt (opt.value)}
                            <option value={opt.value} class="truncate">{opt.label}</option>
                          {/each}
                        </select>
                      {:else if field.type === "number"}
                        <input
                          id={fieldId(selectedGroup, field)}
                          type="number"
                          class="input w-[130px] sm:w-[140px] text-sm shrink-0 font-mono tabular-nums"
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
                            if (!Number.isNaN(value)) {
                              setNestedValue(
                                settingsStore,
                                `${field.bind}.${field.key}`,
                                value,
                              );
                              const setter = setterFor(field.bind);
                              if (setter) setter({ [field.key]: value });
                            }
                          }}
                        />
                      {:else if field.type === "text"}
                        <input
                          id={fieldId(selectedGroup, field)}
                          type="text"
                          class="input w-[260px] sm:w-[300px] max-w-full text-xs shrink-0 font-mono"
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
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </section>

        <!-- Helper note -->
        <p class="text-[11px] text-muted-foreground/70 mt-3 px-1">
          {#if selectedGroup.id === "appearance"}
            Theme and column visibility apply immediately.
          {:else if selectedGroup.id === "otp"}
            Custom regex is applied to newly received messages.
          {:else if selectedGroup.id === "updates"}
            Updates are fetched from the official release endpoint only.
          {:else}
            Changes are saved automatically.
          {/if}
        </p>
      </div>
    </div>
  </div>

  <footer class="page-footer">
    <span>Settings are saved automatically</span>
    <span class="font-mono">{appVersion ? `v${appVersion}` : ""}</span>
  </footer>
</div>
