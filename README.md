# edgefirst-v4l2

Video4Linux2 for Rust: the kernel UAPI definitions, ioctl wrappers, buffer queues, controls, events and device enumeration that the EdgeFirst stack builds on.

- The **EdgeFirst Camera SDK** ([`edgefirst-camera`](https://github.com/EdgeFirstAI/camera)) uses it for capture.
- The **EdgeFirst HAL codecs** ([`edgefirst-codec`](https://github.com/EdgeFirstAI/hal)) use it for memory-to-memory JPEG and H.264.

> **Status:** in development. `uapi` and `ioctl` are in place: the UAPI layer moved out of `edgefirst-codec`, extended for capture, controls, selection, events and codec commands. The safe helpers follow.

## Design

- **One copy of the kernel ABI.** Every `#[repr(C)]` struct mirrors `linux/videodev2.h` and carries a compile-time size check for 64-bit Linux, so a layout mistake fails the build instead of surfacing as `ENOTTY` at run time.
- **No EdgeFirst dependencies.** The crate knows nothing about tensors or images; consumers map buffers to their own types.
- **Linux only, portable to depend on.** On other platforms the crate compiles to nothing, so portable crates can depend on it unconditionally.
- **Driver quirks are documented, not hidden.** The capture and M2M helpers handle what real drivers do: an `EINTR`/`EAGAIN` retry, a bounded event drain, and single- and multi-planar queues.

## Modules

| Module | Contents |
|---|---|
| `uapi` (available) | `#[repr(C)]` structs, constants and FourCCs, with size and offset checks |
| `ioctl` (available) | `nix` bindings for every ioctl the crate uses |
| `device` | Enumeration of `/dev/video*`, capabilities (honouring `V4L2_CAP_DEVICE_CAPS`), single- vs multi-planar detection |
| `queue` | Indexed buffer queues over MMAP, DMABUF and USERPTR; RAII mmap; `poll` with timeout |
| `controls` | Typed control query, get and set with ranges |
| `events` | Bounded `VIDIOC_DQEVENT` drain |
| `m2m` | Dual-queue helper for memory-to-memory codecs |

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
