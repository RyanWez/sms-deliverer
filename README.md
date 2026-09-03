<p align="center">
  <img src="src-tauri/icons/icon.png" width="96" height="96" alt="SIM Bank SMS Reader Icon" />
</p>

<h1 align="center">SIM Bank SMS Reader</h1>

<p align="center">
  <b>A high-performance, multi-port GSM modem manager and real-time SMS/OTP reader built with Tauri v2, Rust, and Svelte 5.</b>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-v2.0-blue.svg?style=flat-square&logo=tauri" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/Rust-2021%20Edition-orange.svg?style=flat-square&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/Svelte-v5-ff3e00.svg?style=flat-square&logo=svelte" alt="Svelte 5" />
  <img src="https://img.shields.io/badge/TypeScript-5.x-3178c6.svg?style=flat-square&logo=typescript" alt="TypeScript" />
  <img src="https://img.shields.io/badge/Platform-Linux%20%7C%20Windows-lightgrey.svg?style=flat-square" alt="Platform" />
</p>

---

## 📋 Table of Contents

- [About the Project](#-about-the-project)
- [Key Features](#-key-features)
- [System Requirements](#-system-requirements)
  - [Linux Requirements & Dependencies](#linux-requirements--dependencies)
  - [Windows Requirements & Dependencies](#windows-requirements--dependencies)
- [Development Setup](#-development-setup)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
- [Running & Building with Tauri](#-running--building-with-tauri)
  - [Development Mode](#development-mode)
  - [Production Packaging / Build](#production-packaging--build)
  - [Web Browser Preview Mode](#web-browser-preview-mode)
- [Physical GSM Modem & SIM Bank Usage](#-physical-gsm-modem--sim-bank-usage)
  - [Supported Hardware](#supported-hardware)
  - [Serial Communication Settings](#serial-communication-settings)
  - [Linux Permissions & Port Access](#linux-permissions--port-access)
  - [Resolving ModemManager Conflicts on Linux](#resolving-modemmanager-conflicts-on-linux)
  - [AT Command Execution Flow](#at-command-execution-flow)
- [Telegram Forwarding](#-telegram-forwarding)
- [Safety & Operational Guidelines](#-safety--operational-guidelines)
- [Build, Check & Testing Commands](#-build-check--testing-commands)
- [Project Architecture](#-project-architecture)

---

## 📖 About the Project

**SIM Bank SMS Reader** (`sms-tauri`) is a specialized desktop application engineered for high-density SMS collection, verification code retrieval (OTP extraction), and automated SIM pool management.

Designed to interface directly with multi-port USB SIM banks (such as 8, 16, 32, or 64-channel GSM modem pools) as well as individual cellular dongles, the application provides multi-threaded serial communication, robust PDU/Text SMS parsing, concatenated SMS reassembly, and automated USSD queries.

By leveraging **Tauri v2** and **Rust** on the backend paired with **Svelte 5** and **Tailwind CSS** on the frontend, it delivers native desktop performance with minimal memory and CPU footprints compared to traditional Electron-based tools.

---

## ✨ Key Features

- **Modem Detection Before Work**: A one-shot `AT` liveness probe (800 ms timeout, two attempts) identifies which serial ports actually have a modem behind them. A SIM bank publishes one serial device per channel whether or not a SIM is inserted, so on a partly-filled bank this is the difference between a scan that finishes in seconds and one that spends its full timeout chain on every empty slot. Ports that do not answer are deselected automatically and skipped by scan, live mode, USSD and SIM cleanup. Ports are opened with DTR/RTS asserted so both platforms present the same lines to the modem. A probe cannot wait out a bank that is still powering up, so re-run Detect a minute after connecting it rather than treating the first result as final.
- **Concurrent Multi-Port Scanning**: Parallel asynchronous workers read SMS across all connected serial ports (`COM1..N` on Windows, `/dev/ttyUSB*` / `/dev/ttyACM*` on Linux) simultaneously without UI freezing.
- **Live SMS Monitoring Mode**: Real-time asynchronous polling and unsolicited event listener (+CMTI notifications) that triggers instant desktop alerts and sounds on incoming SMS. A port is reported `LIVE` only after a modem answers; empty slots are labelled `NO MODEM` and excluded from the ready total instead of showing green.
- **Advanced SMS Decoding & Concatenation**:
  - Full GSM 7-bit default alphabet, 8-bit binary, and 16-bit UCS-2 (Unicode / multi-language / Myanmar unicode) decoding.
  - Automatic reassembly of multi-part concatenated SMS messages (TP-UDHI 8-bit & 16-bit reference numbers), with GSM-7 payloads decoded from the septet boundary that follows the User Data Header.
  - PDU mode primary parsing (`AT+CMGF=0`) with graceful fallback to AT Text mode (`AT+CMGF=1`).
- **Intelligent OTP / Verification Code Detection**:
  - Built-in heuristic and regex engine identifying 4–8 digit verification codes and multilingual patterns.
  - Instant one-click or auto-copy to clipboard.
- **USSD / SIM Phone Number Discovery**: Phone numbers are filed against the SIM's **ICCID**, not against the serial port. `/dev/ttyUSB*` numbering is handed out in enumeration order, so a reboot or a hotplug reshuffles it — keying on the port meant a number learned while a card was `ttyUSB43` reappeared on whatever unrelated stick was called `ttyUSB43` next time, which showed up as one number on two ports and as numbers on ports with no modem at all. Every probe reads `AT+CCID`, so the card identifies itself and its number follows it even if it is moved to another slot. Discovery itself escalates cheapest-first: `AT+CNUM` reads EF_MSISDN straight off the SIM (no network, answers in milliseconds), then the carrier's own-number USSD codes (`*88#`, then the `*124#` balance dialogue, whose reply usually echoes the MSISDN). Any USSD session left open by an earlier run is cancelled first, because firmware that still believes a dialogue is active rejects the next request with an instant `+CME ERROR: 100`, and a rejected code is retried once without the data-coding-scheme argument for firmware that will not accept it.
- **Message Management & Filtering**:
  - Filter by port, sender, date range, or OTP-only messages.
  - Table and card view modes with customizable pagination.
  - Batch message deletion directly from SIM storage (`AT+CMGD`).
- **Cryptographically Secured Auto-Updater**: Native Tauri auto-updater integration with minisign cryptographic signature verification. Settings → Updates presents the release notes for a found version before anything is downloaded, and keeps the download and the install as separate steps so the restart happens when the operator chooses it rather than mid-shift. Manual checks are rate limited to one request per minute.
- **Synthetic Browser Preview**: Built-in mock data provider allowing full frontend development and UI inspection inside standard web browsers without needing physical hardware connected. The updater joins in: a dev-only preview release exercises the notes box, the download progress and the restart prompt without cutting a real release.
- **Telegram Group Forwarding**: Settings → Forwarding pushes messages to one private Telegram group over a single outbound HTTPS connection, with an optional SOCKS5 proxy for networks that block `api.telegram.org`. Membership of that group *is* the permission model — the app stores one destination id and no per-person list, so access is granted and revoked with Telegram's own group tools. The bot is push-only, has no commands, and stays silent in private chats. Setup is verified in three steps (Verify token → Detect group → Send test) rather than by waiting for the first real OTP to find out. See [Telegram Forwarding](#-telegram-forwarding).

---

## 💻 System Requirements

### General Prerequisites

- **Node.js**: v18.0.0+ or v20.0.0+ (LTS recommended)
- **Rust**: `rustc` and `cargo` (1.75.0+ / 2021 edition)
- **C/C++ Build Toolchain**: Native compilation tools for your OS.

---

### Linux Requirements & Dependencies

On Debian/Ubuntu-based distributions, install the required development libraries for Tauri 2 and serial communication:

```bash
sudo apt update && sudo apt install -y \
  build-essential \
  curl \
  wget \
  file \
  pkg-config \
  libssl-dev \
  libudev-dev \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libxdo-dev
```

*For Fedora/RHEL:*
```bash
sudo dnf install -y \
  gcc \
  gcc-c++ \
  pkg-config \
  openssl-devel \
  systemd-devel \
  gtk3-devel \
  webkit2gtk4.1-devel \
  libappindicator-gtk3-devel \
  librsvg2-devel \
  libxdo-devel
```

*For Arch Linux:*
```bash
sudo pacman -S --needed \
  base-devel \
  curl \
  wget \
  openssl \
  systemd-libs \
  gtk3 \
  webkit2gtk-4.1 \
  libayatana-appindicator \
  librsvg \
  xdotool
```

---

### Windows Requirements & Dependencies

1. **Microsoft Visual Studio C++ Build Tools**:
   - Install the **Desktop development with C++** workload via the [Visual Studio Installer](https://visualstudio.microsoft.com/visual-cpp-build-tools/).
2. **WebView2 Runtime**:
   - Pre-installed on Windows 10 (version 1803+) and Windows 11. (Bootstrapper is configured in `tauri.conf.json`).
3. **USB-to-UART / Serial Drivers**:
   - Install the appropriate driver for your SIM bank hardware chipset:
     - **CH340 / CH341**: Common on multi-port USB hubs.
     - **FTDI FT232 / FT4232**: High-speed multi-channel USB UART.
     - **Silicon Labs CP210x** or **Prolific PL2303**.

---

## 🚀 Development Setup

### 1. Clone the Repository

```bash
git clone https://github.com/RyanWez/sms-deliverer.git
cd sms-deliverer
```

### 2. Install Frontend Dependencies

```bash
npm install
```

---

## 🛠️ Running & Building with Tauri

### Development Mode

To launch the full desktop application with hot-reloading for both the Rust backend and Svelte frontend:

```bash
npm run dev
# in a separate terminal or run tauri dev directly:
npm run tauri dev
```

> **Note**: `npm run tauri dev` automatically launches `npm run dev` (Vite dev server on `http://localhost:1420`) as defined in `src-tauri/tauri.conf.json`.

---

### Production Packaging / Build

To compile and produce optimized production installers (`.deb`, `.AppImage` on Linux; `.exe` / `.msi` on Windows):

```bash
npm run tauri build
```

Built artifacts will be generated in:
- **Linux**: `src-tauri/target/release/bundle/deb/` and `src-tauri/target/release/bundle/appimage/`
- **Windows**: `src-tauri/target/release/bundle/nsis/` and `src-tauri/target/release/bundle/msi/`

---

### Web Browser Preview Mode

If you are designing UI or testing frontend layouts without physical modems or a local Rust setup:

```bash
npm run dev
```

Open `http://localhost:1420` in your web browser. The app will automatically detect non-Tauri environments and initialize simulated ports and synthetic incoming SMS data.

---

## 📡 Physical GSM Modem & SIM Bank Usage

### Supported Hardware

- **Multi-Port USB SIM Pools / Banks**: 8, 16, 32, 64-port SIM banks based on Quectel, SIMCom (SIM800C/SIM900), Wavecom, or Huawei modules.
- **Single USB Dongles & Modems**: Standard 3G/4G USB modems exposing AT command serial interfaces.

---

### Serial Communication Settings

The application connects to serial interfaces using the following default configuration:

| Parameter | Value |
| :--- | :--- |
| **Baud Rate** | `115200` bps |
| **Data Bits** | `8` |
| **Parity** | `None` |
| **Stop Bits** | `1` |
| **Flow Control** | `None` |
| **Timeout** | `100 ms` non-blocking slice |

---

### Linux Permissions & Port Access

On Linux, serial devices are assigned to `/dev/ttyUSB0..N` or `/dev/ttyACM0..N` and belong to the `dialout` or `uucp` group. 

To grant your user permission to access serial ports without running as root:

```bash
# Ubuntu / Debian
sudo usermod -aG dialout $USER

# Arch Linux / Fedora
sudo usermod -aG uucp $USER
```

> **Important**: You must log out and log back in (or restart your machine) for group changes to take effect.

---

### Resolving ModemManager Conflicts on Linux

On many Linux distributions, `ModemManager` runs by default. It automatically claims new `/dev/ttyUSB*` ports and sends probe AT commands, which interferes with port scanning or locks the device.

To prevent `ModemManager` from interfering:

```bash
# Stop and disable ModemManager
sudo systemctl stop ModemManager
sudo systemctl disable ModemManager
sudo systemctl mask ModemManager
```

*Alternatively, create a udev rule (`/etc/udev/rules.d/99-simbank-ignore.rules`) to ignore specific vendor/product IDs:*
```udev
ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523", ENV{ID_MM_DEVICE_IGNORE}="1"
```
Reload udev rules:
```bash
sudo udevadm control --reload-rules && sudo udevadm trigger
```

---

### AT Command Execution Flow

Every port-touching operation opens with the same liveness gate, because a
device node existing says nothing about a modem being present:

0. **Liveness probe** (gate for scan, live mode, USSD and delete):
   - `AT`: sent with an 800 ms timeout, up to twice. Any final result code
     (`OK`, `ERROR`, `+CME ERROR`) counts as alive — a modem in a bad command
     state is still worth talking to. No answer after both attempts and the port
     is reported `Modem not responding` immediately, in about a second, instead
     of running the sequences below against silence.
   - A failed write is reported as `Serial I/O failed: …`, never as
     `Modem not responding`. Windows applies the port timeout to writes as well
     as reads, so a stalled bridge sends nothing; blaming the modem for not
     answering a command that never left the host sends you looking for a missing
     SIM. Only the probe's own silence marks a slot as empty.
   - Every port is opened with DTR and RTS asserted, so that both platforms
     present the same lines to the modem. Linux raises them in the USB-serial
     driver, Windows drives them low until told otherwise, and a module whose
     stored profile has hardware flow control on (`AT&K3`) reads low RTS as "the
     host cannot accept data" and withholds every reply. No bank here has been
     shown to need this — it is hardening against a platform difference, not a fix
     for a diagnosed fault.
   - `AT+CCID` (then `AT+ICCID`, `AT^ICCID` for vendors that spell it their own
     way): identifies the card while the port is already open. A slot that stays
     silent is recorded as holding no card, so its previous tenant's number stops
     being displayed there.
   - **The probe cannot wait out a bank that is still powering up.** A module that
     has not finished booting stays silent for minutes; on Linux its `/dev/ttyUSB*`
     node may not exist yet at all, while on Windows the `COM` port persists in the
     registry and opens cleanly without anything answering behind it. Give the bank
     a minute after connecting it and run Detect Modems again rather than reading
     the first result as the final one.

1. **Initialization**:
   - `ATE0`: Disable command echo.
   - `AT+CSCS="GSM"` before an `AT+CNUM` read, `AT+CSCS="UCS2"` before a text-mode
     list: the character set is set per operation, not once when the port opens.
2. **SIM & Registration Status**:
   - `AT+CCID`: Card serial, the key every phone number is filed against.
   - `AT+CUSD=2` then `AT+CNUM`: Cancel a stale USSD session, then read the
     subscriber number off the SIM before any network query is attempted.
   - `AT+CREG?`: Registration state. No result code here means a wedged modem
     and the USSD queries are skipped rather than attempted.
   - `AT+CSQ`: Signal quality check.
3. **Reading SMS**:
   - `AT+CMGF=0`: Switch to PDU mode (reads raw hex PDUs for Unicode/UCS-2 and concatenated headers).
   - `AT+CMGL=4`: List all received messages.
   - `AT+CMGF=1`: Fallback to Text mode if PDU mode is unsupported by legacy modems.
4. **Live Notifications**:
   - `AT+CNMI=2,1,0,0,0`: Configure modem to route new message indications (+CMTI) to the terminal.
5. **Message Deletion**:
   - `AT+CMGD=<index>`: Delete processed message by index to free up SIM storage.

#### Cost of an unpopulated slot

Without the probe, each timeout in a sequence is paid in full before the next
command is tried. On a 64-port bank holding 7 SIMs that is where the wait came
from:

| Operation | Timeout chain on a silent port | With the probe |
|---|---|---|
| Scan (`read_port`) | `+CMGF=0` 4 s → `+CSCS` 4 s → `CMGL="ALL"` 15 s → 24 s | ~1.6 s |
| Get SIM (`get_sim_number`) | `ATE0`/`CSCS` 6 s → `CUSD=2`/`CNUM` 3.5 s → `CREG?`/`CSQ` 8 s → `*88#` 9 s → `*124#` 9 s → 38 s | ~1.6 s |
| Live startup | `+CMGF=0` 4 s → `CNMI` 3 s → `CMGL="ALL"` 15 s → 22 s | ~1.6 s |

---

## 📨 Telegram Forwarding

Settings → **Forwarding** configures a Telegram bot that pushes messages into one
private group, so a code is on every team member's phone instead of only on the
screen of whoever is sitting at the bank.

**The group is the allowlist.** The app holds exactly one destination chat id and
nothing else — no per-person list. Whoever an admin invites to that group receives
everything; whoever is removed receives nothing more, immediately, using Telegram's
own group tools. The bot is push-only: it has no commands and never answers a
private chat, so a stranger who finds its username gets silence.

### Setup

1. Create the bot with [@BotFather](https://t.me/BotFather) and copy the token.
2. Create a **private** group and add the bot to it. No admin rights are needed.
3. Paste the token into **Bot Token** and press **Verify** — this asks Telegram
   who the token belongs to and reports the bot's `@name`.
4. Press **Detect** to fill in the group id.
5. Press **Send Test** to prove the token, the id, the bot's membership and the
   network path in one shot.
6. Switch on **Forward to Telegram**, then start Live mode.

The forwarding settings are read when Live mode **starts**, so changing the token,
the group or the filters takes a Stop → Start to apply. This matches how the
retention window already behaves.

### What arrives, and how fast

One bubble per message while the bank is quiet: the code in a tap-to-copy block,
the SIM's own number, the sender, and the message body.

Under load the queue **coalesces** — several codes in one message. That is not a
cosmetic choice. Telegram accepts about 20 messages per minute into a group, so
one-bubble-per-code with three-second pacing sits exactly on the ceiling, and a
64-stick bank taking a burst would build a backlog that delivers codes after they
expire. The limit counts messages rather than codes, so packing up to ten per
message removes throughput as the constraint. A `429` is honoured by waiting out
the `retry_after` Telegram supplies.

A concatenated SMS — the normal shape for Burmese text, which fits 70 characters
per part in UCS-2 — is posted as its first fragment and then **edited in place**
once the rest arrives, rather than appearing twice.

Two things about step 4 that will otherwise look like bugs:

- Telegram keeps the "bot was added to a group" notice for **24 hours only**. If
  Detect finds nothing, send `/start@your_bot` inside the group and press it
  again — with Group Privacy at its default the bot cannot see ordinary group
  chatter, so a command addressed to it is the reliable way to produce a fresh
  update.
- If Detect reports a **basic group**, upgrade it. A basic group's chat id changes
  the moment it becomes a supergroup — which happens on its own when someone
  enables auto-delete or admin-only posting. The app heals this when it happens
  (Telegram returns the replacement id and it is saved automatically), but
  starting out as a supergroup avoids the round trip.

### Reaching api.telegram.org

The app makes one outbound HTTPS connection to `api.telegram.org:443`. Nothing
listens and nothing accepts inbound traffic.

Where an ISP blocks that host, fill in **SOCKS5 Proxy** (`socks5h://127.0.0.1:9050`).
Bot API traffic works through a SOCKS5 proxy. An MTProto proxy link will **not**
work here — those serve Telegram's own client apps, not the Bot API.

### What it costs you

- **Every group member sees every message.** That is the design: shared visibility
  over compartmentalisation. Protect group invites like passwords, and restrict
  "Add Members" to admins.
- **Forwarding needs Live mode running and the app window open.** There is no tray
  icon, so closing the window ends the process and the forwarding with it.
- **Messages without a code are not forwarded** unless you switch that on, and it
  spends the same 20-per-minute budget the codes need.
- **History lives on Telegram's servers**, not just in the app's 2-hour window.
  Turn on the group's auto-delete timer if that matters.
- **The bot token is stored in the clear**, like every other setting in this app —
  there is no keychain integration and the UI does not pretend otherwise. Anyone
  with the token can post to the group; `/revoke` in @BotFather is the recovery
  path. The token never reaches a log: it lives in the request URL, so every error
  leaving the Telegram module is redacted before it can reach `app.log` or the
  Logs page.

---

## ⚠️ Safety & Operational Guidelines

- 🔒 **SIM Card PIN & PUK Lockout**:
  Ensure all SIM cards inserted into the SIM bank have **PIN lock disabled** prior to insertion. Modems attempting automated queries on PIN-locked SIMs can exhaust PIN attempts and cause permanent PUK locks.
- ⚡ **Power Supply Stability**:
  High-density SIM banks (16/32/64 ports) consume significant instantaneous current during concurrent cellular handshakes. Always use a dedicated, high-amperage external power supply (typically 5V/12V DC). Power drops can cause USB hub resets and serial port dropouts.
- 📶 **Antenna & RF Interference**:
  Ensure all RF antennas are properly screwed into their respective SMA/IPEX connectors. Operating GSM transmitters without antennas can damage transceiver PAs (Power Amplifiers) and result in severe packet loss.
- 🛑 **Carrier Rate Limits & Anti-Spam**:
  Do not initiate rapid, automated USSD polling loops on the same cellular carrier. Excessive USSD requests can trigger carrier-side SIM card suspensions.
- 🛡️ **Sensitive OTP & Data Protection**:
  SMS messages and two-factor authentication (2FA) OTP tokens contain sensitive security credentials. Always handle exported logs and configurations responsibly and avoid exposing raw PDU dumps in public environments.

---

## 🔍 Build, Check & Testing Commands

### Frontend Verification

```bash
# Run Svelte and TypeScript typechecking
npm run check

# Run frontend unit tests
#
# These use Node's built-in test runner plus --experimental-strip-types rather
# than a separate framework, so they add no dependencies. Node 22+ required.
npm test

# Build frontend assets with Vite
npm run build

# Preview built production bundle locally
npm run preview
```

### Rust Backend Verification & Tests

```bash
# Fast compile & syntax check
cargo check --manifest-path src-tauri/Cargo.toml

# Run all unit tests (decoder, AT parser, modem probe, reassembly, logging)
cargo test --manifest-path src-tauri/Cargo.toml

# Run Clippy linter for code health and warnings
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# Check Rust code formatting
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

---

## 🏗️ Project Architecture

```
sms-tauri/
├── src/                          # Svelte 5 Frontend
│   ├── lib/
│   │   ├── components/           # UI components (MessageTable, FilterBar, NavRail, etc.)
│   │   ├── pages/                # Views (Inbox, Ports, Settings)
│   │   ├── services/             # Tauri IPC bridge & synthetic data layer
│   │   ├── stores/               # Svelte 5 runes-based reactive state stores
│   │   └── utils/                # Helper utilities (PDU preview, formatting, synthetic mocks)
│   ├── App.svelte                # Root application layout
│   └── app.css                   # Tailwind CSS & global styling
├── src-tauri/                    # Rust Tauri v2 Backend
│   ├── icons/                    # Application icons (.ico, .icns, .png)
│   ├── src/
│   │   ├── commands/             # Tauri IPC command handlers
│   │   ├── core/                 # Core engine modules
│   │   │   ├── at.rs             # AT command line-oriented serial transport
│   │   │   ├── decoder.rs        # PDU / 7-bit / UCS-2 decoder & regex OTP extractor
│   │   │   ├── modem.rs          # Serial port discovery & batch reader
│   │   │   ├── reassemble.rs     # Concatenated multi-part SMS reassembly
│   │   │   ├── live.rs           # Background live listener & notification dispatcher
│   │   │   └── settings.rs       # Persistent application settings
│   │   ├── logging.rs            # Thread-safe dual logger (stdout + file)
│   │   ├── lib.rs                # Tauri builder & plugin registration
│   │   └── main.rs               # Desktop executable entrypoint
│   ├── Cargo.toml                # Rust dependencies & crate metadata
│   └── tauri.conf.json           # Tauri v2 configuration & window settings
├── package.json                  # Node.js dependencies & scripts
└── tsconfig.json                 # TypeScript compiler configuration
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE) (or your designated project license).
