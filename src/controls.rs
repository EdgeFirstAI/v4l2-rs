// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! Device controls: enumeration with `VIDIOC_QUERY_EXT_CTRL` and
//! `VIDIOC_QUERYMENU`, and typed get and set with `VIDIOC_G/S_EXT_CTRLS`,
//! including 64-bit, string and compound (array) controls.
//!
//! The functions take any open node (`impl AsFd`), usually a
//! [`Device`](crate::device::Device).

use std::os::fd::{AsFd, AsRawFd};

use nix::errno::Errno;

use crate::device::c_string;
use crate::error::{retry, Error, ErrorKind, Result};
use crate::ioctl;
#[allow(clippy::wildcard_imports)]
use crate::uapi::*;

/// The type of a control (`V4L2_CTRL_TYPE_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlType {
    /// 32-bit signed integer.
    Integer,
    /// On or off (0 or 1).
    Boolean,
    /// One of a list of named items.
    Menu,
    /// An action with no value; setting it triggers the action.
    Button,
    /// 64-bit signed integer.
    Integer64,
    /// A heading that groups the controls that follow; it has no value.
    CtrlClass,
    /// A string.
    String,
    /// A 32-bit bit mask.
    Bitmask,
    /// One of a list of 64-bit integer items.
    IntegerMenu,
    /// An array of `u8`.
    U8,
    /// An array of `u16`.
    U16,
    /// An array of `u32`.
    U32,
    /// Any other type, such as a codec-specific compound control.
    Other(u32),
}

impl ControlType {
    /// Converts a `V4L2_CTRL_TYPE_*` value.
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            V4L2_CTRL_TYPE_INTEGER => Self::Integer,
            V4L2_CTRL_TYPE_BOOLEAN => Self::Boolean,
            V4L2_CTRL_TYPE_MENU => Self::Menu,
            V4L2_CTRL_TYPE_BUTTON => Self::Button,
            V4L2_CTRL_TYPE_INTEGER64 => Self::Integer64,
            V4L2_CTRL_TYPE_CTRL_CLASS => Self::CtrlClass,
            V4L2_CTRL_TYPE_STRING => Self::String,
            V4L2_CTRL_TYPE_BITMASK => Self::Bitmask,
            V4L2_CTRL_TYPE_INTEGER_MENU => Self::IntegerMenu,
            V4L2_CTRL_TYPE_U8 => Self::U8,
            V4L2_CTRL_TYPE_U16 => Self::U16,
            V4L2_CTRL_TYPE_U32 => Self::U32,
            other => Self::Other(other),
        }
    }

    /// The `V4L2_CTRL_TYPE_*` value.
    pub fn raw(self) -> u32 {
        match self {
            Self::Integer => V4L2_CTRL_TYPE_INTEGER,
            Self::Boolean => V4L2_CTRL_TYPE_BOOLEAN,
            Self::Menu => V4L2_CTRL_TYPE_MENU,
            Self::Button => V4L2_CTRL_TYPE_BUTTON,
            Self::Integer64 => V4L2_CTRL_TYPE_INTEGER64,
            Self::CtrlClass => V4L2_CTRL_TYPE_CTRL_CLASS,
            Self::String => V4L2_CTRL_TYPE_STRING,
            Self::Bitmask => V4L2_CTRL_TYPE_BITMASK,
            Self::IntegerMenu => V4L2_CTRL_TYPE_INTEGER_MENU,
            Self::U8 => V4L2_CTRL_TYPE_U8,
            Self::U16 => V4L2_CTRL_TYPE_U16,
            Self::U32 => V4L2_CTRL_TYPE_U32,
            Self::Other(raw) => raw,
        }
    }
}

/// `V4L2_CTRL_FLAG_*` bits of a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControlFlags(u32);

impl ControlFlags {
    /// The raw `V4L2_CTRL_FLAG_*` bits.
    pub fn raw(self) -> u32 {
        self.0
    }
    /// The control is disabled and must be ignored.
    pub fn is_disabled(self) -> bool {
        self.0 & V4L2_CTRL_FLAG_DISABLED != 0
    }
    /// Another file handle holds the control (for example while streaming).
    pub fn is_grabbed(self) -> bool {
        self.0 & V4L2_CTRL_FLAG_GRABBED != 0
    }
    /// The control can be read but not set.
    pub fn is_read_only(self) -> bool {
        self.0 & V4L2_CTRL_FLAG_READ_ONLY != 0
    }
    /// The control can be set but not read.
    pub fn is_write_only(self) -> bool {
        self.0 & V4L2_CTRL_FLAG_WRITE_ONLY != 0
    }
    /// The control has no effect in the current configuration (for example
    /// manual exposure while auto exposure is on).
    pub fn is_inactive(self) -> bool {
        self.0 & V4L2_CTRL_FLAG_INACTIVE != 0
    }
    /// The value can change without being set (for example a measured gain).
    pub fn is_volatile(self) -> bool {
        self.0 & V4L2_CTRL_FLAG_VOLATILE != 0
    }
    /// The value is passed by pointer (strings and compound controls).
    pub fn has_payload(self) -> bool {
        self.0 & V4L2_CTRL_FLAG_HAS_PAYLOAD != 0
    }
}

/// One item of a menu control, from `VIDIOC_QUERYMENU`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    /// The item's index, which is the control value that selects it.
    pub index: u32,
    /// The item's name ([`ControlType::Menu`]).
    pub name: Option<String>,
    /// The item's value ([`ControlType::IntegerMenu`]).
    pub value: Option<i64>,
}

/// A control and its range, from `VIDIOC_QUERY_EXT_CTRL`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlInfo {
    /// Control ID (`V4L2_CID_*`).
    pub id: u32,
    /// Driver's name for the control.
    pub name: String,
    /// Value type.
    pub control_type: ControlType,
    /// Smallest value (for strings, the minimum length).
    pub minimum: i64,
    /// Largest value (for strings, the maximum length).
    pub maximum: i64,
    /// Value increment.
    pub step: u64,
    /// Default value.
    pub default_value: i64,
    /// `V4L2_CTRL_FLAG_*` bits.
    pub flags: ControlFlags,
    /// Size of one element in bytes.
    pub elem_size: u32,
    /// Number of elements (1 for scalar controls).
    pub elems: u32,
    /// Array dimensions (empty for scalar controls).
    pub dims: Vec<u32>,
    /// Menu items, for menu and integer-menu controls. Indices the driver
    /// skips are absent.
    pub menu: Vec<MenuItem>,
}

impl ControlInfo {
    fn payload_size(&self) -> usize {
        self.elem_size as usize * self.elems.max(1) as usize
    }
}

/// The value of a control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlValue {
    /// Integer, boolean, menu index, bit mask or button (32-bit).
    Integer(i32),
    /// 64-bit integer.
    Integer64(i64),
    /// String.
    String(String),
    /// Raw bytes of an array or compound control, `elem_size * elems` long.
    Payload(Vec<u8>),
}

/// Lists every control of the device, including control-class headings
/// ([`ControlType::CtrlClass`]) and compound controls, in driver order.
pub fn query_all(dev: impl AsFd) -> Result<Vec<ControlInfo>> {
    let fd = dev.as_fd();
    let mut out = Vec::new();
    let mut id = 0u32;
    loop {
        let next = id | V4L2_CTRL_FLAG_NEXT_CTRL | V4L2_CTRL_FLAG_NEXT_COMPOUND;
        match query_ext(fd, next) {
            Ok(info) => {
                id = info.id;
                out.push(info);
            }
            Err(e) if e.errno() == Some(Errno::EINVAL) => break,
            Err(e) if e.is_unsupported() && out.is_empty() => return query_all_legacy(fd),
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}

/// Describes control `id`.
pub fn query(dev: impl AsFd, id: u32) -> Result<ControlInfo> {
    let fd = dev.as_fd();
    match query_ext(fd, id) {
        Err(e) if e.is_unsupported() => query_legacy(fd, id),
        r => r,
    }
}

/// Reads the current value of the control described by `info`.
pub fn get(dev: impl AsFd, info: &ControlInfo) -> Result<ControlValue> {
    let mut payload = vec![
        0u8;
        if info.flags.has_payload() {
            info.payload_size()
        } else {
            0
        }
    ];
    let mut ctrl = v4l2_ext_control {
        id: info.id,
        ..Default::default()
    };
    if info.flags.has_payload() {
        ctrl.size = payload.len() as u32;
        ctrl.set_ptr(payload.as_mut_ptr().cast());
    }
    ext_ctrls(dev.as_fd(), &mut ctrl, false)?;
    Ok(decode(info, &ctrl, payload))
}

/// Sets the control described by `info` and returns the value the driver
/// applied: drivers clamp integers to the range and round them to the step.
///
/// The value's variant must suit the control type: [`ControlValue::Integer64`]
/// for 64-bit controls, [`ControlValue::String`] for strings,
/// [`ControlValue::Payload`] of exactly `elem_size * elems` bytes for array
/// and compound controls, and [`ControlValue::Integer`] otherwise.
pub fn set(dev: impl AsFd, info: &ControlInfo, value: &ControlValue) -> Result<ControlValue> {
    let invalid = || Error::new(ErrorKind::InvalidArgument, "VIDIOC_S_EXT_CTRLS");
    let mut ctrl = v4l2_ext_control {
        id: info.id,
        ..Default::default()
    };
    let mut payload = Vec::new();
    match (info.control_type, value) {
        (ControlType::Integer64, ControlValue::Integer64(v)) => ctrl.set_value64(*v),
        (ControlType::String, ControlValue::String(s)) => {
            payload = vec![0u8; info.payload_size()];
            if s.len() >= payload.len() {
                return Err(invalid());
            }
            payload[..s.len()].copy_from_slice(s.as_bytes());
        }
        (_, ControlValue::Payload(bytes)) if info.flags.has_payload() => {
            if bytes.len() != info.payload_size() {
                return Err(invalid());
            }
            payload.clone_from(bytes);
        }
        (t, ControlValue::Integer(v))
            if !info.flags.has_payload() && t != ControlType::Integer64 =>
        {
            ctrl.set_value(*v);
        }
        _ => return Err(invalid()),
    }
    if info.flags.has_payload() {
        ctrl.size = payload.len() as u32;
        ctrl.set_ptr(payload.as_mut_ptr().cast());
    }
    ext_ctrls(dev.as_fd(), &mut ctrl, true)?;
    Ok(decode(info, &ctrl, payload))
}

fn ext_ctrls(
    fd: std::os::fd::BorrowedFd<'_>,
    ctrl: &mut v4l2_ext_control,
    write: bool,
) -> Result<()> {
    let mut ctrls = v4l2_ext_controls {
        which: V4L2_CTRL_WHICH_CUR_VAL,
        count: 1,
        controls: ctrl,
        ..Default::default()
    };
    if write {
        // SAFETY: valid fd; `ctrls` points at one control (and its payload)
        // that outlive the call.
        retry("VIDIOC_S_EXT_CTRLS", || unsafe {
            ioctl::vidioc_s_ext_ctrls(fd.as_raw_fd(), &mut ctrls)
        })
    } else {
        // SAFETY: as above.
        retry("VIDIOC_G_EXT_CTRLS", || unsafe {
            ioctl::vidioc_g_ext_ctrls(fd.as_raw_fd(), &mut ctrls)
        })
    }
    .map(|_| ())
}

fn decode(info: &ControlInfo, ctrl: &v4l2_ext_control, payload: Vec<u8>) -> ControlValue {
    match info.control_type {
        ControlType::Integer64 => ControlValue::Integer64(ctrl.value64()),
        ControlType::String => ControlValue::String(c_string(&payload)),
        _ if info.flags.has_payload() => ControlValue::Payload(payload),
        _ => ControlValue::Integer(ctrl.value()),
    }
}

fn query_ext(fd: std::os::fd::BorrowedFd<'_>, id: u32) -> Result<ControlInfo> {
    let mut q = v4l2_query_ext_ctrl {
        id,
        ..Default::default()
    };
    // SAFETY: valid fd; `q` outlives the call.
    retry("VIDIOC_QUERY_EXT_CTRL", || unsafe {
        ioctl::vidioc_query_ext_ctrl(fd.as_raw_fd(), &mut q)
    })?;
    let mut info = ControlInfo {
        id: q.id,
        name: c_string(&q.name),
        control_type: ControlType::from_raw(q.type_),
        minimum: q.minimum,
        maximum: q.maximum,
        step: q.step,
        default_value: q.default_value,
        flags: ControlFlags(q.flags),
        elem_size: q.elem_size,
        elems: q.elems,
        dims: q.dims[..(q.nr_of_dims as usize).min(q.dims.len())].to_vec(),
        menu: Vec::new(),
    };
    info.menu = menu_items(fd, &info)?;
    Ok(info)
}

fn query_legacy(fd: std::os::fd::BorrowedFd<'_>, id: u32) -> Result<ControlInfo> {
    let mut q = v4l2_queryctrl {
        id,
        ..Default::default()
    };
    // SAFETY: valid fd; `q` outlives the call.
    retry("VIDIOC_QUERYCTRL", || unsafe {
        ioctl::vidioc_queryctrl(fd.as_raw_fd(), &mut q)
    })?;
    let control_type = ControlType::from_raw(q.type_);
    let mut info = ControlInfo {
        id: q.id,
        name: c_string(&q.name),
        control_type,
        minimum: i64::from(q.minimum),
        maximum: i64::from(q.maximum),
        step: u64::try_from(q.step).unwrap_or(0),
        default_value: i64::from(q.default_value),
        flags: ControlFlags(q.flags),
        elem_size: if control_type == ControlType::Integer64 {
            8
        } else {
            4
        },
        elems: 1,
        dims: Vec::new(),
        menu: Vec::new(),
    };
    info.menu = menu_items(fd, &info)?;
    Ok(info)
}

fn query_all_legacy(fd: std::os::fd::BorrowedFd<'_>) -> Result<Vec<ControlInfo>> {
    let mut out = Vec::new();
    let mut id = 0u32;
    loop {
        match query_legacy(fd, id | V4L2_CTRL_FLAG_NEXT_CTRL) {
            Ok(info) => {
                id = info.id;
                out.push(info);
            }
            Err(e) if e.errno() == Some(Errno::EINVAL) => return Ok(out),
            Err(e) => return Err(e),
        }
    }
}

fn menu_items(fd: std::os::fd::BorrowedFd<'_>, info: &ControlInfo) -> Result<Vec<MenuItem>> {
    let integer = match info.control_type {
        ControlType::Menu => false,
        ControlType::IntegerMenu => true,
        _ => return Ok(Vec::new()),
    };
    let (Ok(min), Ok(max)) = (u32::try_from(info.minimum), u32::try_from(info.maximum)) else {
        return Ok(Vec::new());
    };
    let mut items = Vec::new();
    for index in min..=max {
        let mut m = v4l2_querymenu {
            id: info.id,
            index,
            ..Default::default()
        };
        // SAFETY: valid fd; `m` outlives the call.
        match retry("VIDIOC_QUERYMENU", || unsafe {
            ioctl::vidioc_querymenu(fd.as_raw_fd(), &mut m)
        }) {
            Ok(_) => items.push(MenuItem {
                index,
                name: (!integer).then(|| c_string(m.name())),
                value: integer.then(|| m.value()),
            }),
            // Drivers leave gaps in menus by rejecting those indices.
            Err(e) if e.errno() == Some(Errno::EINVAL) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_type_round_trips() {
        for raw in 0..=0x0300 {
            assert_eq!(ControlType::from_raw(raw).raw(), raw);
        }
        assert_eq!(
            ControlType::from_raw(V4L2_CTRL_TYPE_STRING),
            ControlType::String
        );
        assert_eq!(ControlType::from_raw(0x0106), ControlType::Other(0x0106));
    }

    #[test]
    fn flags_decode() {
        let f = ControlFlags(V4L2_CTRL_FLAG_READ_ONLY | V4L2_CTRL_FLAG_HAS_PAYLOAD);
        assert!(f.is_read_only());
        assert!(f.has_payload());
        assert!(!f.is_write_only());
        assert!(!f.is_disabled());
    }

    fn info(control_type: ControlType, flags: u32, elem_size: u32, elems: u32) -> ControlInfo {
        ControlInfo {
            id: V4L2_CID_BRIGHTNESS,
            name: String::new(),
            control_type,
            minimum: 0,
            maximum: 255,
            step: 1,
            default_value: 0,
            flags: ControlFlags(flags),
            elem_size,
            elems,
            dims: Vec::new(),
            menu: Vec::new(),
        }
    }

    #[test]
    fn set_rejects_mismatched_values_before_the_driver() {
        let dev = std::fs::File::open("/dev/null").unwrap();
        let int = info(ControlType::Integer, 0, 4, 1);
        let e = set(&dev, &int, &ControlValue::String("x".into())).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::InvalidArgument);
        let i64c = info(ControlType::Integer64, 0, 8, 1);
        assert_eq!(
            set(&dev, &i64c, &ControlValue::Integer(1))
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidArgument
        );
        let arr = info(ControlType::U8, V4L2_CTRL_FLAG_HAS_PAYLOAD, 1, 4);
        assert_eq!(
            set(&dev, &arr, &ControlValue::Payload(vec![0; 3]))
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidArgument
        );
        let s = info(ControlType::String, V4L2_CTRL_FLAG_HAS_PAYLOAD, 4, 1);
        assert_eq!(
            set(&dev, &s, &ControlValue::String("four".into()))
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidArgument,
            "a string needs room for its NUL"
        );
        // A matching value reaches the driver, which /dev/null is not.
        assert!(set(&dev, &int, &ControlValue::Integer(3))
            .unwrap_err()
            .is_unsupported());
    }
}
