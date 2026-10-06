# Architecture

`edgefirst-v4l2` is the single Rust copy of the Video4Linux2 kernel interface used across EdgeFirst. It sits under two consumers:

- **`edgefirst-camera` (V4L2 capture backend):** opens capture nodes, negotiates formats, imports or exports buffers, and polls for frames.
- **`edgefirst-codec` (memory-to-memory codecs):** drives a decoder or encoder through its OUTPUT and CAPTURE queues.

Before this crate existed, the codec carried a private copy of the UAPI structs and queue helpers. Moving them here gives both consumers one tested definition of the ABI.

## Layers

1. **`uapi`:** plain data. The `#[repr(C)]` structs, constants and FourCCs from `linux/videodev2.h`. Every struct has a `const` size assertion for 64-bit Linux, because the ioctl request number is derived from `sizeof`, and a wrong layout would otherwise show up only as `ENOTTY` or a short copy at run time. The `m` union in `v4l2_buffer` and `v4l2_plane` is modelled as one 8-byte field, which matches the kernel on 64-bit targets.
2. **`ioctl`:** one `nix` binding per ioctl. This is the only place request numbers are built.
3. **Safe helpers:** `device`, `queue`, `controls`, `events` and `m2m`, written in safe Rust over the two layers above. Unsafe code is limited to the ioctl calls themselves and to `mmap`.

## Rules

- **No EdgeFirst types.** Consumers translate buffers into tensors, images or anything else, so the crate can stay small and stable.
- **Report what the driver did.** Helpers return what the driver set (pitch, sizes, buffer counts) rather than what was asked for, because drivers adjust requests. vvcam, for example, ignores a requested `bytesperline`.
- **No side effects on nodes other processes own.** Enumeration queries capabilities only; it does not stream or change formats.
- **Linux only.** Everything is gated on `target_os = "linux"`; elsewhere the crate is empty.
