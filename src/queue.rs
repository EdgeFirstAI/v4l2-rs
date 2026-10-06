// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! Indexed V4L2 buffer queue for MMAP, DMABUF and USERPTR memory, single- and
//! multi-planar.
//!
//! A [`Queue`] drives one buffer type (capture or output) of an open device:
//! allocation with `VIDIOC_REQBUFS` or `VIDIOC_CREATE_BUFS`, `VIDIOC_QUERYBUF`,
//! mapping, `VIDIOC_EXPBUF`, `VIDIOC_QBUF`/`VIDIOC_DQBUF`, streaming and
//! waiting with `poll`.
//!
//! # Ownership rules
//!
//! - **Device.** The queue holds its own duplicate of the device file
//!   descriptor. V4L2 binds a queue to the open file, not to the descriptor
//!   number, so the duplicate drives the caller's queue, and the device stays
//!   open until both the caller's descriptor and every `Queue` built from it
//!   are dropped.
//! - **Buffer indices.** The queue tracks which indices the driver holds. An
//!   index is queued from a successful [`Queue::enqueue`] until
//!   [`Queue::dequeue`] returns it, [`Queue::stream_off`] reclaims it, or the
//!   buffers are freed. Enqueueing an index the driver already holds returns
//!   [`ErrorKind::InvalidState`] without calling the driver.
//! - **MMAP.** A [`Mapping`] keeps its pages mapped until it is dropped, even
//!   after [`Queue::free`]: the kernel keeps a mapped buffer alive (it is
//!   orphaned), so a mapping never refers to freed memory. While the index is
//!   queued the device may write to it at any time.
//! - **DMABUF import.** The kernel takes its own reference on the DMA-BUF
//!   during `VIDIOC_QBUF`, so the descriptor passed in a [`Plane::DmaBuf`]
//!   may be closed as soon as [`Queue::enqueue`] returns. The memory must not
//!   be written by the CPU while the index is queued.
//! - **DMABUF export.** A descriptor from [`Queue::export`] is independent of
//!   the queue. It keeps the buffer's memory alive after [`Queue::free`] when
//!   the driver reports [`BufferCapabilities::supports_orphaned_bufs`];
//!   without that capability, `VIDIOC_REQBUFS(0)` fails with `EBUSY` while an
//!   export is held.
//! - **USERPTR.** The caller's memory must stay valid, and must not be
//!   accessed, from [`Queue::enqueue_userptr`] until the index is dequeued or
//!   reclaimed, which is why that call is `unsafe`.
//!
//! # Threads
//!
//! [`Queue::enqueue`], [`Queue::dequeue`], [`Queue::wait`],
//! [`Queue::stream_on`] and [`Queue::stream_off`] take `&self` and may run on
//! different threads. The queue serialises the state-changing ioctls
//! (`QBUF`, `DQBUF`, `STREAMON`, `STREAMOFF`) together with its own record of
//! queued indices, so that record always follows the kernel's order.
//! [`Queue::wait`] takes no lock, and [`Queue::dequeue`] does not wait for a
//! buffer, so a thread waiting for a frame never holds up one queueing a
//! buffer.
//! Allocation and [`Queue::free`] take `&mut self`.

use std::ffi::c_void;
use std::num::NonZeroUsize;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use nix::sys::mman::{mmap, munmap, MapFlags, ProtFlags};

use crate::error::{retry, Error, ErrorKind, Result};
use crate::ioctl;
#[allow(clippy::wildcard_imports)]
use crate::uapi::*;

/// The buffer type a [`Queue`] drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufType {
    /// `V4L2_BUF_TYPE_VIDEO_CAPTURE`.
    VideoCapture,
    /// `V4L2_BUF_TYPE_VIDEO_OUTPUT`.
    VideoOutput,
    /// `V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE`.
    VideoCaptureMplane,
    /// `V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE`.
    VideoOutputMplane,
}

impl BufType {
    /// The `V4L2_BUF_TYPE_*` value.
    pub fn raw(self) -> u32 {
        match self {
            Self::VideoCapture => V4L2_BUF_TYPE_VIDEO_CAPTURE,
            Self::VideoOutput => V4L2_BUF_TYPE_VIDEO_OUTPUT,
            Self::VideoCaptureMplane => V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE,
            Self::VideoOutputMplane => V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
        }
    }

    /// Whether buffers carry a plane array (`*_MPLANE` types).
    pub fn is_multiplanar(self) -> bool {
        matches!(self, Self::VideoCaptureMplane | Self::VideoOutputMplane)
    }

    /// Whether the application fills the buffers (output) rather than the
    /// device (capture).
    pub fn is_output(self) -> bool {
        matches!(self, Self::VideoOutput | Self::VideoOutputMplane)
    }
}

/// The memory type of a queue's buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Memory {
    /// Driver-allocated buffers, mapped with [`Queue::map`] or exported with
    /// [`Queue::export`] (`V4L2_MEMORY_MMAP`).
    Mmap,
    /// Caller memory passed by address (`V4L2_MEMORY_USERPTR`).
    UserPtr,
    /// Caller DMA-BUF descriptors (`V4L2_MEMORY_DMABUF`).
    DmaBuf,
}

impl Memory {
    /// The `V4L2_MEMORY_*` value.
    pub fn raw(self) -> u32 {
        match self {
            Self::Mmap => V4L2_MEMORY_MMAP,
            Self::UserPtr => V4L2_MEMORY_USERPTR,
            Self::DmaBuf => V4L2_MEMORY_DMABUF,
        }
    }
}

/// `V4L2_BUF_CAP_*` bits reported by `VIDIOC_REQBUFS` or `VIDIOC_CREATE_BUFS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BufferCapabilities(u32);

impl BufferCapabilities {
    /// The raw `V4L2_BUF_CAP_*` bits.
    pub fn raw(self) -> u32 {
        self.0
    }
    /// The queue accepts [`Memory::Mmap`].
    pub fn supports_mmap(self) -> bool {
        self.0 & V4L2_BUF_CAP_SUPPORTS_MMAP != 0
    }
    /// The queue accepts [`Memory::UserPtr`].
    pub fn supports_userptr(self) -> bool {
        self.0 & V4L2_BUF_CAP_SUPPORTS_USERPTR != 0
    }
    /// The queue accepts [`Memory::DmaBuf`].
    pub fn supports_dmabuf(self) -> bool {
        self.0 & V4L2_BUF_CAP_SUPPORTS_DMABUF != 0
    }
    /// Buffers that are still mapped or exported survive `VIDIOC_REQBUFS(0)`
    /// and closing the device, instead of the call failing with `EBUSY`.
    pub fn supports_orphaned_bufs(self) -> bool {
        self.0 & V4L2_BUF_CAP_SUPPORTS_ORPHANED_BUFS != 0
    }
}

/// The clock a dequeued buffer's timestamp is taken from
/// (`V4L2_BUF_FLAG_TIMESTAMP_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimestampClock {
    /// The driver does not say which clock it uses.
    Unknown,
    /// `CLOCK_MONOTONIC`.
    Monotonic,
    /// Copied from the matching output buffer (memory-to-memory devices).
    Copy,
}

/// Which instant of the frame a capture timestamp marks
/// (`V4L2_BUF_FLAG_TSTAMP_SRC_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimestampSource {
    /// The last data of the frame was received (the default).
    EndOfFrame,
    /// Exposure of the first line started.
    StartOfExposure,
}

/// `V4L2_BUF_FLAG_*` bits of a dequeued buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BufferFlags(u32);

impl BufferFlags {
    /// The raw `V4L2_BUF_FLAG_*` bits.
    pub fn raw(self) -> u32 {
        self.0
    }
    /// The driver reports that the data may be corrupt
    /// (`V4L2_BUF_FLAG_ERROR`). The buffer is still dequeued normally and must
    /// be queued again.
    pub fn is_error(self) -> bool {
        self.0 & V4L2_BUF_FLAG_ERROR != 0
    }
    /// The last buffer of a memory-to-memory drain (`V4L2_BUF_FLAG_LAST`).
    pub fn is_last(self) -> bool {
        self.0 & V4L2_BUF_FLAG_LAST != 0
    }
    /// A key frame of a compressed stream (`V4L2_BUF_FLAG_KEYFRAME`).
    pub fn is_keyframe(self) -> bool {
        self.0 & V4L2_BUF_FLAG_KEYFRAME != 0
    }
    /// The clock of the buffer's timestamp.
    pub fn timestamp_clock(self) -> TimestampClock {
        match self.0 & V4L2_BUF_FLAG_TIMESTAMP_MASK {
            V4L2_BUF_FLAG_TIMESTAMP_MONOTONIC => TimestampClock::Monotonic,
            V4L2_BUF_FLAG_TIMESTAMP_COPY => TimestampClock::Copy,
            _ => TimestampClock::Unknown,
        }
    }
    /// The instant of the frame the timestamp marks.
    pub fn timestamp_source(self) -> TimestampSource {
        if self.0 & V4L2_BUF_FLAG_TSTAMP_SRC_MASK == V4L2_BUF_FLAG_TSTAMP_SRC_SOE {
            TimestampSource::StartOfExposure
        } else {
            TimestampSource::EndOfFrame
        }
    }
}

/// One plane of a buffer to enqueue with [`Queue::enqueue`].
#[derive(Debug, Clone, Copy)]
pub enum Plane<'a> {
    /// A plane of a [`Memory::Mmap`] buffer. `bytesused` is the payload of an
    /// output buffer; capture queues ignore it.
    Mmap {
        /// Bytes of valid data (output queues).
        bytesused: u32,
    },
    /// A DMA-BUF imported into a [`Memory::DmaBuf`] buffer.
    DmaBuf {
        /// The DMA-BUF. The kernel takes its own reference during the call.
        fd: BorrowedFd<'a>,
        /// Size of the plane in bytes, at least the driver's `sizeimage`.
        length: u32,
        /// Bytes of valid data (output queues).
        bytesused: u32,
        /// Offset of the data from the start of the DMA-BUF (multi-planar
        /// queues only; single-planar buffers start at offset 0).
        data_offset: u32,
    },
}

impl Plane<'_> {
    /// A capture plane of an MMAP buffer.
    pub fn mmap() -> Self {
        Plane::Mmap { bytesused: 0 }
    }
}

/// One plane of a [`Memory::UserPtr`] buffer for [`Queue::enqueue_userptr`].
#[derive(Debug, Clone, Copy)]
pub struct UserPtrPlane {
    /// Start of the caller's memory for this plane.
    pub ptr: *mut u8,
    /// Size of the memory in bytes, at least the driver's `sizeimage`.
    pub length: u32,
    /// Bytes of valid data (output queues).
    pub bytesused: u32,
}

/// Size and location of one plane of an allocated buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaneInfo {
    /// Size of the plane in bytes.
    pub length: u32,
    /// The offset to pass to `mmap` for a [`Memory::Mmap`] buffer (zero for
    /// other memory types).
    pub mem_offset: u32,
}

/// A buffer as reported by `VIDIOC_QUERYBUF`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferInfo {
    /// Buffer index.
    pub index: u32,
    /// `V4L2_BUF_FLAG_*` bits, including whether the buffer is queued.
    pub flags: BufferFlags,
    /// One entry per plane.
    pub planes: Vec<PlaneInfo>,
}

/// Payload of one plane of a dequeued buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlaneUsage {
    /// Bytes of valid data, including `data_offset`.
    pub bytesused: u32,
    /// Offset of the data from the start of the plane.
    pub data_offset: u32,
    /// Size of the plane in bytes.
    pub length: u32,
}

/// A buffer returned by [`Queue::dequeue`]. Its index stays with the caller
/// until it is enqueued again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dequeued {
    /// Buffer index.
    pub index: u32,
    /// The driver's frame counter. It does not count dropped frames on every
    /// driver.
    pub sequence: u32,
    /// `V4L2_BUF_FLAG_*` bits, including the error flag and the timestamp
    /// clock and source.
    pub flags: BufferFlags,
    /// `V4L2_FIELD_*` of the data.
    pub field: u32,
    /// The timestamp, in the clock given by
    /// [`BufferFlags::timestamp_clock`].
    pub timestamp: Duration,
    planes: [PlaneUsage; VIDEO_MAX_PLANES],
    num_planes: usize,
}

impl Dequeued {
    /// Payload of each plane.
    pub fn planes(&self) -> &[PlaneUsage] {
        &self.planes[..self.num_planes]
    }
}

/// A plane of an MMAP buffer mapped into the process. The pages stay mapped
/// until the `Mapping` is dropped.
#[derive(Debug)]
pub struct Mapping {
    ptr: NonNull<c_void>,
    len: usize,
}

// SAFETY: the mapping is plain shared memory owned by this value; nothing in
// it is tied to the creating thread.
unsafe impl Send for Mapping {}
// SAFETY: `&Mapping` only exposes the address and length; access to the
// bytes goes through the `unsafe` accessors, whose callers synchronise.
unsafe impl Sync for Mapping {}

impl Mapping {
    /// Start of the mapping.
    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr().cast()
    }

    /// Length of the mapping in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the mapping is empty (never true for a mapped plane).
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The mapped bytes.
    ///
    /// # Safety
    ///
    /// The buffer must not be queued to the device for the lifetime of the
    /// slice, and no other thread may write to it.
    pub unsafe fn as_slice(&self) -> &[u8] {
        // SAFETY: the mapping covers `len` bytes for the life of `self`; the
        // caller guarantees no concurrent writer.
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    /// The mapped bytes, writable.
    ///
    /// # Safety
    ///
    /// The buffer must not be queued to the device for the lifetime of the
    /// slice, and no other reference to the bytes may exist.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn as_mut_slice(&self) -> &mut [u8] {
        // SAFETY: as for `as_slice`, with exclusive access guaranteed by the
        // caller.
        unsafe { std::slice::from_raw_parts_mut(self.as_ptr(), self.len) }
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        // SAFETY: `ptr`/`len` came from a successful `mmap` and are unmapped
        // exactly once.
        let _ = unsafe { munmap(self.ptr, self.len) };
    }
}

/// A `v4l2_buffer` with its plane array. The plane pointer is set right
/// before each ioctl, because the value may move between calls.
struct RawBuffer {
    buf: v4l2_buffer,
    planes: [v4l2_plane; VIDEO_MAX_PLANES],
}

impl RawBuffer {
    fn new(buf_type: BufType, memory: u32, index: u32) -> Self {
        Self {
            buf: v4l2_buffer {
                index,
                type_: buf_type.raw(),
                memory,
                ..Default::default()
            },
            planes: [v4l2_plane::default(); VIDEO_MAX_PLANES],
        }
    }

    /// Points the buffer at its plane array, reporting `planes` entries, and
    /// returns the pointer to pass to the ioctl. Nothing may write to `self`
    /// between this call and the ioctl.
    fn prepare(&mut self, buf_type: BufType, planes: usize) -> *mut v4l2_buffer {
        if buf_type.is_multiplanar() {
            self.buf.length = planes as u32;
            self.buf.set_planes(self.planes.as_mut_ptr());
        }
        &mut self.buf
    }
}

/// One buffer type of an open V4L2 device. See the [module documentation](self)
/// for the ownership rules.
#[derive(Debug)]
pub struct Queue {
    fd: OwnedFd,
    buf_type: BufType,
    memory: Option<Memory>,
    capabilities: BufferCapabilities,
    num_planes: usize,
    queued: Vec<AtomicBool>,
    streaming: AtomicBool,
    ops: Mutex<()>,
}

impl Queue {
    /// Creates a queue for `buf_type` on the open device `device`. No buffers
    /// are allocated.
    pub fn new(device: impl AsFd, buf_type: BufType) -> Result<Self> {
        let fd = device.as_fd().try_clone_to_owned().map_err(|e| {
            Error::from_errno("dup", Errno::from_raw(e.raw_os_error().unwrap_or(0)))
        })?;
        Ok(Self {
            fd,
            buf_type,
            memory: None,
            capabilities: BufferCapabilities::default(),
            num_planes: 0,
            queued: Vec::new(),
            streaming: AtomicBool::new(false),
            ops: Mutex::new(()),
        })
    }

    /// The buffer type.
    pub fn buf_type(&self) -> BufType {
        self.buf_type
    }

    /// The memory type of the allocated buffers, or `None` before allocation
    /// or after [`Queue::free`].
    pub fn memory(&self) -> Option<Memory> {
        self.memory
    }

    /// The capabilities reported by the last allocation (all false before).
    pub fn capabilities(&self) -> BufferCapabilities {
        self.capabilities
    }

    /// Number of allocated buffers.
    pub fn len(&self) -> usize {
        self.queued.len()
    }

    /// Whether no buffers are allocated.
    pub fn is_empty(&self) -> bool {
        self.queued.is_empty()
    }

    /// Planes per buffer (1 for single-planar types; 0 before allocation).
    pub fn num_planes(&self) -> usize {
        self.num_planes
    }

    /// Whether the driver currently holds buffer `index`.
    pub fn is_queued(&self, index: u32) -> bool {
        self.queued
            .get(index as usize)
            .is_some_and(|q| q.load(Ordering::Acquire))
    }

    /// Number of buffers the driver currently holds.
    pub fn queued_count(&self) -> usize {
        self.queued
            .iter()
            .filter(|q| q.load(Ordering::Acquire))
            .count()
    }

    /// Whether `VIDIOC_STREAMON` succeeded and no `VIDIOC_STREAMOFF` followed.
    pub fn is_streaming(&self) -> bool {
        self.streaming.load(Ordering::Acquire)
    }

    /// Allocates `count` buffers of `memory` with `VIDIOC_REQBUFS` and
    /// returns the number the driver granted, which may differ. Any previous
    /// buffers are released first, including buffers of another memory type.
    /// `count == 0` is the same as [`Queue::free`].
    ///
    /// The plane sizes come from the format currently set on the device
    /// (`VIDIOC_S_FMT`). Fails with [`ErrorKind::InvalidState`] while
    /// streaming.
    pub fn request(&mut self, memory: Memory, count: u32) -> Result<u32> {
        if count == 0 {
            return self.free().map(|()| 0);
        }
        if self.is_streaming() {
            return Err(Error::new(ErrorKind::InvalidState, "VIDIOC_REQBUFS"));
        }
        if let Some(current) = self.memory.filter(|&m| m != memory) {
            self.reqbufs(current, 0)?;
        }
        self.reqbufs(memory, count)
    }

    /// Adds up to `count` buffers of `memory` sized for `format` with
    /// `VIDIOC_CREATE_BUFS`, and returns the indices that were added.
    ///
    /// Not every driver implements it (the i.MX 8M Plus ISP returns `ENOTTY`,
    /// reported as [`ErrorKind::Unsupported`]); [`Queue::request`] always
    /// works. Every buffer of a queue must use the same memory type.
    pub fn create_buffers(
        &mut self,
        memory: Memory,
        count: u32,
        format: &v4l2_format,
    ) -> Result<std::ops::Range<u32>> {
        if self.memory.is_some_and(|m| m != memory) {
            return Err(Error::new(ErrorKind::InvalidArgument, "VIDIOC_CREATE_BUFS"));
        }
        let mut cb = v4l2_create_buffers {
            count,
            memory: memory.raw(),
            format: *format,
            ..Default::default()
        };
        cb.format.type_ = self.buf_type.raw();
        // SAFETY: valid fd; `cb` is initialised and outlives the call.
        retry("VIDIOC_CREATE_BUFS", || unsafe {
            ioctl::vidioc_create_bufs(self.fd.as_raw_fd(), &mut cb)
        })?;
        self.capabilities = BufferCapabilities(cb.capabilities);
        let added = cb.index..cb.index + cb.count;
        let total = (added.end as usize).max(self.queued.len());
        self.queued.resize_with(total, || AtomicBool::new(false));
        if cb.count > 0 {
            self.memory = Some(memory);
            if self.num_planes == 0 {
                self.num_planes = self.query(added.start)?.planes.len();
            }
        }
        Ok(added)
    }

    /// Stops streaming if needed and releases every buffer with
    /// `VIDIOC_REQBUFS(0)` for the memory type in use. Mapped and exported
    /// buffers stay valid where the driver supports orphaned buffers (see the
    /// module documentation).
    pub fn free(&mut self) -> Result<()> {
        if self.is_streaming() {
            self.stream_off()?;
        }
        let memory = self.memory.unwrap_or(Memory::Mmap);
        self.reqbufs(memory, 0).map(|_| ())
    }

    fn reqbufs(&mut self, memory: Memory, count: u32) -> Result<u32> {
        let mut rb = v4l2_requestbuffers {
            count,
            type_: self.buf_type.raw(),
            memory: memory.raw(),
            ..Default::default()
        };
        // SAFETY: valid fd; `rb` is initialised and outlives the call.
        retry("VIDIOC_REQBUFS", || unsafe {
            ioctl::vidioc_reqbufs(self.fd.as_raw_fd(), &mut rb)
        })?;
        self.capabilities = BufferCapabilities(rb.capabilities);
        self.queued = (0..rb.count).map(|_| AtomicBool::new(false)).collect();
        if rb.count == 0 {
            self.memory = None;
            self.num_planes = 0;
        } else {
            self.memory = Some(memory);
            self.num_planes = self.query(0)?.planes.len();
        }
        Ok(rb.count)
    }

    /// Reports buffer `index` with `VIDIOC_QUERYBUF`.
    pub fn query(&self, index: u32) -> Result<BufferInfo> {
        let memory = self.allocated("VIDIOC_QUERYBUF", index)?;
        let mut raw = RawBuffer::new(self.buf_type, memory.raw(), index);
        let p = raw.prepare(self.buf_type, VIDEO_MAX_PLANES);
        // SAFETY: valid fd; `p` points into `raw`, which outlives the call,
        // and its plane array has VIDEO_MAX_PLANES entries.
        retry("VIDIOC_QUERYBUF", || unsafe {
            ioctl::vidioc_querybuf(self.fd.as_raw_fd(), p)
        })?;
        let planes = if self.buf_type.is_multiplanar() {
            let n = (raw.buf.length as usize).min(VIDEO_MAX_PLANES);
            raw.planes[..n]
                .iter()
                .map(|pl| PlaneInfo {
                    length: pl.length,
                    mem_offset: if memory == Memory::Mmap {
                        pl.mem_offset()
                    } else {
                        0
                    },
                })
                .collect()
        } else {
            vec![PlaneInfo {
                length: raw.buf.length,
                mem_offset: if memory == Memory::Mmap {
                    raw.buf.offset()
                } else {
                    0
                },
            }]
        };
        Ok(BufferInfo {
            index,
            flags: BufferFlags(raw.buf.flags),
            planes,
        })
    }

    /// Maps every plane of MMAP buffer `index` read-write.
    pub fn map(&self, index: u32) -> Result<Vec<Mapping>> {
        if self.allocated("mmap", index)? != Memory::Mmap {
            return Err(Error::new(ErrorKind::InvalidState, "mmap"));
        }
        self.query(index)?
            .planes
            .iter()
            .map(|pl| {
                let len = NonZeroUsize::new(pl.length as usize)
                    .ok_or(Error::new(ErrorKind::InvalidState, "mmap"))?;
                // SAFETY: a fresh shared mapping of a driver buffer; the
                // offset and length come from VIDIOC_QUERYBUF.
                let ptr = retry("mmap", || unsafe {
                    mmap(
                        None,
                        len,
                        ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                        MapFlags::MAP_SHARED,
                        &self.fd,
                        libc::off_t::from(pl.mem_offset),
                    )
                })?;
                Ok(Mapping {
                    ptr,
                    len: len.get(),
                })
            })
            .collect()
    }

    /// Exports plane `plane` of MMAP buffer `index` as a DMA-BUF with
    /// `VIDIOC_EXPBUF` (read-write, close-on-exec).
    pub fn export(&self, index: u32, plane: u32) -> Result<OwnedFd> {
        if self.allocated("VIDIOC_EXPBUF", index)? != Memory::Mmap {
            return Err(Error::new(ErrorKind::InvalidState, "VIDIOC_EXPBUF"));
        }
        let mut eb = v4l2_exportbuffer {
            type_: self.buf_type.raw(),
            index,
            plane,
            flags: (libc::O_CLOEXEC | libc::O_RDWR) as u32,
            ..Default::default()
        };
        // SAFETY: valid fd; `eb` is initialised and outlives the call.
        retry("VIDIOC_EXPBUF", || unsafe {
            ioctl::vidioc_expbuf(self.fd.as_raw_fd(), &mut eb)
        })?;
        // SAFETY: the kernel returned a new descriptor that nothing else owns.
        Ok(unsafe { OwnedFd::from_raw_fd(eb.fd) })
    }

    /// Queues buffer `index` with one [`Plane`] per plane, matching the
    /// queue's memory type. `timestamp` is set on output buffers (a
    /// memory-to-memory device copies it to the matching capture buffer).
    pub fn enqueue(
        &self,
        index: u32,
        planes: &[Plane<'_>],
        timestamp: Option<Duration>,
    ) -> Result<()> {
        let memory = self.allocated("VIDIOC_QBUF", index)?;
        self.check_planes(planes.len())?;
        let mut raw = RawBuffer::new(self.buf_type, memory.raw(), index);
        for (i, plane) in planes.iter().enumerate() {
            let (bytesused, length, data_offset) = match (*plane, memory) {
                (Plane::Mmap { bytesused }, Memory::Mmap) => (bytesused, 0, 0),
                (
                    Plane::DmaBuf {
                        fd,
                        length,
                        bytesused,
                        data_offset,
                    },
                    Memory::DmaBuf,
                ) => {
                    if self.buf_type.is_multiplanar() {
                        raw.planes[i].set_fd(fd.as_raw_fd());
                    } else {
                        raw.buf.set_fd(fd.as_raw_fd());
                    }
                    (bytesused, length, data_offset)
                }
                _ => return Err(Error::new(ErrorKind::InvalidArgument, "VIDIOC_QBUF")),
            };
            self.fill_plane(&mut raw, i, bytesused, length, data_offset);
        }
        self.queue_raw(raw, timestamp)
    }

    /// Queues USERPTR buffer `index` with one [`UserPtrPlane`] per plane.
    ///
    /// # Safety
    ///
    /// Each plane's `ptr` must point to `length` bytes that stay valid, and
    /// are not accessed by the process, until the index is dequeued, reclaimed
    /// by [`Queue::stream_off`], or freed.
    pub unsafe fn enqueue_userptr(
        &self,
        index: u32,
        planes: &[UserPtrPlane],
        timestamp: Option<Duration>,
    ) -> Result<()> {
        if self.allocated("VIDIOC_QBUF", index)? != Memory::UserPtr {
            return Err(Error::new(ErrorKind::InvalidArgument, "VIDIOC_QBUF"));
        }
        self.check_planes(planes.len())?;
        let mut raw = RawBuffer::new(self.buf_type, V4L2_MEMORY_USERPTR, index);
        for (i, plane) in planes.iter().enumerate() {
            if self.buf_type.is_multiplanar() {
                raw.planes[i].set_userptr(plane.ptr);
            } else {
                raw.buf.set_userptr(plane.ptr);
            }
            self.fill_plane(&mut raw, i, plane.bytesused, plane.length, 0);
        }
        self.queue_raw(raw, timestamp)
    }

    /// Dequeues the next finished buffer with `VIDIOC_DQBUF`, or returns
    /// `None` when none is ready; use [`Queue::wait`] to sleep until a buffer
    /// is ready. The readiness check and `VIDIOC_DQBUF` run under the queue's
    /// lock, so it never blocks on a descriptor opened with `O_NONBLOCK`, nor
    /// on a blocking one unless something outside this `Queue` dequeues the
    /// same buffer type from the same open file concurrently.
    ///
    /// A buffer flagged [`BufferFlags::is_error`] is returned like any other.
    /// After the last buffer of a memory-to-memory drain the driver reports
    /// [`ErrorKind::EndOfStream`]. Fails with [`ErrorKind::InvalidState`]
    /// when no buffers are allocated or the queue is not streaming.
    pub fn dequeue(&self) -> Result<Option<Dequeued>> {
        let _ops = self.lock();
        let memory = self
            .memory
            .filter(|_| self.is_streaming())
            .ok_or(Error::new(ErrorKind::InvalidState, "VIDIOC_DQBUF"))?;
        // Only the lock holder dequeues, so a buffer `poll` reports as done is
        // still there for `VIDIOC_DQBUF`, which therefore cannot block.
        if !self.ready_now()? {
            return Ok(None);
        }
        let mut raw = RawBuffer::new(self.buf_type, memory.raw(), 0);
        let p = raw.prepare(self.buf_type, VIDEO_MAX_PLANES);
        // SAFETY: valid fd; `p` points into `raw`, which outlives the call.
        match retry("VIDIOC_DQBUF", || unsafe {
            ioctl::vidioc_dqbuf(self.fd.as_raw_fd(), p)
        }) {
            Err(e) if e.errno() == Some(Errno::EAGAIN) => return Ok(None),
            r => r?,
        };
        if let Some(q) = self.queued.get(raw.buf.index as usize) {
            q.store(false, Ordering::Release);
        }
        let mut planes = [PlaneUsage::default(); VIDEO_MAX_PLANES];
        let num_planes = if self.buf_type.is_multiplanar() {
            let n = (raw.buf.length as usize).min(VIDEO_MAX_PLANES);
            for (dst, src) in planes.iter_mut().zip(&raw.planes[..n]) {
                *dst = PlaneUsage {
                    bytesused: src.bytesused,
                    data_offset: src.data_offset,
                    length: src.length,
                };
            }
            n
        } else {
            planes[0] = PlaneUsage {
                bytesused: raw.buf.bytesused,
                data_offset: 0,
                length: raw.buf.length,
            };
            1
        };
        let tv = raw.buf.timestamp;
        let timestamp = Duration::new(
            u64::try_from(tv.tv_sec).unwrap_or(0),
            u32::try_from(tv.tv_usec).unwrap_or(0).saturating_mul(1000),
        );
        Ok(Some(Dequeued {
            index: raw.buf.index,
            sequence: raw.buf.sequence,
            flags: BufferFlags(raw.buf.flags),
            field: raw.buf.field,
            timestamp,
            planes,
            num_planes,
        }))
    }

    /// Starts the stream with `VIDIOC_STREAMON`.
    pub fn stream_on(&self) -> Result<()> {
        let _ops = self.lock();
        let t = self.buf_type.raw() as libc::c_int;
        // SAFETY: valid fd; `t` outlives the call.
        retry("VIDIOC_STREAMON", || unsafe {
            ioctl::vidioc_streamon(self.fd.as_raw_fd(), &t)
        })?;
        self.streaming.store(true, Ordering::Release);
        Ok(())
    }

    /// Stops the stream with `VIDIOC_STREAMOFF`. The driver returns every
    /// queued buffer to the application, so all indices become free to
    /// enqueue again.
    pub fn stream_off(&self) -> Result<()> {
        let _ops = self.lock();
        let t = self.buf_type.raw() as libc::c_int;
        // SAFETY: valid fd; `t` outlives the call.
        retry("VIDIOC_STREAMOFF", || unsafe {
            ioctl::vidioc_streamoff(self.fd.as_raw_fd(), &t)
        })?;
        self.streaming.store(false, Ordering::Release);
        for q in &self.queued {
            q.store(false, Ordering::Release);
        }
        Ok(())
    }

    /// Waits up to `timeout` (forever when `None`) until a buffer can be
    /// dequeued, and returns `false` on timeout. Interrupted waits resume
    /// with the remaining time.
    ///
    /// Fails with [`ErrorKind::InvalidState`] when the driver reports an
    /// error condition instead (typically: not streaming, or no buffer
    /// queued), and with [`ErrorKind::Disconnected`] when the device is gone.
    pub fn wait(&self, timeout: Option<Duration>) -> Result<bool> {
        let ready = self.ready_flag();
        let revents = poll_device(self.fd.as_fd(), ready, timeout)?;
        Ok(revents.is_some_and(|r| r.intersects(ready)))
    }

    fn ready_flag(&self) -> PollFlags {
        if self.buf_type.is_output() {
            PollFlags::POLLOUT
        } else {
            PollFlags::POLLIN
        }
    }

    /// Whether a buffer can be dequeued right now. An error condition from
    /// `poll` (for example nothing queued) means nothing to dequeue.
    fn ready_now(&self) -> Result<bool> {
        let ready = self.ready_flag();
        match poll_device(self.fd.as_fd(), ready, Some(Duration::ZERO)) {
            Ok(r) => Ok(r.is_some_and(|r| r.intersects(ready))),
            Err(e) if e.kind() == ErrorKind::InvalidState => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn lock(&self) -> MutexGuard<'_, ()> {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }

    fn allocated(&self, op: &'static str, index: u32) -> Result<Memory> {
        match self.memory {
            Some(m) if (index as usize) < self.queued.len() => Ok(m),
            Some(_) => Err(Error::new(ErrorKind::InvalidArgument, op)),
            None => Err(Error::new(ErrorKind::InvalidState, op)),
        }
    }

    fn check_planes(&self, n: usize) -> Result<()> {
        let expected = if self.buf_type.is_multiplanar() {
            self.num_planes
        } else {
            1
        };
        if n == expected {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::InvalidArgument, "VIDIOC_QBUF"))
        }
    }

    fn fill_plane(
        &self,
        raw: &mut RawBuffer,
        i: usize,
        bytesused: u32,
        length: u32,
        data_offset: u32,
    ) {
        if self.buf_type.is_multiplanar() {
            raw.planes[i].bytesused = bytesused;
            raw.planes[i].length = length;
            raw.planes[i].data_offset = data_offset;
        } else {
            raw.buf.bytesused = bytesused;
            raw.buf.length = length;
        }
    }

    fn queue_raw(&self, mut raw: RawBuffer, timestamp: Option<Duration>) -> Result<()> {
        let index = raw.buf.index;
        let slot = &self.queued[index as usize];
        let _ops = self.lock();
        if slot.load(Ordering::Acquire) {
            return Err(Error::new(ErrorKind::InvalidState, "VIDIOC_QBUF"));
        }
        if let Some(ts) = timestamp {
            raw.buf.timestamp = libc::timeval {
                tv_sec: ts.as_secs() as libc::time_t,
                tv_usec: libc::suseconds_t::from(ts.subsec_micros()),
            };
        }
        let p = raw.prepare(self.buf_type, self.num_planes);
        // SAFETY: valid fd; `p` points into `raw`, which outlives the call;
        // DMA-BUF descriptors are borrowed for the call and USERPTR memory is
        // guaranteed by the caller of `enqueue_userptr`.
        retry("VIDIOC_QBUF", || unsafe {
            ioctl::vidioc_qbuf(self.fd.as_raw_fd(), p)
        })?;
        slot.store(true, Ordering::Release);
        Ok(())
    }
}

/// Polls `fd` for `events` (plus the error conditions poll always reports)
/// until one is ready or `timeout` elapses, resuming after `EINTR`. Returns
/// the ready events, or `None` on timeout.
pub(crate) fn poll_device(
    fd: BorrowedFd<'_>,
    events: PollFlags,
    timeout: Option<Duration>,
) -> Result<Option<PollFlags>> {
    let deadline = timeout.map(|t| Instant::now() + t);
    loop {
        let wait = match deadline {
            None => PollTimeout::NONE,
            Some(d) => {
                let left = d.saturating_duration_since(Instant::now());
                let ms = left.as_micros().div_ceil(1000).min(i32::MAX as u128);
                PollTimeout::try_from(ms).unwrap_or(PollTimeout::MAX)
            }
        };
        let mut fds = [PollFd::new(fd, events)];
        match poll(&mut fds, wait) {
            Err(Errno::EINTR) => continue,
            Err(e) => return Err(Error::from_errno("poll", e)),
            Ok(0) => return Ok(None),
            Ok(_) => {
                let revents = fds[0].revents().unwrap_or(PollFlags::empty());
                if revents.contains(PollFlags::POLLHUP) {
                    return Err(Error::new(ErrorKind::Disconnected, "poll"));
                }
                if revents.contains(PollFlags::POLLNVAL) {
                    return Err(Error::new(ErrorKind::Io, "poll"));
                }
                if revents.intersects(events) {
                    return Ok(Some(revents));
                }
                if revents.contains(PollFlags::POLLERR) {
                    return Err(Error::new(ErrorKind::InvalidState, "poll"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buf_type_properties() {
        assert_eq!(BufType::VideoCapture.raw(), V4L2_BUF_TYPE_VIDEO_CAPTURE);
        assert_eq!(
            BufType::VideoOutputMplane.raw(),
            V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE
        );
        assert!(BufType::VideoCaptureMplane.is_multiplanar());
        assert!(!BufType::VideoCapture.is_multiplanar());
        assert!(BufType::VideoOutput.is_output());
        assert!(!BufType::VideoCaptureMplane.is_output());
        assert_eq!(Memory::DmaBuf.raw(), V4L2_MEMORY_DMABUF);
    }

    #[test]
    fn buffer_flags_decode() {
        let f = BufferFlags(
            V4L2_BUF_FLAG_ERROR | V4L2_BUF_FLAG_TIMESTAMP_MONOTONIC | V4L2_BUF_FLAG_TSTAMP_SRC_SOE,
        );
        assert!(f.is_error());
        assert!(!f.is_last());
        assert_eq!(f.timestamp_clock(), TimestampClock::Monotonic);
        assert_eq!(f.timestamp_source(), TimestampSource::StartOfExposure);

        let f = BufferFlags(V4L2_BUF_FLAG_LAST | V4L2_BUF_FLAG_TIMESTAMP_COPY);
        assert!(f.is_last());
        assert_eq!(f.timestamp_clock(), TimestampClock::Copy);
        assert_eq!(f.timestamp_source(), TimestampSource::EndOfFrame);
        assert_eq!(BufferFlags(0).timestamp_clock(), TimestampClock::Unknown);
    }

    #[test]
    fn capabilities_decode() {
        let c =
            BufferCapabilities(V4L2_BUF_CAP_SUPPORTS_MMAP | V4L2_BUF_CAP_SUPPORTS_ORPHANED_BUFS);
        assert!(c.supports_mmap());
        assert!(c.supports_orphaned_bufs());
        assert!(!c.supports_dmabuf());
        assert!(!c.supports_userptr());
    }

    #[test]
    fn unallocated_queue_rejects_buffer_calls() {
        let dev = std::fs::File::open("/dev/null").unwrap();
        let q = Queue::new(&dev, BufType::VideoCapture).unwrap();
        assert!(q.is_empty());
        assert_eq!(q.memory(), None);
        assert_eq!(q.query(0).unwrap_err().kind(), ErrorKind::InvalidState);
        assert_eq!(
            q.enqueue(0, &[Plane::mmap()], None).unwrap_err().kind(),
            ErrorKind::InvalidState
        );
        assert_eq!(q.dequeue().unwrap_err().kind(), ErrorKind::InvalidState);
        assert_eq!(q.export(0, 0).unwrap_err().kind(), ErrorKind::InvalidState);
    }

    #[test]
    fn ioctls_on_a_non_v4l2_fd_are_unsupported() {
        let dev = std::fs::File::open("/dev/null").unwrap();
        let mut q = Queue::new(&dev, BufType::VideoCapture).unwrap();
        assert_eq!(
            q.request(Memory::Mmap, 4).unwrap_err().kind(),
            ErrorKind::Unsupported
        );
        assert_eq!(q.stream_on().unwrap_err().op(), "VIDIOC_STREAMON");
    }

    #[test]
    fn poll_times_out() {
        let (r, _w) = nix::unistd::pipe().unwrap();
        let t = Instant::now();
        let got = poll_device(
            r.as_fd(),
            PollFlags::POLLIN,
            Some(Duration::from_millis(20)),
        )
        .unwrap();
        assert_eq!(got, None);
        assert!(t.elapsed() >= Duration::from_millis(20));
    }

    #[test]
    fn poll_reports_hangup_as_disconnected() {
        let (r, w) = nix::unistd::pipe().unwrap();
        drop(w);
        let e = poll_device(
            r.as_fd(),
            PollFlags::POLLIN,
            Some(Duration::from_millis(100)),
        )
        .unwrap_err();
        assert_eq!(e.kind(), ErrorKind::Disconnected);
    }
}
