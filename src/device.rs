// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! Opening and describing V4L2 video nodes: enumeration of `/dev/video*`,
//! capabilities, formats, frame sizes and intervals, frame rate and
//! selection.
//!
//! Nothing here changes a device's state except the explicit setters
//! ([`Device::set_format`], [`Device::set_frame_interval`],
//! [`Device::set_selection`]). Enumeration opens each node, queries its
//! capabilities and closes it again, so it is safe to run while other
//! processes stream from those nodes.

use std::fs::{File, OpenOptions};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use nix::errno::Errno;

use crate::error::{retry, Error, ErrorKind, Result};
use crate::ioctl;
use crate::queue::BufType;
#[allow(clippy::wildcard_imports)]
use crate::uapi::*;

/// What a node can do, from `VIDIOC_QUERYCAP`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    /// Driver name, such as `uvcvideo` or `vivid`.
    pub driver: String,
    /// Device name.
    pub card: String,
    /// Location of the device, such as `usb-0000:00:14.0-1`.
    pub bus_info: String,
    /// Driver version (`KERNEL_VERSION` encoding).
    pub version: u32,
    /// Capabilities of the whole physical device.
    pub capabilities: u32,
    /// Capabilities of this node, valid when `capabilities` has
    /// `V4L2_CAP_DEVICE_CAPS`.
    pub device_caps: u32,
}

impl Capabilities {
    fn from_raw(c: &v4l2_capability) -> Self {
        Self {
            driver: c_string(&c.driver),
            card: c_string(&c.card),
            bus_info: c_string(&c.bus_info),
            version: c.version,
            capabilities: c.capabilities,
            device_caps: c.device_caps,
        }
    }

    /// The capabilities of this node: `device_caps` when the driver reports
    /// them, otherwise the device-wide `capabilities`. A device with several
    /// nodes lists every node's abilities in `capabilities`, so only this
    /// value says what the opened node does.
    pub fn effective(&self) -> u32 {
        if self.capabilities & V4L2_CAP_DEVICE_CAPS != 0 {
            self.device_caps
        } else {
            self.capabilities
        }
    }

    /// The node captures video (single- or multi-planar, not M2M).
    pub fn is_capture(&self) -> bool {
        self.effective() & (V4L2_CAP_VIDEO_CAPTURE | V4L2_CAP_VIDEO_CAPTURE_MPLANE) != 0
    }

    /// The node outputs video (single- or multi-planar, not M2M).
    pub fn is_output(&self) -> bool {
        self.effective() & (V4L2_CAP_VIDEO_OUTPUT | V4L2_CAP_VIDEO_OUTPUT_MPLANE) != 0
    }

    /// The node is a memory-to-memory device such as a codec.
    pub fn is_m2m(&self) -> bool {
        self.effective() & (V4L2_CAP_VIDEO_M2M | V4L2_CAP_VIDEO_M2M_MPLANE) != 0
    }

    /// The node supports streaming I/O (buffer queues).
    pub fn has_streaming(&self) -> bool {
        self.effective() & V4L2_CAP_STREAMING != 0
    }

    /// The buffer type for captured frames, if the node produces any:
    /// capture nodes and the capture side of M2M nodes. Multi-planar is
    /// preferred when the node supports both.
    pub fn capture_buf_type(&self) -> Option<BufType> {
        let c = self.effective();
        if c & (V4L2_CAP_VIDEO_CAPTURE_MPLANE | V4L2_CAP_VIDEO_M2M_MPLANE) != 0 {
            Some(BufType::VideoCaptureMplane)
        } else if c & (V4L2_CAP_VIDEO_CAPTURE | V4L2_CAP_VIDEO_M2M) != 0 {
            Some(BufType::VideoCapture)
        } else {
            None
        }
    }

    /// The buffer type for frames the application sends, if the node
    /// accepts any: output nodes and the output side of M2M nodes.
    pub fn output_buf_type(&self) -> Option<BufType> {
        let c = self.effective();
        if c & (V4L2_CAP_VIDEO_OUTPUT_MPLANE | V4L2_CAP_VIDEO_M2M_MPLANE) != 0 {
            Some(BufType::VideoOutputMplane)
        } else if c & (V4L2_CAP_VIDEO_OUTPUT | V4L2_CAP_VIDEO_M2M) != 0 {
            Some(BufType::VideoOutput)
        } else {
            None
        }
    }
}

/// A pixel format offered by `VIDIOC_ENUM_FMT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatDesc {
    /// FourCC code (`V4L2_PIX_FMT_*`).
    pub fourcc: u32,
    /// Driver's description of the format.
    pub description: String,
    /// `V4L2_FMT_FLAG_*` bits.
    pub flags: u32,
}

impl FormatDesc {
    /// A compressed format such as MJPEG or H.264.
    pub fn is_compressed(&self) -> bool {
        self.flags & V4L2_FMT_FLAG_COMPRESSED != 0
    }

    /// A format the driver converts to in software (libv4l-style emulation).
    pub fn is_emulated(&self) -> bool {
        self.flags & V4L2_FMT_FLAG_EMULATED != 0
    }
}

/// A frame size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// A range of frame sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeRange {
    /// Smallest size.
    pub min: Size,
    /// Largest size.
    pub max: Size,
    /// Width and height increments (1 for continuous ranges).
    pub step: Size,
}

/// The frame sizes a format supports, from `VIDIOC_ENUM_FRAMESIZES`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameSizes {
    /// A list of sizes.
    Discrete(Vec<Size>),
    /// Any size in the range, in steps.
    Stepwise(SizeRange),
    /// Any size in the range.
    Continuous(SizeRange),
}

/// A time per frame in seconds, `numerator / denominator`. The frame rate is
/// the reciprocal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fraction {
    /// Numerator.
    pub numerator: u32,
    /// Denominator.
    pub denominator: u32,
}

impl Fraction {
    /// `numerator / denominator`.
    pub fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// Frames per second for this frame interval, or `None` when the
    /// numerator is zero.
    pub fn fps(self) -> Option<f64> {
        (self.numerator != 0).then(|| f64::from(self.denominator) / f64::from(self.numerator))
    }

    fn from_raw(f: v4l2_fract) -> Self {
        Self::new(f.numerator, f.denominator)
    }

    fn to_raw(self) -> v4l2_fract {
        v4l2_fract {
            numerator: self.numerator,
            denominator: self.denominator,
        }
    }
}

/// The frame intervals a format and size support, from
/// `VIDIOC_ENUM_FRAMEINTERVALS`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameIntervals {
    /// A list of intervals.
    Discrete(Vec<Fraction>),
    /// Any interval in the range, in steps.
    Stepwise {
        /// Shortest interval (highest frame rate).
        min: Fraction,
        /// Longest interval.
        max: Fraction,
        /// Increment.
        step: Fraction,
    },
    /// Any interval in the range.
    Continuous {
        /// Shortest interval (highest frame rate).
        min: Fraction,
        /// Longest interval.
        max: Fraction,
    },
}

/// A rectangle for crop and compose selections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl Rect {
    fn from_raw(r: v4l2_rect) -> Self {
        Self {
            left: r.left,
            top: r.top,
            width: r.width,
            height: r.height,
        }
    }

    fn to_raw(self) -> v4l2_rect {
        v4l2_rect {
            left: self.left,
            top: self.top,
            width: self.width,
            height: self.height,
        }
    }
}

/// A video node found by [`enumerate`].
#[derive(Debug)]
pub struct DeviceInfo {
    /// Path of the node, such as `/dev/video0`.
    pub path: PathBuf,
    /// Its capabilities, or why it could not be opened or queried (for
    /// example no permission).
    pub capabilities: Result<Capabilities>,
}

/// Lists `/dev/video*` in numeric order (`video2` before `video10`) with
/// each node's capabilities.
pub fn enumerate() -> Result<Vec<DeviceInfo>> {
    enumerate_in(Path::new("/dev"))
}

fn enumerate_in(dir: &Path) -> Result<Vec<DeviceInfo>> {
    let entries = std::fs::read_dir(dir).map_err(|e| io_error("read_dir", &e))?;
    let mut nodes: Vec<(u32, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let n = name.strip_prefix("video")?.parse().ok()?;
            Some((n, e.path()))
        })
        .collect();
    nodes.sort_unstable_by_key(|(n, _)| *n);
    Ok(nodes
        .into_iter()
        .map(|(_, path)| {
            let capabilities = Device::open(&path).map(|d| d.capabilities().clone());
            DeviceInfo { path, capabilities }
        })
        .collect())
}

/// An open video node.
///
/// Opened read-write, non-blocking and close-on-exec. Pass it to
/// [`Queue::new`](crate::queue::Queue::new) or
/// [`M2m::new`](crate::m2m::M2m::new) to stream, and to the
/// [`controls`](crate::controls) and [`events`](crate::events) functions.
#[derive(Debug)]
pub struct Device {
    file: File,
    path: PathBuf,
    caps: Capabilities,
}

impl Device {
    /// Opens `path` and queries its capabilities.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|e| io_error("open", &e))?;
        let mut cap = v4l2_capability::default();
        // SAFETY: valid fd; `cap` outlives the call.
        retry("VIDIOC_QUERYCAP", || unsafe {
            ioctl::vidioc_querycap(file.as_raw_fd(), &mut cap)
        })?;
        Ok(Self {
            file,
            path,
            caps: Capabilities::from_raw(&cap),
        })
    }

    /// The node's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The capabilities queried at open.
    pub fn capabilities(&self) -> &Capabilities {
        &self.caps
    }

    /// The pixel formats offered for `buf_type`, in driver order.
    pub fn formats(&self, buf_type: BufType) -> Result<Vec<FormatDesc>> {
        let mut out = Vec::new();
        for index in 0.. {
            let mut d = v4l2_fmtdesc {
                index,
                type_: buf_type.raw(),
                ..Default::default()
            };
            // SAFETY: valid fd; `d` outlives the call.
            match retry("VIDIOC_ENUM_FMT", || unsafe {
                ioctl::vidioc_enum_fmt(self.raw(), &mut d)
            }) {
                Ok(_) => out.push(FormatDesc {
                    fourcc: d.pixelformat,
                    description: c_string(&d.description),
                    flags: d.flags,
                }),
                Err(e) if end_of_list(&e) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }

    /// The frame sizes supported for `fourcc`. Fails with
    /// [`ErrorKind::Unsupported`] when the driver does not enumerate sizes
    /// for `fourcc` (including a format it does not offer).
    pub fn frame_sizes(&self, fourcc: u32) -> Result<FrameSizes> {
        let mut sizes = Vec::new();
        for index in 0.. {
            let mut e = v4l2_frmsizeenum {
                index,
                pixel_format: fourcc,
                ..Default::default()
            };
            // SAFETY: valid fd; `e` outlives the call.
            match retry("VIDIOC_ENUM_FRAMESIZES", || unsafe {
                ioctl::vidioc_enum_framesizes(self.raw(), &mut e)
            }) {
                Ok(_) => {}
                Err(err) if end_of_list(&err) && index > 0 => break,
                Err(err) if end_of_list(&err) => return Err(err.with_kind(ErrorKind::Unsupported)),
                Err(err) => return Err(err),
            }
            match e.type_ {
                V4L2_FRMSIZE_TYPE_DISCRETE => {
                    let d = e.discrete();
                    sizes.push(Size {
                        width: d.width,
                        height: d.height,
                    });
                }
                t => {
                    let s = e.stepwise();
                    let range = SizeRange {
                        min: Size {
                            width: s.min_width,
                            height: s.min_height,
                        },
                        max: Size {
                            width: s.max_width,
                            height: s.max_height,
                        },
                        step: Size {
                            width: s.step_width,
                            height: s.step_height,
                        },
                    };
                    return Ok(if t == V4L2_FRMSIZE_TYPE_CONTINUOUS {
                        FrameSizes::Continuous(range)
                    } else {
                        FrameSizes::Stepwise(range)
                    });
                }
            }
        }
        Ok(FrameSizes::Discrete(sizes))
    }

    /// The frame intervals supported for `fourcc` at `size`. Fails with
    /// [`ErrorKind::Unsupported`] when the driver does not enumerate them
    /// for that format and size (including one it does not offer).
    pub fn frame_intervals(&self, fourcc: u32, size: Size) -> Result<FrameIntervals> {
        let mut list = Vec::new();
        for index in 0.. {
            let mut e = v4l2_frmivalenum {
                index,
                pixel_format: fourcc,
                width: size.width,
                height: size.height,
                ..Default::default()
            };
            // SAFETY: valid fd; `e` outlives the call.
            match retry("VIDIOC_ENUM_FRAMEINTERVALS", || unsafe {
                ioctl::vidioc_enum_frameintervals(self.raw(), &mut e)
            }) {
                Ok(_) => {}
                Err(err) if end_of_list(&err) && index > 0 => break,
                Err(err) if end_of_list(&err) => return Err(err.with_kind(ErrorKind::Unsupported)),
                Err(err) => return Err(err),
            }
            match e.type_ {
                V4L2_FRMIVAL_TYPE_DISCRETE => list.push(Fraction::from_raw(e.discrete())),
                t => {
                    let s = e.stepwise();
                    let (min, max) = (Fraction::from_raw(s.min), Fraction::from_raw(s.max));
                    return Ok(if t == V4L2_FRMIVAL_TYPE_CONTINUOUS {
                        FrameIntervals::Continuous { min, max }
                    } else {
                        FrameIntervals::Stepwise {
                            min,
                            max,
                            step: Fraction::from_raw(s.step),
                        }
                    });
                }
            }
        }
        Ok(FrameIntervals::Discrete(list))
    }

    /// The current format for `buf_type` (`VIDIOC_G_FMT`).
    pub fn format(&self, buf_type: BufType) -> Result<v4l2_format> {
        let mut fmt = v4l2_format {
            type_: buf_type.raw(),
            ..Default::default()
        };
        // SAFETY: valid fd; `fmt` outlives the call.
        retry("VIDIOC_G_FMT", || unsafe {
            ioctl::vidioc_g_fmt(self.raw(), &mut fmt)
        })?;
        Ok(fmt)
    }

    /// Sets the format (`VIDIOC_S_FMT`). The driver adjusts unsupported
    /// values; `fmt` holds what it applied on return.
    pub fn set_format(&self, fmt: &mut v4l2_format) -> Result<()> {
        // SAFETY: valid fd; `fmt` outlives the call.
        retry("VIDIOC_S_FMT", || unsafe {
            ioctl::vidioc_s_fmt(self.raw(), fmt)
        })
        .map(|_| ())
    }

    /// Asks what the driver would apply for `fmt` without changing the
    /// device (`VIDIOC_TRY_FMT`); `fmt` holds the answer on return.
    pub fn try_format(&self, fmt: &mut v4l2_format) -> Result<()> {
        // SAFETY: valid fd; `fmt` outlives the call.
        retry("VIDIOC_TRY_FMT", || unsafe {
            ioctl::vidioc_try_fmt(self.raw(), fmt)
        })
        .map(|_| ())
    }

    /// The current time per frame for a capture or output `buf_type`
    /// (`VIDIOC_G_PARM`), or `None` when the driver does not support frame
    /// rate control (`V4L2_CAP_TIMEPERFRAME` clear).
    pub fn frame_interval(&self, buf_type: BufType) -> Result<Option<Fraction>> {
        let p = self.get_parm(buf_type)?;
        let (cap, tpf) = if buf_type.is_output() {
            let o = p.output();
            (o.capability, o.timeperframe)
        } else {
            let c = p.capture();
            (c.capability, c.timeperframe)
        };
        Ok((cap & V4L2_CAP_TIMEPERFRAME != 0).then(|| Fraction::from_raw(tpf)))
    }

    /// Requests a time per frame (`VIDIOC_S_PARM`) and returns the interval
    /// the driver applied, which may differ. Fails with
    /// [`ErrorKind::Unsupported`] when the driver does not support frame rate
    /// control.
    pub fn set_frame_interval(&self, buf_type: BufType, interval: Fraction) -> Result<Fraction> {
        let mut p = self.get_parm(buf_type)?;
        let applied = if buf_type.is_output() {
            let mut o = p.output();
            if o.capability & V4L2_CAP_TIMEPERFRAME == 0 {
                return Err(Error::new(ErrorKind::Unsupported, "VIDIOC_S_PARM"));
            }
            o.timeperframe = interval.to_raw();
            p.set_output(o);
            self.set_parm(&mut p)?;
            p.output().timeperframe
        } else {
            let mut c = p.capture();
            if c.capability & V4L2_CAP_TIMEPERFRAME == 0 {
                return Err(Error::new(ErrorKind::Unsupported, "VIDIOC_S_PARM"));
            }
            c.timeperframe = interval.to_raw();
            p.set_capture(c);
            self.set_parm(&mut p)?;
            p.capture().timeperframe
        };
        Ok(Fraction::from_raw(applied))
    }

    /// The selection rectangle `target` (`V4L2_SEL_TGT_*`) for `buf_type`
    /// (`VIDIOC_G_SELECTION`).
    pub fn selection(&self, buf_type: BufType, target: u32) -> Result<Rect> {
        let mut sel = v4l2_selection {
            type_: selection_type(buf_type),
            target,
            ..Default::default()
        };
        // SAFETY: valid fd; `sel` outlives the call.
        retry("VIDIOC_G_SELECTION", || unsafe {
            ioctl::vidioc_g_selection(self.raw(), &mut sel)
        })?;
        Ok(Rect::from_raw(sel.r))
    }

    /// Sets the selection rectangle `target` for `buf_type`
    /// (`VIDIOC_S_SELECTION`) with `V4L2_SEL_FLAG_*` `flags`, and returns the
    /// rectangle the driver applied.
    pub fn set_selection(
        &self,
        buf_type: BufType,
        target: u32,
        rect: Rect,
        flags: u32,
    ) -> Result<Rect> {
        let mut sel = v4l2_selection {
            type_: selection_type(buf_type),
            target,
            flags,
            r: rect.to_raw(),
            ..Default::default()
        };
        // SAFETY: valid fd; `sel` outlives the call.
        retry("VIDIOC_S_SELECTION", || unsafe {
            ioctl::vidioc_s_selection(self.raw(), &mut sel)
        })?;
        Ok(Rect::from_raw(sel.r))
    }

    fn get_parm(&self, buf_type: BufType) -> Result<v4l2_streamparm> {
        let mut p = v4l2_streamparm {
            type_: buf_type.raw(),
            ..Default::default()
        };
        // SAFETY: valid fd; `p` outlives the call.
        retry("VIDIOC_G_PARM", || unsafe {
            ioctl::vidioc_g_parm(self.raw(), &mut p)
        })?;
        Ok(p)
    }

    fn set_parm(&self, p: &mut v4l2_streamparm) -> Result<()> {
        // SAFETY: valid fd; `p` outlives the call.
        retry("VIDIOC_S_PARM", || unsafe {
            ioctl::vidioc_s_parm(self.raw(), p)
        })
        .map(|_| ())
    }

    fn raw(&self) -> i32 {
        self.file.as_raw_fd()
    }
}

impl AsFd for Device {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

/// The V4L2 specification asks for the single-planar buffer type in
/// selection calls, and drivers written before multi-planar support in the
/// selection API reject the `_MPLANE` types.
fn selection_type(buf_type: BufType) -> u32 {
    match buf_type {
        BufType::VideoCaptureMplane => V4L2_BUF_TYPE_VIDEO_CAPTURE,
        BufType::VideoOutputMplane => V4L2_BUF_TYPE_VIDEO_OUTPUT,
        t => t.raw(),
    }
}

/// Enumeration ioctls end their list with `EINVAL` at the first index past
/// the end.
fn end_of_list(e: &Error) -> bool {
    e.errno() == Some(Errno::EINVAL)
}

fn io_error(op: &'static str, e: &std::io::Error) -> Error {
    Error::from_errno(op, Errno::from_raw(e.raw_os_error().unwrap_or(0)))
}

/// A NUL-terminated byte array from the kernel as a `String`, replacing
/// invalid UTF-8.
pub(crate) fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(capabilities: u32, device_caps: u32) -> Capabilities {
        Capabilities {
            driver: String::new(),
            card: String::new(),
            bus_info: String::new(),
            version: 0,
            capabilities,
            device_caps,
        }
    }

    #[test]
    fn effective_caps_prefer_device_caps() {
        // A device whose nodes together capture and output, queried on its
        // output node.
        let c = caps(
            V4L2_CAP_DEVICE_CAPS | V4L2_CAP_VIDEO_CAPTURE | V4L2_CAP_VIDEO_OUTPUT,
            V4L2_CAP_VIDEO_OUTPUT | V4L2_CAP_STREAMING,
        );
        assert!(!c.is_capture());
        assert!(c.is_output());
        assert!(c.has_streaming());
        assert_eq!(c.capture_buf_type(), None);
        assert_eq!(c.output_buf_type(), Some(BufType::VideoOutput));

        let legacy = caps(V4L2_CAP_VIDEO_CAPTURE, 0);
        assert!(legacy.is_capture());
    }

    #[test]
    fn m2m_buf_types() {
        let c = caps(
            V4L2_CAP_DEVICE_CAPS,
            V4L2_CAP_VIDEO_M2M_MPLANE | V4L2_CAP_STREAMING,
        );
        assert!(c.is_m2m());
        assert!(!c.is_capture());
        assert_eq!(c.capture_buf_type(), Some(BufType::VideoCaptureMplane));
        assert_eq!(c.output_buf_type(), Some(BufType::VideoOutputMplane));
    }

    #[test]
    fn fraction_fps() {
        assert_eq!(Fraction::new(1, 30).fps(), Some(30.0));
        assert_eq!(
            Fraction::new(1001, 30000)
                .fps()
                .map(|f| (f * 1000.0).round()),
            Some(29970.0)
        );
        assert_eq!(Fraction::new(0, 30).fps(), None);
    }

    #[test]
    fn selection_uses_single_planar_types() {
        assert_eq!(
            selection_type(BufType::VideoCaptureMplane),
            V4L2_BUF_TYPE_VIDEO_CAPTURE
        );
        assert_eq!(
            selection_type(BufType::VideoOutput),
            V4L2_BUF_TYPE_VIDEO_OUTPUT
        );
    }

    #[test]
    fn c_string_stops_at_nul() {
        assert_eq!(c_string(b"vivid\0\0junk"), "vivid");
        assert_eq!(c_string(b"full"), "full");
    }

    #[test]
    fn enumerate_sorts_numerically_and_reports_failures() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-tmp")
            .join(format!("v4l2-enum-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for n in ["video10", "video2", "video0", "videoX", "media0"] {
            std::fs::write(dir.join(n), b"").unwrap();
        }
        let found = enumerate_in(&dir).unwrap();
        let names: Vec<_> = found
            .iter()
            .map(|d| d.path.file_name().unwrap().to_str().unwrap().to_owned())
            .collect();
        assert_eq!(names, ["video0", "video2", "video10"]);
        // Regular files are not V4L2 nodes: QUERYCAP fails.
        assert!(found.iter().all(|d| d.capabilities.is_err()));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
