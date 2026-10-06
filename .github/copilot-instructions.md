# AI assistant guidelines — edgefirst-v4l2

Canonical process, CI tiers, runner policy and release chain:

https://github.com/EdgeFirstAI/.github/blob/main/.github/copilot-instructions.md

Keep this file for project-specific notes only. Do not duplicate branch, commit, PR or release rules here.

## Project-specific

### Layout

- Single crate at the repository root: `src/` (planned modules `uapi`, `ioctl`, `device`, `queue`, `controls`, `events`, `m2m`), `tests/` for device-backed integration tests, `examples/`.
- Consumers: `edgefirst-camera` (EdgeFirstAI/camera, V4L2 capture) and `edgefirst-codec` (EdgeFirstAI/hal, M2M codecs). API changes must be checked against both.

### Tests

- Unit: `cargo test --locked`
- Device tests skip cleanly without a suitable node. CI runs them on `vivid` (hosted `ubuntu-24.04`) and on the board fleet via `ci:hardware`.
- When moving code between this crate and a consumer, confirm with `strace -e trace=ioctl` that the ioctl sequence and struct sizes are unchanged.

### Platform notes

- Linux only. Everything is gated on `target_os = "linux"`; the crate is empty elsewhere so portable crates can depend on it.
- Every `#[repr(C)]` struct mirrors `linux/videodev2.h` and carries a compile-time `size_of` assertion for 64-bit Linux.
- No EdgeFirst dependencies (no tensor or image types).
- Drivers adjust requests (pitch, sizes, counts); helpers return what the driver set.
