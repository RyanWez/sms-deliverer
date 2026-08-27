# Changelog

## [1.1.0](https://github.com/RyanWez/sms-deliverer/compare/sms-tauri-v1.0.1...sms-tauri-v1.1.0) (2026-08-27)


### Features

* add configurable confirmation for deletions, optional message table columns, and auto-start functionality for live mode ([e4b71b0](https://github.com/RyanWez/sms-deliverer/commit/e4b71b0d0145f7fdae324be7f51bb18c999613c4))
* add port status utilities and implement PortDetail component for side panel information display ([f8d338a](https://github.com/RyanWez/sms-deliverer/commit/f8d338a9c2b58da71edf02003f5b8304c80c230a))
* add scan and USSD busy states with UI loading indicators ([e0a2be0](https://github.com/RyanWez/sms-deliverer/commit/e0a2be0cdd397d3dd692cf10e8810e2f716f4ccc))
* add smooth transition animations for sidebar expansion and collapse states ([f450088](https://github.com/RyanWez/sms-deliverer/commit/f4500880f2f8ffee0df3001994be93f98a4668e6))
* add Tauri runtime detection to enable web browser fallback mode and lazy-load Tauri dependencies. ([85ad849](https://github.com/RyanWez/sms-deliverer/commit/85ad849b9814f3b5de93f25f1af94072f1c0e5ed))
* add updater and process capabilities and implement loading state for update checks in Settings ([be3f89c](https://github.com/RyanWez/sms-deliverer/commit/be3f89ca0668f006cde026b6508c5808d5564bbd))
* dynamically fetch and display app version in sidebar using Tauri API ([57878c1](https://github.com/RyanWez/sms-deliverer/commit/57878c180cce876cf0145aff05afb8ee562360b6))
* implement auto-updater functionality with signing keys, workflow automation, and UI integration ([f1c784c](https://github.com/RyanWez/sms-deliverer/commit/f1c784c92aef1b03de8f6c16b8c54034ba99cf92))
* implement message details view and refactor table UI with shared icon components ([452454f](https://github.com/RyanWez/sms-deliverer/commit/452454f5c8b2c47116389d0a353e58d54786bf85))
* implement SMS concatenation support and add configurable page size to pagination UI ([b843089](https://github.com/RyanWez/sms-deliverer/commit/b84308948a1f435bbe1b6539b78e3b55c8c4e5e5))
* replace SVG port icon with remote image asset in Ports page ([61428a6](https://github.com/RyanWez/sms-deliverer/commit/61428a6c8488dcf91876fbcec5a67e7b65d8f71c))
* update message grid layout, introduce fixed-page pagination, and improve port card interactivity ([39f2064](https://github.com/RyanWez/sms-deliverer/commit/39f20641036d46f31ffb0c96bd5917b6d95c9722))


### Bug Fixes

* add libudev-dev to Linux build dependencies ([5534216](https://github.com/RyanWez/sms-deliverer/commit/553421644bd3264e6fa1fdfd1fbebac1bc7eddb6))
* cut release for Windows and Linux only ([420cac9](https://github.com/RyanWez/sms-deliverer/commit/420cac9a2701c292776287a364df6dab24fd2e25))
* rotate signing keys and harden release workflow ([566ee0b](https://github.com/RyanWez/sms-deliverer/commit/566ee0b3244ed659e3af8ed7413eeca34166069d))
* **updater:** surface update-check results via toasts and native dialogs ([37a6e64](https://github.com/RyanWez/sms-deliverer/commit/37a6e64b9a624001c96e42f011bd1f4b09612b55))

## 1.0.1 (2026-08-27)

### Bug Fixes

* restore updater ACL permissions (`updater:default`, `process:default`, `dialog:default`) so update checks actually execute
* surface update-check results through toasts and native dialogs instead of silently-swallowed browser alerts
* wire automatic background update checks driven by the Updates settings (autoCheck / checkInterval)
* show real application version in the Settings footer instead of a hard-coded value

### Build

* reset all project versions to 1.0.1 and align Node / Tauri / Cargo manifests
* configure release-please to bump package.json, tauri.conf.json and Cargo.toml together via a shared manifest config
