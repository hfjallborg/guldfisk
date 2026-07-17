<img src="docs/guldfisk-banner.svg" width="320" alt="guldfisk" />

![Status](https://img.shields.io/badge/status-WIP-8b949e?style=flat-square)
![Rust](https://img.shields.io/badge/rust-stable-8b949e?style=flat-square&logo=rust&logoColor=white)
![Version](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fhfjallborg%2Fguldfisk%2Fmaster%2FCargo.toml&query=%24.package.version&label=version&color=7c6cf0&style=flat-square)

***"We have Redis at home"***

In-memory cache and message broker written in Rust, currently a work in progress. Heavily inspired by Redis, it builds
upon a key-value hashmap and uses a single execution thread to handle all commands. Clients can connect using either TCP or Unix sockets.

## Configuration

Configured via environment variables:

| Variable    | Default | Description                  |
|-------------|---------|------------------------------|
| `GULDFISK_PORT` | `1983`  | TCP port the server binds to |
