// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! # edgefirst-v4l2
//!
//! Video4Linux2 for Rust: kernel UAPI definitions, ioctl wrappers, buffer
//! queues, controls, events and device enumeration, shared by the EdgeFirst
//! camera SDK (capture) and the EdgeFirst HAL codecs (memory-to-memory).
//!
//! The crate has no dependency on the EdgeFirst tensor types; consumers map
//! V4L2 buffers to their own types.
//!
//! On platforms other than Linux the crate compiles to nothing, so it can be
//! an unconditional dependency of portable crates.

#[cfg(target_os = "linux")]
mod error;
#[cfg(target_os = "linux")]
pub mod ioctl;
#[cfg(target_os = "linux")]
pub mod m2m;
#[cfg(target_os = "linux")]
pub mod queue;
#[cfg(target_os = "linux")]
pub mod uapi;

#[cfg(target_os = "linux")]
pub use error::{Error, ErrorKind, Result};
