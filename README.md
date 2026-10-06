# edgefirst-v4l2

Video4Linux2 for Rust: the kernel UAPI definitions, ioctl wrappers, buffer queues, controls, events and device enumeration that the EdgeFirst stack builds on.

- The **EdgeFirst Camera SDK** ([`edgefirst-camera`](https://github.com/EdgeFirstAI/camera)) uses it for capture.
- The **EdgeFirst HAL codecs** ([`edgefirst-codec`](https://github.com/EdgeFirstAI/hal)) use it for memory-to-memory JPEG and H.264.

> **Status:** in development. Every module is in place; the API may still change before 1.0.

## Design

- **One copy of the kernel ABI.** Every `#[repr(C)]` struct mirrors `linux/videodev2.h` and carries a compile-time size check for 64-bit Linux, so a layout mistake fails the build instead of surfacing as `ENOTTY` at run time.
- **No EdgeFirst dependencies.** The crate knows nothing about tensors or images; consumers map buffers to their own types.
- **Linux only, portable to depend on.** On other platforms the crate compiles to nothing, so portable crates can depend on it unconditionally.
- **Driver quirks are documented, not hidden.** The capture and M2M helpers handle what real drivers do: an `EINTR`/`EAGAIN` retry, a bounded event drain, and single- and multi-planar queues.

## Modules

| Module | Contents |
|---|---|
| `uapi` | `#[repr(C)]` structs, constants and FourCCs, with size and offset checks |
| `ioctl` | `nix` bindings for every ioctl the crate uses |
| `device` | Numeric-order enumeration of `/dev/video*`; capabilities honouring `V4L2_CAP_DEVICE_CAPS`, with the capture and output buffer types; formats, frame sizes and intervals; `G/S/TRY_FMT`; frame rate (`G/S_PARM`); crop and compose (`G/S_SELECTION`) |
| `queue` | Indexed buffer queues over MMAP, DMABUF and USERPTR, single- and multi-planar; RAII mmap; `EXPBUF`; `poll` with timeout; buffer flags and timestamp clock |
| `controls` | Control enumeration with ranges and menus; typed get and set, including 64-bit, string and array controls; returns the value the driver applied |
| `events` | Subscription; dequeue and a bounded drain that never block; source-change and control payloads |
| `m2m` | Output and capture queues of a memory-to-memory device, with one `poll` for both and for events |

## Examples

```sh
cargo run --example v4l2-devices                                  # every node: capabilities, formats, sizes, rates
cargo run --example v4l2-controls -- /dev/video0                  # controls, ranges, menus and current values
cargo run --example v4l2-controls -- /dev/video0 brightness=128   # set, then show what the driver applied
```

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
