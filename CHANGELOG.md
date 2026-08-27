# Changelog

## [1.1.0](https://github.com/RyanWez/sms-deliverer/compare/v1.0.3...v1.1.0) (2026-08-27)


### Features

* add configurable confirmation for deletions, optional message table columns, and auto-start functionality for live mode ([e4b71b0](https://github.com/RyanWez/sms-deliverer/commit/e4b71b0d0145f7fdae324be7f51bb18c999613c4))
* add scan and USSD busy states with UI loading indicators ([e0a2be0](https://github.com/RyanWez/sms-deliverer/commit/e0a2be0cdd397d3dd692cf10e8810e2f716f4ccc))
* add Tauri runtime detection to enable web browser fallback mode and lazy-load Tauri dependencies. ([85ad849](https://github.com/RyanWez/sms-deliverer/commit/85ad849b9814f3b5de93f25f1af94072f1c0e5ed))

## 1.0.0 (2026-08-26)


### Features

* add port status utilities and implement PortDetail component for side panel information display ([f8d338a](https://github.com/RyanWez/sms-deliverer/commit/f8d338a9c2b58da71edf02003f5b8304c80c230a))
* implement auto-updater functionality with signing keys, workflow automation, and UI integration ([f1c784c](https://github.com/RyanWez/sms-deliverer/commit/f1c784c92aef1b03de8f6c16b8c54034ba99cf92))
* implement message details view and refactor table UI with shared icon components ([452454f](https://github.com/RyanWez/sms-deliverer/commit/452454f5c8b2c47116389d0a353e58d54786bf85))
* replace SVG port icon with remote image asset in Ports page ([61428a6](https://github.com/RyanWez/sms-deliverer/commit/61428a6c8488dcf91876fbcec5a67e7b65d8f71c))
* update message grid layout, introduce fixed-page pagination, and improve port card interactivity ([39f2064](https://github.com/RyanWez/sms-deliverer/commit/39f20641036d46f31ffb0c96bd5917b6d95c9722))


### Bug Fixes

* rotate signing keys and harden release workflow ([566ee0b](https://github.com/RyanWez/sms-deliverer/commit/566ee0b3244ed659e3af8ed7413eeca34166069d))
