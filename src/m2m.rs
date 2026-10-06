// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! Memory-to-memory devices (codecs, scalers): an output queue the
//! application fills and a capture queue the device fills, on one file
//! descriptor.

use std::os::fd::AsFd;
use std::time::Duration;

use nix::poll::PollFlags;

use crate::error::Result;
use crate::queue::{poll_device, BufType, Queue};

/// What [`M2m::wait`] found ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ready {
    /// An output buffer can be dequeued (the device consumed it).
    pub output: bool,
    /// A capture buffer can be dequeued (the device produced it).
    pub capture: bool,
    /// An event such as `V4L2_EVENT_SOURCE_CHANGE` is pending.
    pub event: bool,
}

impl Ready {
    /// Whether nothing was ready (the wait timed out).
    pub fn is_empty(&self) -> bool {
        !(self.output || self.capture || self.event)
    }
}

/// The output and capture queues of a memory-to-memory device.
///
/// The [`Queue`] ownership rules apply to each queue. Formats are set on
/// each queue's buffer type with `VIDIOC_S_FMT` before allocation, as for
/// any queue.
#[derive(Debug)]
pub struct M2m {
    output: Queue,
    capture: Queue,
}

impl M2m {
    /// Creates both queues on the open device `device`, using the
    /// multi-planar buffer types when `multiplanar` is set.
    pub fn new(device: impl AsFd, multiplanar: bool) -> Result<Self> {
        let (out, cap) = if multiplanar {
            (BufType::VideoOutputMplane, BufType::VideoCaptureMplane)
        } else {
            (BufType::VideoOutput, BufType::VideoCapture)
        };
        let fd = device.as_fd();
        Ok(Self {
            output: Queue::new(fd, out)?,
            capture: Queue::new(fd, cap)?,
        })
    }

    /// The output queue (application to device).
    pub fn output(&self) -> &Queue {
        &self.output
    }

    /// The output queue, for allocation.
    pub fn output_mut(&mut self) -> &mut Queue {
        &mut self.output
    }

    /// The capture queue (device to application).
    pub fn capture(&self) -> &Queue {
        &self.capture
    }

    /// The capture queue, for allocation.
    pub fn capture_mut(&mut self) -> &mut Queue {
        &mut self.capture
    }

    /// Splits into the output and capture queues.
    pub fn into_queues(self) -> (Queue, Queue) {
        (self.output, self.capture)
    }

    /// Starts streaming on both queues, output first.
    pub fn stream_on(&self) -> Result<()> {
        self.output.stream_on()?;
        self.capture.stream_on()
    }

    /// Stops streaming on both queues, capture first, and reclaims every
    /// queued buffer. Both are attempted; the first failure is returned.
    pub fn stream_off(&self) -> Result<()> {
        let capture = self.capture.stream_off();
        let output = self.output.stream_off();
        capture.and(output)
    }

    /// Waits up to `timeout` (forever when `None`) until an output buffer is
    /// consumed, a capture buffer is produced, or an event is pending, and
    /// reports which. All three come from one `poll` of the shared
    /// descriptor. An empty [`Ready`] means the wait timed out.
    pub fn wait(&self, timeout: Option<Duration>) -> Result<Ready> {
        let events = PollFlags::POLLIN | PollFlags::POLLOUT | PollFlags::POLLPRI;
        let revents = poll_device(self.capture.fd(), events, timeout)?;
        Ok(revents.map_or(Ready::default(), |r| Ready {
            output: r.contains(PollFlags::POLLOUT),
            capture: r.contains(PollFlags::POLLIN),
            event: r.contains(PollFlags::POLLPRI),
        }))
    }
}
