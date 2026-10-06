// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! V4L2 events: subscription and a dequeue that never blocks.
//!
//! Some drivers block in `VIDIOC_DQEVENT` when no event is pending, even on
//! a non-blocking descriptor (the i.MX `mxc-jpeg` decoder does). Every
//! dequeue here first checks `POLLPRI` with a zero timeout, and [`drain`]
//! stops at the first event that reports nothing else pending, so neither
//! can hang.

use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::time::Duration;

use nix::errno::Errno;
use nix::poll::PollFlags;

use crate::error::{retry, Result};
use crate::ioctl;
use crate::queue::poll_device;
#[allow(clippy::wildcard_imports)]
use crate::uapi::*;

/// Payload of a control event ([`V4L2_EVENT_CTRL`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlEvent {
    /// What changed: `V4L2_EVENT_CTRL_CH_*` bits.
    pub changes: u32,
    /// The control's type (`V4L2_CTRL_TYPE_*`).
    pub control_type: u32,
    /// The control's value (64-bit controls use the full width).
    pub value: i64,
    /// The control's `V4L2_CTRL_FLAG_*` bits.
    pub flags: u32,
    /// Smallest value.
    pub minimum: i32,
    /// Largest value.
    pub maximum: i32,
    /// Value increment.
    pub step: i32,
    /// Default value.
    pub default_value: i32,
}

/// A dequeued event.
#[derive(Debug, Clone, Copy)]
pub struct Event {
    raw: v4l2_event,
}

impl Event {
    /// The event type (`V4L2_EVENT_*`).
    pub fn kind(&self) -> u32 {
        self.raw.type_
    }

    /// The ID the event relates to, such as the control ID of a control
    /// event or the pad of a source-change event.
    pub fn id(&self) -> u32 {
        self.raw.id
    }

    /// The event's sequence number.
    pub fn sequence(&self) -> u32 {
        self.raw.sequence
    }

    /// Events still pending after this one.
    pub fn pending(&self) -> u32 {
        self.raw.pending
    }

    /// When the event was raised, in `CLOCK_MONOTONIC`.
    pub fn timestamp(&self) -> Duration {
        let t = self.raw.timestamp;
        Duration::new(
            u64::try_from(t.tv_sec).unwrap_or(0),
            u32::try_from(t.tv_nsec).unwrap_or(0),
        )
    }

    /// The `V4L2_EVENT_SRC_CH_*` bits of a source-change event.
    pub fn source_change(&self) -> Option<u32> {
        (self.raw.type_ == V4L2_EVENT_SOURCE_CHANGE).then(|| self.raw.src_change())
    }

    /// The payload of a control event.
    pub fn control(&self) -> Option<ControlEvent> {
        (self.raw.type_ == V4L2_EVENT_CTRL).then(|| {
            let c = self.raw.ctrl();
            ControlEvent {
                changes: c.changes,
                control_type: c.type_,
                value: if c.type_ == V4L2_CTRL_TYPE_INTEGER64 {
                    c.value64()
                } else {
                    i64::from(c.value())
                },
                flags: c.flags,
                minimum: c.minimum,
                maximum: c.maximum,
                step: c.step,
                default_value: c.default_value,
            }
        })
    }

    /// The kernel's event structure, for event types without an accessor.
    pub fn raw(&self) -> &v4l2_event {
        &self.raw
    }
}

/// Subscribes to events of `kind` (`V4L2_EVENT_*`) for `id` (a control ID
/// for control events, otherwise usually 0), with `V4L2_EVENT_SUB_FL_*`
/// `flags`. `V4L2_EVENT_SUB_FL_SEND_INITIAL` queues the current state of a
/// control at once.
pub fn subscribe(dev: impl AsFd, kind: u32, id: u32, flags: u32) -> Result<()> {
    let sub = v4l2_event_subscription {
        type_: kind,
        id,
        flags,
        ..Default::default()
    };
    // SAFETY: valid fd; `sub` outlives the call.
    retry("VIDIOC_SUBSCRIBE_EVENT", || unsafe {
        ioctl::vidioc_subscribe_event(dev.as_fd().as_raw_fd(), &sub)
    })
    .map(|_| ())
}

/// Ends a subscription made with [`subscribe`]. `kind` [`V4L2_EVENT_ALL`]
/// ends every subscription of this file handle.
pub fn unsubscribe(dev: impl AsFd, kind: u32, id: u32) -> Result<()> {
    let sub = v4l2_event_subscription {
        type_: kind,
        id,
        ..Default::default()
    };
    // SAFETY: valid fd; `sub` outlives the call.
    retry("VIDIOC_UNSUBSCRIBE_EVENT", || unsafe {
        ioctl::vidioc_unsubscribe_event(dev.as_fd().as_raw_fd(), &sub)
    })
    .map(|_| ())
}

/// Dequeues the next pending event, or returns `None` when there is none.
/// Never blocks.
pub fn dequeue(dev: impl AsFd) -> Result<Option<Event>> {
    dequeue_fd(dev.as_fd())
}

/// Dequeues pending events until none is left, the last one dequeued
/// reports nothing else pending, or `max` events have been read. Never
/// blocks. Use it after a wait reports an event (`POLLPRI`), for example
/// [`M2m::wait`](crate::m2m::M2m::wait).
pub fn drain(dev: impl AsFd, max: usize) -> Result<Vec<Event>> {
    let fd = dev.as_fd();
    let mut events = Vec::new();
    while events.len() < max {
        let Some(ev) = dequeue_fd(fd)? else {
            break;
        };
        let last = ev.pending() == 0;
        events.push(ev);
        if last {
            break;
        }
    }
    Ok(events)
}

fn dequeue_fd(fd: BorrowedFd<'_>) -> Result<Option<Event>> {
    if poll_device(fd, PollFlags::POLLPRI, Some(Duration::ZERO))?.is_none() {
        return Ok(None);
    }
    let mut raw = v4l2_event::default();
    // SAFETY: valid fd; `raw` outlives the call.
    match retry("VIDIOC_DQEVENT", || unsafe {
        ioctl::vidioc_dqevent(fd.as_raw_fd(), &mut raw)
    }) {
        Ok(_) => Ok(Some(Event { raw })),
        Err(e) if e.errno() == Some(Errno::ENOENT) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_control_and_source_change_payloads() {
        let mut raw = v4l2_event {
            type_: V4L2_EVENT_CTRL,
            id: V4L2_CID_BRIGHTNESS,
            ..Default::default()
        };
        raw.u[..4].copy_from_slice(&V4L2_EVENT_CTRL_CH_VALUE.to_ne_bytes());
        raw.u[4..8].copy_from_slice(&V4L2_CTRL_TYPE_INTEGER.to_ne_bytes());
        raw.u[8..12].copy_from_slice(&(-7i32).to_ne_bytes());
        raw.u[24..28].copy_from_slice(&255i32.to_ne_bytes());
        let ev = Event { raw };
        let c = ev.control().unwrap();
        assert_eq!(c.changes, V4L2_EVENT_CTRL_CH_VALUE);
        assert_eq!(c.value, -7);
        assert_eq!(c.maximum, 255);
        assert_eq!(ev.source_change(), None);

        let mut raw = v4l2_event {
            type_: V4L2_EVENT_SOURCE_CHANGE,
            ..Default::default()
        };
        raw.u[..4].copy_from_slice(&V4L2_EVENT_SRC_CH_RESOLUTION.to_ne_bytes());
        let ev = Event { raw };
        assert_eq!(ev.source_change(), Some(V4L2_EVENT_SRC_CH_RESOLUTION));
        assert!(ev.control().is_none());
    }

    #[test]
    fn dequeue_returns_none_without_pending_events() {
        let (r, _w) = nix::unistd::pipe().unwrap();
        assert!(dequeue(&r).unwrap().is_none());
        assert!(drain(&r, 8).unwrap().is_empty());
    }
}
