<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import UpdateCard from "$lib/components/UpdateCard.svelte";
  import type { IconName } from "$lib/icons";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { navigationStore } from "$lib/stores/navigation.svelte";
  import { RETENTION_OPTIONS } from "$lib/utils/retention";
  import { api } from "$lib/services/api";
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
          key: "confirmDelete",
          label: "Confirm Before Deleting",
          description: "Show confirmation dialog when deleting messages",
          type: "checkbox" as const,
          bind: "general",
        },
        {
          key: "portRefreshInterval",
          label: "Port Refresh Interval (seconds)",
          description:
            "How often the port list is re-enumerated in the background, so a re-plugged modem appears without pressing Refresh. 5–300 seconds; 0 turns it off. Skipped while a scan, live session, SIM lookup or delete is running.",
          type: "number" as const,
          bind: "general",
          min: 0,
          max: 300,
          step: 5,
        },
        {
          key: "retentionHours",
          label: "Message Retention Period",
          description:
            "How long a received SMS is kept before it is deleted from the inbox and from SIM storage. One hour is the only window offered — a code is spent within minutes of arriving, and anything still on the card after that is a code someone else can still read.",
          type: "select" as const,
          bind: "general",
          options: RETENTION_OPTIONS,
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
          description:
            "Show an in-app notification when a new OTP arrives. Error and status messages are always shown.",
          type: "checkbox" as const,
          bind: "notifications",
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
      id: "forwarding",
      label: "Forwarding",
      icon: "rocket" as IconName,
      description: "Push OTPs to a private Telegram group",
      fields: [
        {
          key: "enabled",
          label: "Forward to Telegram",
          description:
            "Send incoming messages to the group below while Live mode runs. Applied when Live mode starts, so a change here takes a Stop → Start.",
          type: "checkbox" as const,
          bind: "forwarding",
        },
        {
          key: "botToken",
          label: "Bot Token",
          description:
            "The token @BotFather gave you, in the form 123456789:AA… Stored in this profile as plain text like every other setting — it is not encrypted. If it ever leaks, run /revoke in @BotFather.",
          type: "secret" as const,
          bind: "forwarding",
          placeholder: "123456789:AA…",
        },
        {
          key: "verifyToken",
          label: "Verify Token",
          description: "Ask Telegram who this token belongs to, without sending anything",
          type: "button" as const,
          action: "verifyTelegramToken",
          buttonText: "Verify",
          variant: "secondary",
        },
        {
          key: "chatId",
          label: "Destination Group ID",
          description:
            "Where every forwarded message goes. Add the bot to your private group, then press Detect — membership in that group is the whole permission model.",
          type: "text" as const,
          bind: "forwarding",
          placeholder: "-1001234567890",
        },
        {
          key: "detectGroup",
          label: "Detect Group ID",
          description:
            "Reads the group the bot was most recently added to. Telegram keeps that notice for 24 hours only; after that, send /start@your_bot in the group first.",
          type: "button" as const,
          action: "detectTelegramGroup",
          buttonText: "Detect",
          variant: "primary",
        },
        {
          key: "proxyUrl",
          label: "SOCKS5 Proxy",
          description:
            "Only for networks that block api.telegram.org. Example: socks5h://127.0.0.1:9050 — leave empty to connect directly. An MTProto proxy link will not work here; those only serve Telegram's own apps.",
          type: "text" as const,
          bind: "forwarding",
          placeholder: "socks5h://127.0.0.1:9050",
        },
        {
          key: "sendTest",
          label: "Send Test Message",
          description:
            "Proves the token, the group ID, the bot's membership and the network path in one shot",
          type: "button" as const,
          action: "sendTelegramTest",
          buttonText: "Send Test",
          variant: "secondary",
        },
        {
          key: "forwardOtp",
          label: "Forward Messages With a Code",
          description: "Send messages an OTP was extracted from",
          type: "checkbox" as const,
          bind: "forwarding",
        },
        {
          key: "forwardNonOtp",
          label: "Forward Messages Without a Code",
          description:
            "Send ordinary SMS too. Off by default: Telegram accepts 20 messages a minute into a group, and a bank of promotional SMS would spend that budget ahead of the codes.",
          type: "checkbox" as const,
          bind: "forwarding",
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

  // A caller may ask for a specific category (the update card links here).
  const requested = navigationStore.takeSettingsGroup();
  let selectedId = $state<string>(
    settingsGroups.some((g) => g.id === requested) ? requested! : "general",
  );
  let appVersion = $state<string | null>(null);
  /**
   * Whether the bot token is shown in the clear. Off by default so the field is
   * not readable over a shoulder, but toggleable because the operator has to be
   * able to confirm a paste — and hiding it forever would only hide it from the
   * person who is allowed to see it.
   */
  let revealSecrets = $state(false);
  /**
   * Which button-field action is mid-flight, or `null`.
   *
   * The Telegram actions are network calls with a 15-second timeout, so without
   * this the operator has no way to tell a pressed button from an unpressed one
   * and presses again — which fires a second request, and for Detect can consume
   * the update that the first call was going to read.
   */
  let pendingAction = $state<string | null>(null);

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
    if (pendingAction) return;
    pendingAction = action;
    try {
      await runAction(action);
    } finally {
      pendingAction = null;
    }
  }

  async function runAction(action: string) {
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
    } else if (action === "openLogFolder") {
      await api.openLogFolder();
    } else if (action === "clearLogs") {
      await api.clearLogs();
    } else if (action === "verifyTelegramToken") {
      await api.verifyTelegramToken();
    } else if (action === "detectTelegramGroup") {
      await api.detectTelegramGroup();
    } else if (action === "sendTelegramTest") {
      await api.sendTelegramTest();
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
                    disabled={pendingAction !== null}
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
                      disabled={pendingAction !== null}
                      onclick={() => handleAction(field.action)}
                    >
                      {#if pendingAction === field.action}
                        <Icon name="loader" size={14} class="animate-spin" />
                        <span>Working…</span>
                      {:else}
                        {field.buttonText ?? 'Execute'}
                      {/if}
                    </button>
                  {:else if field.type === "checkbox" || field.type === "select" || field.type === "number" || field.type === "text" || field.type === "secret"}
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
                    <div class="shrink-0 flex items-center gap-2">
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
                      {:else if field.type === "text" || field.type === "secret"}
                        <input
                          id={fieldId(selectedGroup, field)}
                          type={field.type === "secret" && !revealSecrets ? "password" : "text"}
                          class="input w-[220px] sm:w-[260px] max-w-full text-xs sm:text-sm font-mono"
                          autocomplete="off"
                          spellcheck="false"
                          placeholder={(field as any).placeholder ?? ""}
                          value={getNestedValue(
                            settingsStore,
                            `${field.bind}.${field.key}`,
                          )}
                          onchange={(e) => {
                            const target = e.target as HTMLInputElement;
                            // Trimmed on the way in: a pasted token or group id
                            // routinely carries a trailing space, and Telegram
                            // answers a bare 401 for it. Rust trims again — the
                            // store is rehydrated from localStorage and this is
                            // convenience, not validation.
                            const value = target.value.trim();
                            setNestedValue(
                              settingsStore,
                              `${field.bind}.${field.key}`,
                              value,
                            );
                            const setter = setterFor(field.bind);
                            if (setter) setter({ [field.key]: value });
                          }}
                        />
                        {#if field.type === "secret"}
                          <button
                            type="button"
                            class="text-[11px] text-muted-foreground hover:text-foreground underline underline-offset-2 shrink-0"
                            onclick={() => (revealSecrets = !revealSecrets)}
                          >
                            {revealSecrets ? "Hide" : "Show"}
                          </button>
                        {/if}
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
            {#if selectedGroup.id === "updates"}
              <!-- The check button, release notes and install flow live together
                   in one component so the panel can hide the check row while a
                   version is waiting to be installed. -->
              <UpdateCard />
            {/if}
          {/if}
        </section>

        <!-- Helper note -->
        <p class="text-[11px] text-muted-foreground/70 mt-3 px-1">
          {#if selectedGroup.id === "appearance"}
            Theme and column visibility apply immediately.
          {:else if selectedGroup.id === "updates"}
            Updates are fetched from the official release endpoint only.
          {:else if selectedGroup.id === "forwarding"}
            Forwarding needs Live mode running and the app window open. The bot posts
            only to the group above and never reads private chats.
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
