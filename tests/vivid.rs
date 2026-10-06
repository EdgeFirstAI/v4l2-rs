// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! Buffer queue tests against the kernel's virtual drivers: `vivid` for
//! capture (one single-planar and one multi-planar instance) and `vim2m` for
//! memory-to-memory. Load them with:
//!
//! ```sh
//! sudo modprobe vivid n_devs=2 node_types=0x1,0x1 multiplanar=1,2
//! sudo modprobe vim2m
//! ```
//!
//! Without the drivers each test prints `SKIPPED` and passes, unless
//! `EDGEFIRST_V4L2_REQUIRE_VIVID=1` is set (CI), which turns a missing driver
//! into a failure.

#![cfg(target_os = "linux")]

use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::num::NonZeroUsize;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::time::Duration;

use edgefirst_v4l2::ioctl;
use edgefirst_v4l2::m2m::M2m;
use edgefirst_v4l2::queue::{
    BufType, Dequeued, Memory, Plane, Queue, TimestampClock, UserPtrPlane,
};
use edgefirst_v4l2::uapi::*;
use edgefirst_v4l2::ErrorKind;
use nix::sys::mman::{mmap, munmap, MapFlags, ProtFlags};

const FRAME_TIMEOUT: Duration = Duration::from_secs(2);

fn skip(what: &str) {
    skip_because(what, "");
}

fn skip_because(what: &str, why: &str) {
    if std::env::var_os("EDGEFIRST_V4L2_REQUIRE_VIVID").is_some() {
        panic!("{what} is required (EDGEFIRST_V4L2_REQUIRE_VIVID is set) but not available{why}");
    }
    eprintln!("SKIPPED: {what} not available{why}");
}

/// An open, `flock`ed device node. The lock serialises tests that share a
/// node across test processes.
struct Device {
    file: File,
    path: String,
}

impl Device {
    fn open(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path)?;
        Ok(Self {
            file,
            path: path.to_owned(),
        })
    }

    fn open_blocking(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(path)?;
        Ok(Self {
            file,
            path: path.to_owned(),
        })
    }

    fn lock(&self) {
        // SAFETY: valid fd.
        assert_eq!(
            unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_EX) },
            0
        );
    }

    fn caps(&self) -> v4l2_capability {
        let mut cap = v4l2_capability::default();
        // SAFETY: valid fd and argument.
        unsafe { ioctl::vidioc_querycap(self.file.as_raw_fd(), &mut cap) }.unwrap();
        cap
    }

    fn get_format(&self, buf_type: BufType) -> v4l2_format {
        let mut fmt = v4l2_format {
            type_: buf_type.raw(),
            ..Default::default()
        };
        // SAFETY: valid fd and argument.
        unsafe { ioctl::vidioc_g_fmt(self.file.as_raw_fd(), &mut fmt) }.unwrap();
        fmt
    }

    fn set_format(&self, fmt: &mut v4l2_format) {
        // SAFETY: valid fd and argument.
        unsafe { ioctl::vidioc_s_fmt(self.file.as_raw_fd(), fmt) }.unwrap();
    }
}

impl AsFd for Device {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.file.as_fd()
    }
}

/// Finds the node of `driver` whose effective capabilities include `cap`,
/// opens and locks it. On failure, returns what each node was rejected for.
fn find(driver: &str, cap: u32) -> Result<Device, String> {
    let mut nodes: Vec<_> = std::fs::read_dir("/dev")
        .map_err(|e| format!("/dev: {e}"))?
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("video"))
        .collect();
    nodes.sort();
    let mut tried = Vec::new();
    for node in nodes {
        let path = format!("/dev/{node}");
        let dev = match Device::open(&path) {
            Ok(d) => d,
            Err(e) => {
                tried.push(format!("{path}: {e}"));
                continue;
            }
        };
        let c = dev.caps();
        let name = CStr::from_bytes_until_nul(&c.driver)
            .ok()
            .and_then(|n| n.to_str().ok())
            .unwrap_or("?");
        if name == driver && c.effective_caps() & cap != 0 {
            dev.lock();
            return Ok(dev);
        }
        tried.push(format!(
            "{path}: driver {name}, caps {:#x}",
            c.effective_caps()
        ));
    }
    Err(format!(" (tried: [{}])", tried.join("; ")))
}

fn vivid(mplane: bool) -> Option<(Device, BufType)> {
    let (cap, buf_type) = if mplane {
        (V4L2_CAP_VIDEO_CAPTURE_MPLANE, BufType::VideoCaptureMplane)
    } else {
        (V4L2_CAP_VIDEO_CAPTURE, BufType::VideoCapture)
    };
    match find("vivid", cap) {
        Ok(d) => Some((d, buf_type)),
        Err(why) => {
            let what = if mplane {
                "vivid (multi-planar)"
            } else {
                "vivid (single-planar)"
            };
            skip_because(what, &why);
            None
        }
    }
}

/// Plane sizes of the device's current format.
fn plane_sizes(dev: &Device, buf_type: BufType) -> Vec<u32> {
    let mut fmt = dev.get_format(buf_type);
    if buf_type.is_multiplanar() {
        // SAFETY: the type selects the multi-planar payload.
        let mp = unsafe { fmt.pix_mp() };
        mp.plane_fmt[..mp.num_planes as usize]
            .iter()
            .map(|p| p.sizeimage)
            .collect()
    } else {
        // SAFETY: the type selects the single-planar payload.
        vec![unsafe { fmt.pix() }.sizeimage]
    }
}

/// Waits for and dequeues one buffer, checking what every capture buffer
/// must satisfy.
fn next(q: &Queue) -> Dequeued {
    assert!(
        q.wait(Some(FRAME_TIMEOUT)).unwrap(),
        "no frame within {FRAME_TIMEOUT:?}"
    );
    let d = q.dequeue().unwrap().expect("poll reported a buffer ready");
    assert!(!d.flags.is_error());
    assert_eq!(d.flags.timestamp_clock(), TimestampClock::Monotonic);
    assert_eq!(d.planes().len(), q.num_planes());
    assert!(d.planes().iter().all(|p| p.bytesused > 0));
    assert!(!q.is_queued(d.index));
    d
}

/// Captures `n` frames, requeueing each with `requeue`, and checks that the
/// sequence and timestamps advance.
fn capture(q: &Queue, n: usize, requeue: impl Fn(u32)) -> Vec<Dequeued> {
    let frames: Vec<_> = (0..n)
        .map(|_| {
            let d = next(q);
            requeue(d.index);
            d
        })
        .collect();
    for w in frames.windows(2) {
        assert!(w[1].sequence > w[0].sequence);
        assert!(w[1].timestamp > w[0].timestamp);
    }
    frames
}

fn not_all_zero(bytes: &[u8]) -> bool {
    bytes.iter().any(|&b| b != 0)
}

/// Maps `len` bytes of a DMA-BUF read-only and passes them to `f`.
fn with_dmabuf_bytes<R>(fd: &OwnedFd, len: usize, f: impl FnOnce(&[u8]) -> R) -> R {
    let n = NonZeroUsize::new(len).unwrap();
    // SAFETY: a fresh read-only shared mapping, unmapped below.
    let ptr = unsafe { mmap(None, n, ProtFlags::PROT_READ, MapFlags::MAP_SHARED, fd, 0) }.unwrap();
    // SAFETY: the mapping covers `len` bytes; the buffer is not queued.
    let r = f(unsafe { std::slice::from_raw_parts(ptr.as_ptr().cast::<u8>(), len) });
    // SAFETY: unmapping the mapping created above.
    unsafe { munmap(ptr, len) }.unwrap();
    r
}

fn mmap_capture(mplane: bool) {
    let Some((dev, buf_type)) = vivid(mplane) else {
        return;
    };
    let mut q = Queue::new(&dev, buf_type).unwrap();
    let granted = q.request(Memory::Mmap, 4).unwrap();
    assert!(granted >= 2, "{}: granted {granted}", dev.path);
    assert!(q.capabilities().supports_mmap());
    let sizes = plane_sizes(&dev, buf_type);
    assert_eq!(q.num_planes(), sizes.len());

    let maps: Vec<_> = (0..granted).map(|i| q.map(i).unwrap()).collect();
    for (i, m) in maps.iter().enumerate() {
        assert_eq!(m.len(), q.num_planes());
        for (plane, size) in m.iter().zip(&sizes) {
            assert!(plane.len() >= *size as usize);
        }
        let planes = vec![Plane::mmap(); q.num_planes()];
        q.enqueue(i as u32, &planes, None).unwrap();
    }
    assert_eq!(q.queued_count(), granted as usize);
    q.stream_on().unwrap();

    let planes = vec![Plane::mmap(); q.num_planes()];
    let frames = capture(&q, 6, |i| {
        // SAFETY: the buffer was just dequeued and is not queued.
        let first = unsafe { maps[i as usize][0].as_slice() };
        assert!(not_all_zero(first));
        q.enqueue(i, &planes, None).unwrap();
    });
    assert_eq!(frames.len(), 6);

    q.stream_off().unwrap();
    assert_eq!(q.queued_count(), 0);
    assert!(!q.is_streaming());
    q.free().unwrap();
    assert!(q.is_empty());
    drop(maps);
}

#[test]
fn mmap_capture_single_planar() {
    mmap_capture(false);
}

#[test]
fn mmap_capture_multi_planar() {
    mmap_capture(true);
}

#[repr(C)]
struct DmaHeapAllocationData {
    len: u64,
    fd: u32,
    fd_flags: u32,
    heap_flags: u64,
}
nix::ioctl_readwrite!(
    /// `DMA_HEAP_IOCTL_ALLOC` = `_IOWR('H', 0, struct dma_heap_allocation_data)`.
    dma_heap_alloc,
    b'H',
    0,
    DmaHeapAllocationData
);

/// Allocates `len` bytes from the system DMA heap, or `None` when the heap
/// is not accessible.
fn heap_alloc(len: u32) -> Option<OwnedFd> {
    let heap = File::open("/dev/dma_heap/system").ok()?;
    let mut data = DmaHeapAllocationData {
        len: u64::from(len),
        fd: 0,
        fd_flags: (libc::O_RDWR | libc::O_CLOEXEC) as u32,
        heap_flags: 0,
    };
    // SAFETY: valid fd and argument.
    unsafe { dma_heap_alloc(heap.as_raw_fd(), &mut data) }.ok()?;
    // SAFETY: the kernel returned a new descriptor that nothing else owns.
    Some(unsafe { OwnedFd::from_raw_fd(data.fd as i32) })
}

fn dmabuf_heap_capture(mplane: bool) {
    let Some((dev, buf_type)) = vivid(mplane) else {
        return;
    };
    let sizes = plane_sizes(&dev, buf_type);
    let Some(probe) = heap_alloc(4096) else {
        skip("/dev/dma_heap/system");
        return;
    };
    drop(probe);

    let mut q = Queue::new(&dev, buf_type).unwrap();
    let granted = q.request(Memory::DmaBuf, 4).unwrap();
    assert!(q.capabilities().supports_dmabuf());
    let bufs: Vec<Vec<OwnedFd>> = (0..granted)
        .map(|_| sizes.iter().map(|&s| heap_alloc(s).unwrap()).collect())
        .collect();
    let enqueue = |i: u32| {
        let planes: Vec<_> = bufs[i as usize]
            .iter()
            .zip(&sizes)
            .map(|(fd, &length)| Plane::DmaBuf {
                fd: fd.as_fd(),
                length,
                bytesused: 0,
                data_offset: 0,
            })
            .collect();
        q.enqueue(i, &planes, None).unwrap();
    };
    (0..granted).for_each(enqueue);
    q.stream_on().unwrap();
    capture(&q, 6, |i| {
        let fd = &bufs[i as usize][0];
        assert!(with_dmabuf_bytes(fd, sizes[0] as usize, not_all_zero));
        enqueue(i);
    });
    q.free().unwrap();
}

#[test]
fn dmabuf_heap_capture_single_planar() {
    dmabuf_heap_capture(false);
}

#[test]
fn dmabuf_heap_capture_multi_planar() {
    dmabuf_heap_capture(true);
}

fn userptr_capture(mplane: bool) {
    let Some((dev, buf_type)) = vivid(mplane) else {
        return;
    };
    let sizes = plane_sizes(&dev, buf_type);
    let mut q = Queue::new(&dev, buf_type).unwrap();
    let granted = match q.request(Memory::UserPtr, 4) {
        Ok(n) => n,
        Err(e) if e.kind() == ErrorKind::InvalidArgument => {
            skip("USERPTR on vivid (load vivid with the vmalloc allocator)");
            return;
        }
        Err(e) => panic!("{e}"),
    };
    assert!(q.capabilities().supports_userptr());

    let page = 4096;
    let layouts: Vec<_> = sizes
        .iter()
        .map(|&s| {
            std::alloc::Layout::from_size_align((s as usize).div_ceil(page) * page, page).unwrap()
        })
        .collect();
    // SAFETY: non-zero, page-aligned layouts; freed at the end of the test
    // after the queue has released every buffer.
    let mem: Vec<Vec<*mut u8>> = (0..granted)
        .map(|_| {
            layouts
                .iter()
                .map(|l| unsafe { std::alloc::alloc_zeroed(*l) })
                .collect()
        })
        .collect();
    let enqueue = |i: u32| {
        let planes: Vec<_> = mem[i as usize]
            .iter()
            .zip(&layouts)
            .map(|(&ptr, l)| UserPtrPlane {
                ptr,
                length: l.size() as u32,
                bytesused: 0,
            })
            .collect();
        // SAFETY: the memory outlives the queue's use of it: the queue is
        // freed before the memory below, and the test does not touch a
        // buffer while it is queued.
        unsafe { q.enqueue_userptr(i, &planes, None) }.unwrap();
    };
    (0..granted).for_each(enqueue);
    q.stream_on().unwrap();
    capture(&q, 6, |i| {
        // SAFETY: the buffer was dequeued, so the device no longer writes it.
        let bytes = unsafe { std::slice::from_raw_parts(mem[i as usize][0], sizes[0] as usize) };
        assert!(not_all_zero(bytes));
        enqueue(i);
    });
    q.free().unwrap();
    for bufs in mem {
        for (ptr, l) in bufs.into_iter().zip(&layouts) {
            // SAFETY: allocated above with the same layout; no longer queued.
            unsafe { std::alloc::dealloc(ptr, *l) };
        }
    }
}

#[test]
fn userptr_capture_single_planar() {
    userptr_capture(false);
}

#[test]
fn userptr_capture_multi_planar() {
    userptr_capture(true);
}

/// Exported buffers carry the captured data, and stay readable after the
/// queue frees them when the driver supports orphaned buffers.
fn export_and_orphan(mplane: bool) {
    let Some((dev, buf_type)) = vivid(mplane) else {
        return;
    };
    let sizes = plane_sizes(&dev, buf_type);
    let mut q = Queue::new(&dev, buf_type).unwrap();
    let granted = q.request(Memory::Mmap, 3).unwrap();
    let exported: Vec<Vec<OwnedFd>> = (0..granted)
        .map(|i| {
            (0..q.num_planes() as u32)
                .map(|p| q.export(i, p).unwrap())
                .collect()
        })
        .collect();
    let maps: Vec<_> = (0..granted).map(|i| q.map(i).unwrap()).collect();
    let planes = vec![Plane::mmap(); q.num_planes()];
    for i in 0..granted {
        q.enqueue(i, &planes, None).unwrap();
    }
    q.stream_on().unwrap();
    let d = next(&q);
    let i = d.index as usize;
    // SAFETY: the buffer was dequeued and is not queued again.
    let mapped = unsafe { maps[i][0].as_slice() }[..sizes[0] as usize].to_vec();
    let via_export = with_dmabuf_bytes(&exported[i][0], sizes[0] as usize, <[u8]>::to_vec);
    assert_eq!(mapped, via_export, "EXPBUF and mmap see the same memory");

    drop(maps);
    let orphans = q.capabilities().supports_orphaned_bufs();
    match q.free() {
        Ok(()) => {
            assert!(orphans, "free succeeded with exports held");
            let after = with_dmabuf_bytes(&exported[i][0], sizes[0] as usize, <[u8]>::to_vec);
            assert_eq!(after, via_export, "an orphaned export keeps its contents");
        }
        Err(e) => {
            assert!(!orphans);
            assert_eq!(e.kind(), ErrorKind::Busy);
        }
    }
}

#[test]
fn export_and_orphan_single_planar() {
    export_and_orphan(false);
}

#[test]
fn export_and_orphan_multi_planar() {
    export_and_orphan(true);
}

#[test]
fn restart_after_stream_off() {
    let Some((dev, buf_type)) = vivid(false) else {
        return;
    };
    let mut q = Queue::new(&dev, buf_type).unwrap();
    let granted = q.request(Memory::Mmap, 3).unwrap();
    for round in 0..2 {
        for i in 0..granted {
            q.enqueue(i, &[Plane::mmap()], None).unwrap();
        }
        q.stream_on().unwrap();
        capture(&q, 3, |i| q.enqueue(i, &[Plane::mmap()], None).unwrap());
        q.stream_off().unwrap();
        assert_eq!(q.queued_count(), 0, "round {round}");
    }
    q.free().unwrap();
}

#[test]
fn create_buffers_extends_the_queue() {
    let Some((dev, buf_type)) = vivid(false) else {
        return;
    };
    let fmt = dev.get_format(buf_type);
    let mut q = Queue::new(&dev, buf_type).unwrap();
    q.request(Memory::Mmap, 2).unwrap();
    let before = q.len() as u32;
    match q.create_buffers(Memory::Mmap, 2, &fmt) {
        Ok(added) => {
            assert_eq!(added.start, before);
            assert_eq!(q.len() as u32, added.end);
            q.query(added.end - 1).unwrap();
        }
        Err(e) => assert!(e.is_unsupported(), "{e}"),
    }
    assert_eq!(
        q.create_buffers(Memory::DmaBuf, 1, &fmt)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidArgument,
        "a queue cannot mix memory types"
    );
    q.free().unwrap();
}

#[test]
fn queue_state_errors() {
    let Some((dev, buf_type)) = vivid(false) else {
        return;
    };
    let mut q = Queue::new(&dev, buf_type).unwrap();
    let granted = q.request(Memory::Mmap, 2).unwrap();

    assert_eq!(
        q.wait(Some(Duration::from_millis(50))).unwrap_err().kind(),
        ErrorKind::InvalidState,
        "poll reports an error before streaming"
    );
    assert_eq!(
        q.enqueue(granted, &[Plane::mmap()], None)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidArgument,
        "index out of range"
    );
    let fd = File::open("/dev/null").unwrap();
    let wrong = Plane::DmaBuf {
        fd: fd.as_fd(),
        length: 0,
        bytesused: 0,
        data_offset: 0,
    };
    assert_eq!(
        q.enqueue(0, &[wrong], None).unwrap_err().kind(),
        ErrorKind::InvalidArgument,
        "plane memory type must match the queue"
    );
    q.enqueue(0, &[Plane::mmap()], None).unwrap();
    assert_eq!(
        q.enqueue(0, &[Plane::mmap()], None).unwrap_err().kind(),
        ErrorKind::InvalidState,
        "an index cannot be queued twice"
    );

    let other = Device::open(&dev.path).unwrap();
    let mut q2 = Queue::new(&other, buf_type).unwrap();
    assert_eq!(
        q2.request(Memory::Mmap, 2).unwrap_err().kind(),
        ErrorKind::Busy,
        "another file handle owns the queue"
    );

    assert_eq!(
        q.dequeue().unwrap_err().kind(),
        ErrorKind::InvalidState,
        "nothing can be dequeued before streaming"
    );
    q.stream_on().unwrap();
    // Once streaming, dequeue succeeds whether or not a frame has arrived.
    q.dequeue().unwrap();
    q.free().unwrap();
}

#[test]
fn m2m_round_trip_on_vim2m() {
    let dev = match find("vim2m", V4L2_CAP_VIDEO_M2M) {
        Ok(d) => d,
        Err(why) => {
            skip_because("vim2m", &why);
            return;
        }
    };
    let (w, h) = (320u32, 240u32);
    for t in [V4L2_BUF_TYPE_VIDEO_OUTPUT, V4L2_BUF_TYPE_VIDEO_CAPTURE] {
        let mut fmt = v4l2_format {
            type_: t,
            ..Default::default()
        };
        // SAFETY: the type selects the single-planar payload.
        let pix = unsafe { fmt.pix() };
        pix.width = w;
        pix.height = h;
        pix.pixelformat = V4L2_PIX_FMT_RGB24;
        pix.field = V4L2_FIELD_NONE;
        dev.set_format(&mut fmt);
    }
    let size = plane_sizes(&dev, BufType::VideoOutput)[0];

    let mut m2m = M2m::new(&dev, false).unwrap();
    let n_out = m2m.output_mut().request(Memory::Mmap, 2).unwrap();
    let n_cap = m2m.capture_mut().request(Memory::Mmap, 2).unwrap();
    let out_maps: Vec<_> = (0..n_out).map(|i| m2m.output().map(i).unwrap()).collect();
    let cap_maps: Vec<_> = (0..n_cap).map(|i| m2m.capture().map(i).unwrap()).collect();

    let pattern: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
    // SAFETY: the output buffer is not queued yet.
    let staging = unsafe { out_maps[0][0].as_mut_slice() };
    staging[..size as usize].copy_from_slice(&pattern);
    for i in 0..n_cap {
        m2m.capture().enqueue(i, &[Plane::mmap()], None).unwrap();
    }
    let stamp = Duration::new(1234, 567_000);
    m2m.output()
        .enqueue(0, &[Plane::Mmap { bytesused: size }], Some(stamp))
        .unwrap();
    m2m.stream_on().unwrap();

    let mut produced = None;
    let mut consumed = false;
    while produced.is_none() || !consumed {
        let ready = m2m.wait(Some(FRAME_TIMEOUT)).unwrap();
        assert!(
            !ready.is_empty(),
            "vim2m produced nothing within {FRAME_TIMEOUT:?}"
        );
        if ready.capture {
            if let Some(d) = m2m.capture().dequeue().unwrap() {
                produced = Some(d);
            }
        }
        if ready.output && m2m.output().dequeue().unwrap().is_some() {
            consumed = true;
        }
    }
    let d = produced.unwrap();
    assert_eq!(d.flags.timestamp_clock(), TimestampClock::Copy);
    assert_eq!(
        d.timestamp, stamp,
        "the output timestamp is copied to the capture buffer"
    );
    assert_eq!(d.planes()[0].bytesused, size);
    // SAFETY: the capture buffer was dequeued.
    let got = unsafe { cap_maps[d.index as usize][0].as_slice() };
    assert_eq!(
        &got[..size as usize],
        &pattern[..],
        "RGB24 to RGB24 is a copy"
    );

    m2m.stream_off().unwrap();
    assert_eq!(m2m.capture().queued_count(), 0);
    drop((out_maps, cap_maps));
    let (mut out, mut cap) = m2m.into_queues();
    out.free().unwrap();
    cap.free().unwrap();
}

#[test]
fn request_switches_memory_type() {
    let Some((dev, buf_type)) = vivid(false) else {
        return;
    };
    let mut q = Queue::new(&dev, buf_type).unwrap();
    q.request(Memory::Mmap, 2).unwrap();
    assert_eq!(q.memory(), Some(Memory::Mmap));
    let n = q.request(Memory::DmaBuf, 3).unwrap();
    assert!(n >= 2);
    assert_eq!(q.memory(), Some(Memory::DmaBuf));
    assert_eq!(
        q.request(Memory::Mmap, 0).unwrap(),
        0,
        "a zero count frees whatever memory type is allocated"
    );
    assert_eq!(q.memory(), None);
    assert!(q.is_empty());
}

#[test]
fn dequeue_never_blocks_on_a_blocking_descriptor() {
    let Some((locked, buf_type)) = vivid(false) else {
        return;
    };
    let dev = Device::open_blocking(&locked.path).unwrap();
    drop(locked);
    dev.lock();
    let mut q = Queue::new(&dev, buf_type).unwrap();
    let granted = q.request(Memory::Mmap, 2).unwrap();
    for i in 0..granted {
        q.enqueue(i, &[Plane::mmap()], None).unwrap();
    }
    q.stream_on().unwrap();
    let t = std::time::Instant::now();
    let _ = q.dequeue().unwrap();
    assert!(
        t.elapsed() < Duration::from_millis(20),
        "dequeue blocked for {:?}",
        t.elapsed()
    );
    next(&q);
    q.free().unwrap();
}

/// One thread stops and restarts the stream and requeues everything while
/// another dequeues and requeues frames. The kernel must never see an index
/// queued twice, and the queue's record must match the kernel's afterwards.
#[test]
fn concurrent_stream_control_keeps_state_consistent() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let Some((dev, buf_type)) = vivid(false) else {
        return;
    };
    let mut q = Queue::new(&dev, buf_type).unwrap();
    let granted = q.request(Memory::Mmap, 4).unwrap();
    for i in 0..granted {
        q.enqueue(i, &[Plane::mmap()], None).unwrap();
    }
    q.stream_on().unwrap();

    // Only state errors are expected from racing the two threads; anything
    // else (in particular EINVAL from a double QBUF) is a bug.
    let tolerate = |r: edgefirst_v4l2::Result<()>| {
        if let Err(e) = r {
            assert_eq!(e.kind(), ErrorKind::InvalidState, "{e}");
        }
    };
    let done = AtomicBool::new(false);
    let frames = std::thread::scope(|s| {
        let consumer = s.spawn(|| {
            let mut frames = 0;
            while !done.load(Ordering::Acquire) {
                match q.wait(Some(Duration::from_millis(20))) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(e) => {
                        assert_eq!(e.kind(), ErrorKind::InvalidState, "{e}");
                        std::thread::yield_now();
                        continue;
                    }
                }
                match q.dequeue() {
                    Ok(Some(d)) => {
                        frames += 1;
                        tolerate(q.enqueue(d.index, &[Plane::mmap()], None));
                    }
                    Ok(None) => {}
                    Err(e) => assert_eq!(e.kind(), ErrorKind::InvalidState, "{e}"),
                }
            }
            frames
        });
        for _ in 0..15 {
            std::thread::sleep(Duration::from_millis(40));
            q.stream_off().unwrap();
            for i in 0..granted {
                if !q.is_queued(i) {
                    tolerate(q.enqueue(i, &[Plane::mmap()], None));
                }
            }
            q.stream_on().unwrap();
        }
        done.store(true, Ordering::Release);
        consumer.join().unwrap()
    });
    assert!(frames > 0, "no frames were captured while toggling");

    for i in 0..granted {
        let kernel =
            q.query(i).unwrap().flags.raw() & (V4L2_BUF_FLAG_QUEUED | V4L2_BUF_FLAG_DONE) != 0;
        assert_eq!(
            q.is_queued(i),
            kernel,
            "index {i}: queue record disagrees with the kernel"
        );
    }
    q.free().unwrap();
}

mod device_controls_events {
    use super::*;
    use edgefirst_v4l2::controls::{self, ControlInfo, ControlType, ControlValue};
    use edgefirst_v4l2::device::{self, Fraction, FrameIntervals, FrameSizes};
    use edgefirst_v4l2::events;

    /// The crate's `Device` on the node the test holds locked.
    fn open(locked: &Device) -> device::Device {
        device::Device::open(&locked.path).unwrap()
    }

    #[test]
    fn enumerate_finds_vivid_in_numeric_order() {
        let Some((locked, _)) = vivid(false) else {
            return;
        };
        let found = device::enumerate().unwrap();
        let numbers: Vec<u32> = found
            .iter()
            .map(|d| {
                d.path.file_name().unwrap().to_str().unwrap()["video".len()..]
                    .parse()
                    .unwrap()
            })
            .collect();
        assert!(numbers.windows(2).all(|w| w[0] < w[1]), "{numbers:?}");
        let ours = found
            .iter()
            .find(|d| d.path.to_str() == Some(locked.path.as_str()))
            .expect("the locked vivid node is listed");
        let caps = ours.capabilities.as_ref().unwrap();
        assert_eq!(caps.driver, "vivid");
        assert!(caps.is_capture() && caps.has_streaming() && !caps.is_m2m());
        assert_eq!(caps.capture_buf_type(), Some(BufType::VideoCapture));
    }

    #[test]
    fn multi_planar_node_reports_mplane_type() {
        let Some((locked, buf_type)) = vivid(true) else {
            return;
        };
        let dev = open(&locked);
        assert_eq!(dev.capabilities().capture_buf_type(), Some(buf_type));
        assert!(!dev.formats(buf_type).unwrap().is_empty());
    }

    #[test]
    fn formats_sizes_and_intervals() {
        let Some((locked, buf_type)) = vivid(false) else {
            return;
        };
        let dev = open(&locked);
        let formats = dev.formats(buf_type).unwrap();
        assert!(
            formats
                .iter()
                .any(|f| f.fourcc == V4L2_PIX_FMT_YUYV && !f.is_compressed()),
            "{formats:?}"
        );
        let size = match dev.frame_sizes(V4L2_PIX_FMT_YUYV).unwrap() {
            FrameSizes::Discrete(sizes) => {
                assert!(!sizes.is_empty());
                sizes[0]
            }
            FrameSizes::Stepwise(r) | FrameSizes::Continuous(r) => {
                assert!(r.min.width <= r.max.width && r.min.height <= r.max.height);
                r.min
            }
        };
        match dev.frame_intervals(V4L2_PIX_FMT_YUYV, size).unwrap() {
            FrameIntervals::Discrete(list) => {
                assert!(!list.is_empty());
                assert!(list.iter().all(|f| f.fps().is_some()));
            }
            FrameIntervals::Stepwise { min, max, .. } | FrameIntervals::Continuous { min, max } => {
                assert!(min.fps() >= max.fps());
            }
        }
    }

    #[test]
    fn enumerating_an_unoffered_format_is_unsupported() {
        let Some((locked, _)) = vivid(false) else {
            return;
        };
        let dev = open(&locked);
        let h264 = fourcc(b'H', b'2', b'6', b'4');
        let e = dev.frame_sizes(h264).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::Unsupported, "{e}");
        assert_eq!(e.errno(), Some(nix::errno::Errno::EINVAL));
        let odd = device::Size {
            width: 1234,
            height: 567,
        };
        let e = dev.frame_intervals(V4L2_PIX_FMT_YUYV, odd).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::Unsupported, "{e}");
    }

    #[test]
    fn set_format_reports_what_the_driver_applied() {
        let Some((locked, buf_type)) = vivid(false) else {
            return;
        };
        let dev = open(&locked);
        let mut want = dev.format(buf_type).unwrap();
        // SAFETY: the type selects the single-planar payload.
        let pix = unsafe { want.pix() };
        pix.width = 641;
        pix.height = 479;
        pix.pixelformat = V4L2_PIX_FMT_YUYV;
        let mut tried = want;
        dev.try_format(&mut tried).unwrap();
        // SAFETY: as above.
        let t = unsafe { tried.pix() };
        assert_eq!(t.pixelformat, V4L2_PIX_FMT_YUYV);
        assert!(t.bytesperline >= t.width * 2);
        dev.set_format(&mut want).unwrap();
        let mut now = dev.format(buf_type).unwrap();
        // SAFETY: as above.
        let (w, n) = (unsafe { *want.pix() }, unsafe { *now.pix() });
        assert_eq!(
            (w.width, w.height, w.sizeimage),
            (n.width, n.height, n.sizeimage)
        );
    }

    #[test]
    fn frame_interval_round_trip() {
        let Some((locked, buf_type)) = vivid(false) else {
            return;
        };
        let dev = open(&locked);
        let Some(current) = dev.frame_interval(buf_type).unwrap() else {
            panic!("vivid supports V4L2_CAP_TIMEPERFRAME");
        };
        assert!(current.fps().is_some());
        let applied = dev
            .set_frame_interval(buf_type, Fraction::new(1, 15))
            .unwrap();
        assert_eq!(dev.frame_interval(buf_type).unwrap(), Some(applied));
        dev.set_frame_interval(buf_type, current).unwrap();
    }

    #[test]
    fn selection_where_supported() {
        let Some((locked, buf_type)) = vivid(false) else {
            return;
        };
        let dev = open(&locked);
        // Which targets a driver supports depends on the input; an
        // unsupported one is reported as `Unsupported`, never as an I/O
        // error.
        let get = |target| match dev.selection(buf_type, target) {
            Ok(r) => Some(r),
            Err(e) => {
                assert!(
                    matches!(
                        e.kind(),
                        ErrorKind::Unsupported | ErrorKind::InvalidArgument
                    ),
                    "{e}"
                );
                eprintln!("NOTE: selection target {target:#x} not supported here ({e})");
                None
            }
        };
        if let Some(bounds) = get(V4L2_SEL_TGT_CROP_BOUNDS) {
            assert!(bounds.width > 0 && bounds.height > 0);
        }
        if let Some(crop) = get(V4L2_SEL_TGT_CROP) {
            let applied = dev
                .set_selection(buf_type, V4L2_SEL_TGT_CROP, crop, 0)
                .unwrap();
            assert_eq!(applied, crop);
        }
    }

    fn writable(c: &ControlInfo) -> bool {
        !(c.flags.is_read_only() || c.flags.is_disabled() || c.flags.is_write_only())
    }

    fn first(all: &[ControlInfo], f: impl Fn(&ControlInfo) -> bool) -> &ControlInfo {
        all.iter()
            .find(|c| f(c))
            .expect("vivid exposes this kind of control")
    }

    #[test]
    fn controls_enumerate_and_round_trip() {
        let Some((locked, _)) = vivid(false) else {
            return;
        };
        let dev = open(&locked);
        let all = controls::query_all(&dev).unwrap();
        assert!(all.windows(2).all(|w| w[0].id != w[1].id));
        assert!(all.iter().any(|c| c.control_type == ControlType::CtrlClass));

        // A standard integer control: set, clamp to the range, query by ID.
        let b = first(&all, |c| c.id == V4L2_CID_BRIGHTNESS);
        assert_eq!(&controls::query(&dev, b.id).unwrap(), b);
        let mid = ((b.minimum + b.maximum) / 2) as i32;
        assert_eq!(
            controls::set(&dev, b, &ControlValue::Integer(mid)).unwrap(),
            ControlValue::Integer(mid)
        );
        assert_eq!(controls::get(&dev, b).unwrap(), ControlValue::Integer(mid));
        let over = controls::set(&dev, b, &ControlValue::Integer(b.maximum as i32 + 1000)).unwrap();
        assert_eq!(
            over,
            ControlValue::Integer(b.maximum as i32),
            "clamped to the maximum"
        );

        let i64c = first(&all, |c| {
            c.control_type == ControlType::Integer64 && writable(c)
        });
        let v = i64c.minimum.max(-(1 << 40));
        assert_eq!(
            controls::set(&dev, i64c, &ControlValue::Integer64(v)).unwrap(),
            ControlValue::Integer64(v)
        );

        let s = first(&all, |c| {
            c.control_type == ControlType::String && writable(c)
        });
        let text: String = "edgefirst".chars().take(s.maximum as usize).collect();
        controls::set(&dev, s, &ControlValue::String(text.clone())).unwrap();
        assert_eq!(controls::get(&dev, s).unwrap(), ControlValue::String(text));

        let menu = first(&all, |c| c.control_type == ControlType::Menu && writable(c));
        assert!(!menu.menu.is_empty());
        assert!(menu
            .menu
            .iter()
            .all(|m| m.name.is_some() && m.value.is_none()));
        let item = menu.menu.last().unwrap().index as i32;
        assert_eq!(
            controls::set(&dev, menu, &ControlValue::Integer(item)).unwrap(),
            ControlValue::Integer(item)
        );

        let int_menu = first(&all, |c| c.control_type == ControlType::IntegerMenu);
        assert!(!int_menu.menu.is_empty());
        assert!(int_menu.menu.iter().all(|m| m.value.is_some()));

        let array = first(&all, |c| {
            c.control_type == ControlType::U8 && c.flags.has_payload() && c.elems > 1 && writable(c)
        });
        let n = (array.elem_size * array.elems) as usize;
        let lo = array.minimum.max(0) as u8;
        let pattern: Vec<u8> = (0..n).map(|i| lo.saturating_add((i % 7) as u8)).collect();
        controls::set(&dev, array, &ControlValue::Payload(pattern.clone())).unwrap();
        assert_eq!(
            controls::get(&dev, array).unwrap(),
            ControlValue::Payload(pattern)
        );

        match all
            .iter()
            .find(|c| c.flags.is_dynamic_array() && writable(c))
        {
            Some(dynamic) => {
                assert_eq!(dynamic.control_type, ControlType::U32);
                // Two in-range elements, each a native-endian u32.
                let two: Vec<u8> = [dynamic.minimum, dynamic.maximum]
                    .iter()
                    .flat_map(|&v| (v as u32).to_ne_bytes())
                    .collect();
                assert!(dynamic.dims.first().is_some_and(|&max| max >= 2));
                controls::set(&dev, dynamic, &ControlValue::Payload(two.clone())).unwrap();
                assert_eq!(
                    controls::get(&dev, dynamic).unwrap(),
                    ControlValue::Payload(two),
                    "get returns the current length, not the capacity"
                );
            }
            None => eprintln!("NOTE: this vivid has no dynamic-array control"),
        }

        let button = first(&all, |c| c.control_type == ControlType::Button);
        assert_eq!(
            controls::get(&dev, button).unwrap_err().kind(),
            ErrorKind::PermissionDenied,
            "a button is write-only"
        );
        let ro = first(&all, |c| {
            c.flags.is_read_only() && c.control_type == ControlType::Integer
        });
        assert_eq!(
            controls::set(&dev, ro, &ControlValue::Integer(ro.minimum as i32))
                .unwrap_err()
                .kind(),
            ErrorKind::PermissionDenied
        );

        controls::set(&dev, b, &ControlValue::Integer(b.default_value as i32)).unwrap();
    }

    #[test]
    fn control_events_are_delivered_and_drained() {
        let Some((locked, _)) = vivid(false) else {
            return;
        };
        let dev = open(&locked);
        let b = controls::query(&dev, V4L2_CID_BRIGHTNESS).unwrap();
        let start = ((b.minimum + b.maximum) / 2) as i32;
        controls::set(&dev, &b, &ControlValue::Integer(start)).unwrap();

        events::subscribe(&dev, V4L2_EVENT_CTRL, b.id, V4L2_EVENT_SUB_FL_SEND_INITIAL).unwrap();
        let initial = events::dequeue(&dev)
            .unwrap()
            .expect("SEND_INITIAL queues an event");
        assert_eq!(initial.kind(), V4L2_EVENT_CTRL);
        assert_eq!(initial.id(), b.id);
        assert_eq!(initial.control().unwrap().value, i64::from(start));
        assert!(
            events::dequeue(&dev).unwrap().is_none(),
            "nothing else pending"
        );

        assert!(
            !events::wait(&dev, Some(Duration::from_millis(50))).unwrap(),
            "no event pending yet"
        );

        // A second handle changes the control, as another process would.
        let other = open(&locked);
        for v in [start + 1, start + 2] {
            controls::set(&other, &b, &ControlValue::Integer(v)).unwrap();
        }
        assert!(events::wait(&dev, Some(Duration::from_secs(1))).unwrap());
        let drained = events::drain(&dev, 16).unwrap();
        assert!(!drained.is_empty());
        let last = drained.last().unwrap().control().unwrap();
        assert_ne!(last.changes & V4L2_EVENT_CTRL_CH_VALUE, 0);
        assert_eq!(last.value, i64::from(start + 2));
        assert_eq!(drained.last().unwrap().pending(), 0);
        assert!(drained.iter().all(|e| e.timestamp() > Duration::ZERO));

        events::unsubscribe(&dev, V4L2_EVENT_ALL, 0).unwrap();
        controls::set(&other, &b, &ControlValue::Integer(start)).unwrap();
        assert!(
            events::dequeue(&dev).unwrap().is_none(),
            "no events after unsubscribing"
        );
        controls::set(&dev, &b, &ControlValue::Integer(b.default_value as i32)).unwrap();
    }

    #[test]
    fn queue_accepts_the_crate_device() {
        let Some((locked, buf_type)) = vivid(false) else {
            return;
        };
        let dev = open(&locked);
        let mut q = Queue::new(&dev, buf_type).unwrap();
        let granted = q.request(Memory::Mmap, 2).unwrap();
        for i in 0..granted {
            q.enqueue(i, &[Plane::mmap()], None).unwrap();
        }
        q.stream_on().unwrap();
        next(&q);
        q.free().unwrap();
    }
}
