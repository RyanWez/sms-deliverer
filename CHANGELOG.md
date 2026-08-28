# Changelog

## [1.2.0](https://github.com/RyanWez/sms-deliverer/compare/v1.1.0...v1.2.0) (2026-08-28)


### Features

* auto-reconnect live ports, SIM retention cleanup, and truthful live status ([6930da1](https://github.com/RyanWez/sms-deliverer/commit/6930da166906947a7a6be4948e1acaf850ff7c06))

## [1.1.0](https://github.com/RyanWez/sms-deliverer/compare/v1.0.1...v1.1.0) (2026-08-27)


### Features

* add cascading entrance animations and scan skeleton to message table ([f8d785e](https://github.com/RyanWez/sms-deliverer/commit/f8d785e1de8cca5234a33f489d8573845dac99db))
* add dynamic button text to settings, improve select input UI, and truncate long text labels ([417c393](https://github.com/RyanWez/sms-deliverer/commit/417c393186e5550bdc48f21df1e8747244f60674))
* add SIM-based filtering/selection in Ports and enable log filtering by port in Logs ([c95d7f3](https://github.com/RyanWez/sms-deliverer/commit/c95d7f3ca27cb1bc553082f03be8c227210869c1))
* enable dark mode and set consistent background colors for the app window ([7b1ecc5](https://github.com/RyanWez/sms-deliverer/commit/7b1ecc57945a929d63ab93dc79a3fe652a1e13d4))
* implement background expiration and auto-deletion of old SMS messages with configurable retention periods ([439d4d9](https://github.com/RyanWez/sms-deliverer/commit/439d4d9ca533967dd300327278b8214081c2713f))
* implement DCS parsing to support 8-bit/UCS2 alphabets and add automatic GSM-7 decoding recovery ([9f564f3](https://github.com/RyanWez/sms-deliverer/commit/9f564f3d62568e0aaaa713b8fc07798ab9b737a5))
* implement system-wide log capture, storage, and UI visualization ([6e58f38](https://github.com/RyanWez/sms-deliverer/commit/6e58f38ecae52ba725b50164743d10ff0c1b6ee6))
* **logging:** add capped rotating release-build file logger ([4c95e94](https://github.com/RyanWez/sms-deliverer/commit/4c95e943c2c63c6db026fecf9a0b2ce80b674a54))
* **ports:** stable identity via /dev/serial/by-path with legacy migration ([0862109](https://github.com/RyanWez/sms-deliverer/commit/08621096dc3161ec98719fcda51ac995d6c3482b))
* **ui:** native confirm dialogs, updater download progress, and inbox export button ([36469b1](https://github.com/RyanWez/sms-deliverer/commit/36469b13f2561f96fb921d613057e078780da9ee))
* update port list animation timing and increase maximum log file size to 5MB ([88cebae](https://github.com/RyanWez/sms-deliverer/commit/88cebae01820c5f45ef2d182ad22f62bdc03f21c))


### Performance Improvements

* bound scan and USSD port concurrency at 16; feat(inbox): native CSV/JSON export ([ef33a1b](https://github.com/RyanWez/sms-deliverer/commit/ef33a1b3ab7ea356e4cb0a35d2d4bd5c1db833a6))
* implement message and port event batching with optimized waterfall transition animations ([f98b08d](https://github.com/RyanWez/sms-deliverer/commit/f98b08dc9e002961d458e07537f9d88b8fe8f4c8))
* optimize sidebar animation with GPU acceleration, layout containment, and grid state freezing to prevent visual jank ([c7ef4d2](https://github.com/RyanWez/sms-deliverer/commit/c7ef4d2d77bcd4a4eea59b55ac0c6e091ef678e8))

## 1.0.1 (2026-08-27)

### Bug Fixes

* restore updater ACL permissions (`updater:default`, `process:default`, `dialog:default`) so update checks actually execute
* surface update-check results through toasts and native dialogs instead of silently-swallowed browser alerts
* wire automatic background update checks driven by the Updates settings (autoCheck / checkInterval)
* show real application version in the Settings footer instead of a hard-coded value

### Build

* reset all project versions to 1.0.1 and align Node / Tauri / Cargo manifests
* configure release-please to bump package.json, tauri.conf.json and Cargo.toml together via a shared manifest config
