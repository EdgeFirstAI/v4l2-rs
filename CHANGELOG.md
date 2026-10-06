# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-10-06

### Added
- `events::wait`: waits for a pending event alone, ignoring buffer readiness, for example a source change while buffers complete (EDGEAI-1516).

### Fixed
- The `events` documentation said `mxc-jpeg` blocks in `VIDIOC_DQEVENT` even on a non-blocking descriptor. Measured on i.MX 95, it blocks only on a blocking one (EDGEAI-1516).
- The `events` and `queue` documentation promised that dequeueing never blocks however the node was opened. On a blocking descriptor that holds only while nothing else dequeues from the same open file concurrently; the documentation now says so (EDGEAI-1516).

## [0.2.0] - 2026-10-06

### Added
- `device`: `enumerate` lists `/dev/video*` in numeric order with each node's capabilities. `Device` opens a node and provides capabilities (honouring `V4L2_CAP_DEVICE_CAPS`, with the capture and output buffer types), formats, frame sizes and intervals, `G/S/TRY_FMT`, frame rate through `G/S_PARM`, and `G/S_SELECTION`. Setters return what the driver applied (EDGEAI-1515).
- `controls`: enumeration with `QUERY_EXT_CTRL` (falling back to `QUERYCTRL`) and `QUERYMENU`, and typed get and set through `G/S_EXT_CTRLS`, including 64-bit, string and array or compound controls. `set` returns the value the driver applied (EDGEAI-1515).
- `events`: subscribe and unsubscribe, plus a dequeue and a bounded drain that never block, with source-change and control payloads (EDGEAI-1515).
- `uapi`: `v4l2_event_ctrl` with layout checks, `v4l2_event::ctrl`, and the `V4L2_EVENT_CTRL_CH_*` constants (EDGEAI-1515).
- `ErrorKind::PermissionDenied` for `EACCES` and `EPERM` (EDGEAI-1515).
- Examples `v4l2-devices` and `v4l2-controls`, and `scripts/ioctl-trace.sh` for the strace ABI check (EDGEAI-1515).
- `queue`: indexed buffer queue over MMAP, DMABUF and USERPTR, single- and multi-planar. It covers `REQBUFS`, optional `CREATE_BUFS`, `QUERYBUF`, RAII `mmap`, `EXPBUF`, `QBUF`/`DQBUF`, and `STREAMON`/`STREAMOFF`, with per-index queued state kept in step with the kernel across threads, a `poll` wait with timeout, and a dequeue that never blocks. Dequeued buffers report the error and last flags, the timestamp clock and source, and per-plane payload. Buffer capabilities include orphaned-buffer support. `EINTR` is retried everywhere, and `ENODEV` maps to `Disconnected` (EDGEAI-1514).
- `m2m`: the output and capture queues of a memory-to-memory device, with one wait for both queues and pending events (EDGEAI-1514).
- `Error` and `ErrorKind` for the safe helpers (EDGEAI-1514).
- `uapi`: `set_userptr` on `v4l2_buffer` and `v4l2_plane` (EDGEAI-1514).
- CI: a `vivid` job runs the queue tests against `vivid` and `vim2m` on hosted runners (EDGEAI-1514).

### Changed
- `ErrorKind::Unsupported` also covers `ENODATA`, and `ErrorKind::InvalidArgument` also covers `ERANGE` (EDGEAI-1515).

## [0.1.0] - 2026-10-06

### Added
- `uapi`: the V4L2 kernel ABI from `linux/videodev2.h` and `linux/v4l2-controls.h`. It covers capability, format, frame size and interval enumeration, stream parameters, buffers (MMAP, DMABUF, USERPTR, `EXPBUF`, `CREATE_BUFS`), controls and extended controls, menus, events, selection, and encoder and decoder commands. Every struct has compile-time size and union-offset checks generated from the kernel headers on x86_64 and aarch64; constant values are generated from the same headers. Unions are modelled as fixed-size fields with safe typed accessors, so no struct needs `repr(packed)`. The definitions `edgefirst-codec` already used keep their names and types (EDGEAI-1513).
- `ioctl`: `nix` bindings for the 35 V4L2 ioctls the crate uses, with a test that checks every request number against the kernel header (EDGEAI-1513).
- Repository setup: crate skeleton, CI tiers, release chain and project documentation (EDGEAI-1513).

[Unreleased]: https://github.com/EdgeFirstAI/v4l2-rs/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/EdgeFirstAI/v4l2-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/EdgeFirstAI/v4l2-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/EdgeFirstAI/v4l2-rs/releases/tag/v0.1.0
