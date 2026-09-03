# Changelog

## [1.6.0](https://github.com/RyanWez/sms-deliverer/compare/v1.5.0...v1.6.0) (2026-09-03)


### Features

* add Telegram bot configuration for group forwarding ([243e070](https://github.com/RyanWez/sms-deliverer/commit/243e070bee5a9b4f7d3cb77ea3e2bd4be9f59fa1))
* forward live SMS and OTPs into the Telegram group ([88144a4](https://github.com/RyanWez/sms-deliverer/commit/88144a499259e10f7f8a02b9cbae45b51a4bd8e2))


### Bug Fixes

* prevent extraction of date or time components as OTP codes by validating surrounding numeric fields. ([fb479a4](https://github.com/RyanWez/sms-deliverer/commit/fb479a4af9cdb923c48bfa5fa5eb8e5e6295e612))
* retry a Telegram send the network never carried ([3345581](https://github.com/RyanWez/sms-deliverer/commit/334558147ec8cb7a224169ba3352d32c12f0abd6))
* say what to do when Telegram refuses a forward ([eec4353](https://github.com/RyanWez/sms-deliverer/commit/eec4353c0cb89d3de1bd2e31adbf6d4500b15f35))

## [1.5.0](https://github.com/RyanWez/sms-deliverer/compare/v1.4.0...v1.5.0) (2026-09-01)


### Features

* surface delete, export and reconnect outcomes to the operator ([#17](https://github.com/RyanWez/sms-deliverer/issues/17)) ([1c3c7a9](https://github.com/RyanWez/sms-deliverer/commit/1c3c7a9a6900738bf898930739f98e2c84b54591))


### Bug Fixes

* clear every busy flag on panic and report a crashed live worker ([#18](https://github.com/RyanWez/sms-deliverer/issues/18)) ([1263ca3](https://github.com/RyanWez/sms-deliverer/commit/1263ca30b5274ab39a294541646b362e3c05e79a))
* confirm the live SIM sweep against the card, and bound the toast column ([#20](https://github.com/RyanWez/sms-deliverer/issues/20)) ([4f87921](https://github.com/RyanWez/sms-deliverer/commit/4f87921dde4f6ea590cdc005e9f449e86682fa7f))
* give live-mode messages their real SIM slot index ([#12](https://github.com/RyanWez/sms-deliverer/issues/12)) ([492634e](https://github.com/RyanWez/sms-deliverer/commit/492634ea4471a790706ea95035b18b34fd3327aa))
* stop the inbox search box from discarding what the operator types ([#15](https://github.com/RyanWez/sms-deliverer/issues/15)) ([56fd2c8](https://github.com/RyanWez/sms-deliverer/commit/56fd2c88fa5a492b100c7ad9776cbce3ecf02ca9))
* tell a port that could not be probed apart from an empty SIM slot ([#19](https://github.com/RyanWez/sms-deliverer/issues/19)) ([3710a64](https://github.com/RyanWez/sms-deliverer/commit/3710a64ec3eb7b5d906251131db6c5e8aeb0edde))
* treat an absurd retention window as "keep everything" instead of panicking ([#13](https://github.com/RyanWez/sms-deliverer/issues/13)) ([b37edb7](https://github.com/RyanWez/sms-deliverer/commit/b37edb77684a25f2e45c27f3f7e23b7c5a986191))

## [1.4.0](https://github.com/RyanWez/sms-deliverer/compare/v1.3.1...v1.4.0) (2026-08-30)


### Features

* working theme switching, port auto-refresh, and reliability hardening ([#11](https://github.com/RyanWez/sms-deliverer/issues/11)) ([d5d53e5](https://github.com/RyanWez/sms-deliverer/commit/d5d53e52aba0211efa305275645351882f38b3d4))


### Bug Fixes

* tell a failed serial write apart from a silent modem ([41bbc71](https://github.com/RyanWez/sms-deliverer/commit/41bbc71a47cc5274fe96cbcc175d514a4e7c2868))

## [1.3.1](https://github.com/RyanWez/sms-deliverer/compare/v1.3.0...v1.3.1) (2026-08-29)


### Bug Fixes

* **ci:** pass the release body into latest.json so the update panel has notes ([0b82eb5](https://github.com/RyanWez/sms-deliverer/commit/0b82eb543c43cd4b7faf52192491d05ecbfafc6b))

## [1.3.0](https://github.com/RyanWez/sms-deliverer/compare/v1.2.0...v1.3.0) (2026-08-29)


### Features

* add expand/collapse toggle for long message text ([091c48d](https://github.com/RyanWez/sms-deliverer/commit/091c48d03e6f4d50f043791d694c15b6e6b30a28))
* **ports:** probe modems before work so empty slots cost ~1.6s not 24s ([e50d0b0](https://github.com/RyanWez/sms-deliverer/commit/e50d0b0efb99e4643231d7328dfe3d0d8cb3b550))
* **updates:** review release notes in-app, then download and restart separately ([82686e3](https://github.com/RyanWez/sms-deliverer/commit/82686e30ca6a68b85c45ca8c66ef54047a00cf8c))


### Bug Fixes

* decode GSM-7 payload from UDH septet boundary in deliver PDUs ([431dcaf](https://github.com/RyanWez/sms-deliverer/commit/431dcaf75e70dde5dd0827df41b1f0c27f32e496))
* **delete:** confirm a delete by re-reading the SIM, not by per-command replies ([712cbef](https://github.com/RyanWez/sms-deliverer/commit/712cbef06ed6936c97bd0e84796cfae3758cda1e))
* **delete:** confirm each AT+CMGD instead of assuming it worked ([7613a80](https://github.com/RyanWez/sms-deliverer/commit/7613a80e81e8eb5aecd45a7ee107714829e60496))
* **sim:** key SIM numbers on ICCID, not on the serial port ([31d3e5b](https://github.com/RyanWez/sms-deliverer/commit/31d3e5bf8f0360f9c807a528b2510b0ae19bb157))
* **sim:** read EF_MSISDN before USSD and clear stale USSD sessions ([060fb3e](https://github.com/RyanWez/sms-deliverer/commit/060fb3e9082b2524b885b2050890ee53fae325c6))

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
