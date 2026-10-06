# Testing

```bash
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

- **UAPI layout:** the size assertions in `uapi` are checked at compile time, so `cargo build` on x86_64 and aarch64 Linux is itself a test of the ABI.
- **Queue tests (`tests/vivid.rs`):** these run against the kernel's virtual drivers: `vivid` for capture (single- and multi-planar) and `vim2m` for memory-to-memory. They cover MMAP, DMABUF from the system DMA heap, USERPTR, `EXPBUF` with orphaned buffers, `CREATE_BUFS`, restart after `STREAMOFF`, state errors, and an M2M round trip with timestamp copy. Load the drivers once:

  ```bash
  sudo modprobe vivid n_devs=2 node_types=0x1,0x1 multiplanar=1,2
  sudo modprobe vim2m
  cargo test --locked --test vivid
  ```

  The user needs read-write access to `/dev/video*` and `/dev/dma_heap/system` (usually the `video` group). Each test takes an exclusive `flock` on its node, so tests sharing a node run one at a time even under `cargo nextest`. Without the drivers the tests print `SKIPPED` and pass. Setting `EDGEFIRST_V4L2_REQUIRE_VIVID=1` makes a missing driver a failure; the CI `vivid` job sets it.
- **CI:** the `vivid` job in `ci.yml` installs `linux-modules-extra` for the hosted `ubuntu-24.04` kernel, loads both drivers and runs the queue tests on every pull request. If the archive lacks modules for the runner's kernel, it warns and skips. Real capture and M2M drivers run on the EdgeFirst board fleet (`ci:hardware`).
- **ABI check:** `strace -e trace=ioctl` shows the exact ioctl sequence and struct sizes. Use it when moving code between this crate and its consumers; the sequence must not change.
