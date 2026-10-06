# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-10-06

### Added
- `uapi`: the V4L2 kernel ABI from `linux/videodev2.h` and `linux/v4l2-controls.h`. It covers capability, format, frame size and interval enumeration, stream parameters, buffers (MMAP, DMABUF, USERPTR, `EXPBUF`, `CREATE_BUFS`), controls and extended controls, menus, events, selection, and encoder and decoder commands. Every struct has compile-time size and union-offset checks generated from the kernel headers on x86_64 and aarch64; constant values are generated from the same headers. Unions are modelled as fixed-size fields with safe typed accessors, so no struct needs `repr(packed)`. The definitions `edgefirst-codec` already used keep their names and types (EDGEAI-1513).
- `ioctl`: `nix` bindings for the 35 V4L2 ioctls the crate uses, with a test that checks every request number against the kernel header (EDGEAI-1513).
- Repository setup: crate skeleton, CI tiers, release chain and project documentation (EDGEAI-1513).

[Unreleased]: https://github.com/EdgeFirstAI/v4l2-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/EdgeFirstAI/v4l2-rs/releases/tag/v0.1.0
