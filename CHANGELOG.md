# Changelog

## 1.0.1 (2026-08-27)

### Bug Fixes

* restore updater ACL permissions (`updater:default`, `process:default`, `dialog:default`) so update checks actually execute
* surface update-check results through toasts and native dialogs instead of silently-swallowed browser alerts
* wire automatic background update checks driven by the Updates settings (autoCheck / checkInterval)
* show real application version in the Settings footer instead of a hard-coded value

### Build

* reset all project versions to 1.0.1 and align Node / Tauri / Cargo manifests
* configure release-please to bump package.json, tauri.conf.json and Cargo.toml together via a shared manifest config

