// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! Errors returned by the buffer queue and memory-to-memory helpers.

use std::fmt;

use nix::errno::Errno;

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// What went wrong, independent of the operation that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The device is gone (`ENODEV`, `ENXIO`, or `POLLHUP`), for example a
    /// USB camera that was unplugged. The file descriptor cannot recover.
    Disconnected,
    /// The driver does not implement the request (`ENOTTY`), does not
    /// support it for the current input or output (`ENODATA`, for example a
    /// selection target), or does not support the memory type or buffer type
    /// that was asked for.
    Unsupported,
    /// The queue is owned by another file handle or is streaming (`EBUSY`).
    Busy,
    /// The driver rejected an argument (`EINVAL`), or a value outside what
    /// the control accepts (`ERANGE`).
    InvalidArgument,
    /// The operation is not permitted: reading a write-only control,
    /// setting a read-only one, or no access to the node (`EACCES`,
    /// `EPERM`).
    PermissionDenied,
    /// The call is not valid in the queue's current state: no buffers are
    /// allocated, a buffer is already queued, or the stream is not running.
    InvalidState,
    /// A memory-to-memory queue returned its last buffer and is drained
    /// (`EPIPE` from `VIDIOC_DQBUF`).
    EndOfStream,
    /// Any other operating-system error; see [`Error::errno`].
    Io,
}

/// An error from a V4L2 operation: the [`ErrorKind`], the operation that
/// failed and, when the kernel reported one, the `errno`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    op: &'static str,
    errno: Option<Errno>,
}

impl Error {
    /// Classifies `errno` returned by the ioctl or system call `op`.
    pub(crate) fn from_errno(op: &'static str, errno: Errno) -> Self {
        let kind = match errno {
            Errno::ENODEV | Errno::ENXIO => ErrorKind::Disconnected,
            Errno::ENOTTY | Errno::ENODATA => ErrorKind::Unsupported,
            Errno::EBUSY => ErrorKind::Busy,
            Errno::EINVAL | Errno::ERANGE => ErrorKind::InvalidArgument,
            Errno::EACCES | Errno::EPERM => ErrorKind::PermissionDenied,
            Errno::EPIPE => ErrorKind::EndOfStream,
            _ => ErrorKind::Io,
        };
        Self {
            kind,
            op,
            errno: Some(errno),
        }
    }

    /// An error detected by the crate before reaching the kernel.
    pub(crate) fn new(kind: ErrorKind, op: &'static str) -> Self {
        Self {
            kind,
            op,
            errno: None,
        }
    }

    /// The same failure reported as `kind`, keeping the operation and errno.
    pub(crate) fn with_kind(self, kind: ErrorKind) -> Self {
        Self { kind, ..self }
    }

    /// The category of the failure.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The operation that failed, such as `"VIDIOC_QBUF"` or `"poll"`.
    pub fn op(&self) -> &'static str {
        self.op
    }

    /// The `errno` reported by the kernel, if the failure came from a system
    /// call.
    pub fn errno(&self) -> Option<Errno> {
        self.errno
    }

    /// Whether the device has gone away.
    pub fn is_disconnected(&self) -> bool {
        self.kind == ErrorKind::Disconnected
    }

    /// Whether the driver does not support the request.
    pub fn is_unsupported(&self) -> bool {
        self.kind == ErrorKind::Unsupported
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.kind {
            ErrorKind::Disconnected => "device disconnected",
            ErrorKind::Unsupported => "not supported by the driver",
            ErrorKind::Busy => "device busy",
            ErrorKind::InvalidArgument => "invalid argument",
            ErrorKind::PermissionDenied => "permission denied",
            ErrorKind::InvalidState => "invalid in the current queue state",
            ErrorKind::EndOfStream => "end of stream",
            ErrorKind::Io => "I/O error",
        };
        match self.errno {
            Some(e) => write!(f, "{}: {what} ({e})", self.op),
            None => write!(f, "{}: {what}", self.op),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

/// Runs `f` until it returns something other than `EINTR`, and classifies a
/// failure as an [`Error`] for `op`.
pub(crate) fn retry<T>(op: &'static str, mut f: impl FnMut() -> nix::Result<T>) -> Result<T> {
    loop {
        match f() {
            Err(Errno::EINTR) => continue,
            r => return r.map_err(|e| Error::from_errno(op, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errno_classification() {
        let k = |e| Error::from_errno("op", e).kind();
        assert_eq!(k(Errno::ENODEV), ErrorKind::Disconnected);
        assert_eq!(k(Errno::ENXIO), ErrorKind::Disconnected);
        assert_eq!(k(Errno::ENOTTY), ErrorKind::Unsupported);
        assert_eq!(k(Errno::ENODATA), ErrorKind::Unsupported);
        assert_eq!(k(Errno::EBUSY), ErrorKind::Busy);
        assert_eq!(k(Errno::EINVAL), ErrorKind::InvalidArgument);
        assert_eq!(k(Errno::ERANGE), ErrorKind::InvalidArgument);
        assert_eq!(k(Errno::EACCES), ErrorKind::PermissionDenied);
        assert_eq!(k(Errno::EPIPE), ErrorKind::EndOfStream);
        assert_eq!(k(Errno::EIO), ErrorKind::Io);
    }

    #[test]
    fn retry_repeats_eintr_only() {
        let mut calls = 0;
        let r = retry("op", || {
            calls += 1;
            if calls < 3 {
                Err(Errno::EINTR)
            } else {
                Ok(calls)
            }
        });
        assert_eq!(r, Ok(3));

        let e = retry::<()>("VIDIOC_QBUF", || Err(Errno::EAGAIN)).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::Io);
        assert_eq!(e.errno(), Some(Errno::EAGAIN));
        assert_eq!(e.to_string(), "VIDIOC_QBUF: I/O error (EAGAIN: Try again)");
    }
}
