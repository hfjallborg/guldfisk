# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/hfjallborg/guldfisk/compare/v0.3.0...v0.4.0) - 2026-07-22

### Added

- add array support for cache ([#4](https://github.com/hfjallborg/guldfisk/pull/4))

## [0.3.0](https://github.com/hfjallborg/guldfisk/compare/v0.2.0...v0.3.0) - 2026-07-20

### Added

- [**breaking**] change formatting for strings

### Other

- sync README with cargo.toml version

## [0.2.0](https://github.com/hfjallborg/guldfisk/compare/v0.1.0...v0.2.0) - 2026-07-17

### Added

- add active expiration of keys

### Other

- set git-only mode for release-plz

## [0.1.0](https://github.com/hfjallborg/guldfisk/releases/tag/v0.1.0) - 2026-07-07

### Added

- add expiration operation (set ttl for key-value)
- parse and execute commands from tcp/unix connections
- add basic parsing from command to operation
- add executor command protocol with oneshot reply channels
- add unix socket listener
- scaffold tcp listener with thread spawning

### Fixed

- add missing steps in release-plz action

### Other

- add release-plz action
- add github action for Rust
- [**breaking**] rename project to guldfisk
- make README prettier (with rainbows)
- add test for parsing error recovery
- add basic benchmarks for set operations
- add double set test
- extract lib.rs and flatten cache module
