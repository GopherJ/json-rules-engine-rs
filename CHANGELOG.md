# Changelog

## 0.10.0 (2022-xx-yy)
## Added
- There is a new `async` feature which is disabled by default. By enabling this feature, `validate`, `trigger` and `run` methods will be async, and the lib will use `Arc` instead of `Rc`. `email` and `callback` enable `async` feature internally. 
## Changed
## Removed

## 0.9.4 (2021-08-06)
## Added
- Support adding custom events. (Check the tests/tests.rs file for an example.)
- Support `path` property of a condition.
## Changed
## Removed
