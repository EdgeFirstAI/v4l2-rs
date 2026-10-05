# Testing

```bash
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

- **UAPI layout:** the size assertions in `uapi` are checked at compile time, so `cargo build` on x86_64 and aarch64 Linux is itself a test of the ABI.
- **Device tests:** device-backed tests run against the `vivid` virtual driver on hosted `ubuntu-24.04` runners, and on the EdgeFirst board fleet for real capture and M2M drivers (`ci:hardware`). They skip cleanly when no suitable device exists.
- **ABI check:** `strace -e trace=ioctl` shows the exact ioctl sequence and struct sizes. Use it when moving code between this crate and its consumers; the sequence must not change.
