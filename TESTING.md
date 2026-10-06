# Testing

```bash
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

- **UAPI layout:** the size assertions in `uapi` are checked at compile time, so `cargo build` on x86_64 and aarch64 Linux is itself a test of the ABI.
- **Device tests (`tests/vivid.rs`):** these run against the kernel's virtual drivers: `vivid` for capture (single- and multi-planar) and `vim2m` for memory-to-memory.
  - **Queues:** MMAP, DMABUF from the system DMA heap and USERPTR; `EXPBUF` with orphaned buffers; `CREATE_BUFS`; restart after `STREAMOFF`; switching memory types; concurrent stream control; state errors; an M2M round trip with timestamp copy.
  - **Devices:** numeric enumeration, capabilities and buffer types, formats, frame sizes and intervals, `TRY_FMT`/`S_FMT`, frame rate, and selection where the input supports it.
  - **Controls and events:** vivid's test controls cover every type: integer with clamping, 64-bit, string, menu with gaps, integer menu, `u8` array, write-only button and read-only integer. Control events cover `SEND_INITIAL`, a drain after changes from another handle, and unsubscribing. Load the drivers once:

  ```bash
  sudo modprobe vivid n_devs=2 node_types=0x1,0x1 multiplanar=1,2
  sudo modprobe vim2m
  cargo test --locked --test vivid
  ```

  The user needs read-write access to `/dev/video*` and `/dev/dma_heap/system` (usually the `video` group). Each test takes an exclusive `flock` on its node, so tests sharing a node run one at a time even under `cargo nextest`. Without the drivers the tests print `SKIPPED` and pass. Setting `EDGEFIRST_V4L2_REQUIRE_VIVID=1` makes a missing driver a failure; the CI `vivid` job sets it.
- **CI:** the `vivid` job in `ci.yml` installs `linux-modules-extra` for the hosted `ubuntu-24.04` kernel, loads both drivers and runs the queue tests on every pull request. If the archive lacks modules for the runner's kernel, it warns and skips. Real capture and M2M drivers run on the EdgeFirst board fleet (`ci:hardware`).
- **ABI check (`scripts/ioctl-trace.sh`):** when moving V4L2 code between this crate and its consumers, the ioctl sequence must not change. Record the same workload before and after, then diff:

  ```bash
  scripts/ioctl-trace.sh record before.trace -- ./old-binary --args
  scripts/ioctl-trace.sh record after.trace -- ./new-binary --args
  scripts/ioctl-trace.sh diff before.trace after.trace
  ```

  `record` runs the command under `strace -ff -e trace=ioctl`, one file per thread so that concurrent calls are never split across lines, joins the threads in thread-ID order, and keeps only the ioctls. It replaces descriptor numbers, pointers (64-bit addresses, printed with nine or more hex digits), buffer timestamps and frame sequence numbers with placeholders, so two runs of the same code compare equal; 32-bit values such as unknown control IDs are kept. A request whose struct strace does not decode, such as `VIDIOC_QUERYMENU`, is compared by request only, because its argument prints as a pointer. A multi-threaded workload compares equal only when its threads issue the same calls in the same order. strace decodes V4L2 requests and their structs by name, so a struct of the wrong size shows up as an undecoded `_IOC(...)` request, and `record` reports how many there are (it should be zero). `diff` exits 0 when the recordings match. On a board, copy the script along with the binaries; it needs only `bash`, `sed` and `strace`.
- **Examples:** `cargo run --example v4l2-devices` and `cargo run --example v4l2-controls -- /dev/videoN` make a quick manual check of a new board or driver.
- **Other platforms:** the crate compiles to nothing outside Linux. `cargo clippy --target x86_64-pc-windows-gnu --all-targets` (and the same for a macOS target) checks that it still does.
