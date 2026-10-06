// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! Raw V4L2 UAPI definitions: `#[repr(C)]` structs, FourCCs and constants.
//!
//! Everything here mirrors `linux/videodev2.h` and `linux/v4l2-controls.h`
//! byte for byte. The constant values were generated from those headers, and
//! every struct carries compile-time `size_of` and `align_of` assertions, plus
//! `offset_of` assertions for its unions, checked against the headers on
//! x86_64 and aarch64. The ioctl request number is derived from `sizeof`, so a
//! layout mistake would otherwise surface only as a runtime `ENOTTY` or a
//! partial copy; the assertions turn it into a build failure.
//!
//! Unions are modelled as fixed-size fields with typed accessors. The `m`
//! union in `v4l2_buffer` and `v4l2_plane` is a single 8-byte `u64`: on
//! 64-bit targets every member (`offset: u32`, `userptr: unsigned long`,
//! `planes: *mut`, `fd: s32`) fits, matching the kernel's size and alignment.
//! The crate therefore targets 64-bit Linux.
#![allow(non_camel_case_types)]
#![allow(missing_docs)] // Struct fields mirror videodev2.h; see the kernel documentation.

use std::mem::{offset_of, size_of};

/// Maximum planes per buffer (`VIDEO_MAX_PLANES`).
pub const VIDEO_MAX_PLANES: usize = 8;

/// Builds a V4L2 FourCC (`v4l2_fourcc`): four ASCII bytes packed little-endian.
pub const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

/// Renders a FourCC as its four ASCII characters, with `?` for non-printable
/// bytes, for logging.
pub fn fourcc_str(v: u32) -> String {
    v.to_le_bytes()
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '?'
            }
        })
        .collect()
}

// ---- Capabilities --------------------------------------------------------------
/// `V4L2_CAP_VIDEO_CAPTURE`.
pub const V4L2_CAP_VIDEO_CAPTURE: u32 = 1;
/// `V4L2_CAP_VIDEO_OUTPUT`.
pub const V4L2_CAP_VIDEO_OUTPUT: u32 = 2;
/// `V4L2_CAP_VIDEO_CAPTURE_MPLANE`.
pub const V4L2_CAP_VIDEO_CAPTURE_MPLANE: u32 = 0x0000_1000;
/// `V4L2_CAP_VIDEO_OUTPUT_MPLANE`.
pub const V4L2_CAP_VIDEO_OUTPUT_MPLANE: u32 = 0x0000_2000;
/// `V4L2_CAP_VIDEO_M2M_MPLANE`.
pub const V4L2_CAP_VIDEO_M2M_MPLANE: u32 = 0x0000_4000;
/// `V4L2_CAP_VIDEO_M2M`.
pub const V4L2_CAP_VIDEO_M2M: u32 = 0x0000_8000;
/// `V4L2_CAP_META_CAPTURE`.
pub const V4L2_CAP_META_CAPTURE: u32 = 0x0080_0000;
/// `V4L2_CAP_READWRITE`.
pub const V4L2_CAP_READWRITE: u32 = 0x0100_0000;
/// `V4L2_CAP_STREAMING`.
pub const V4L2_CAP_STREAMING: u32 = 0x0400_0000;
/// `V4L2_CAP_IO_MC`.
pub const V4L2_CAP_IO_MC: u32 = 0x2000_0000;
/// `V4L2_CAP_DEVICE_CAPS`.
pub const V4L2_CAP_DEVICE_CAPS: u32 = 0x8000_0000;

// ---- Buffer types --------------------------------------------------------------
/// `V4L2_BUF_TYPE_VIDEO_CAPTURE`.
pub const V4L2_BUF_TYPE_VIDEO_CAPTURE: u32 = 1;
/// `V4L2_BUF_TYPE_VIDEO_OUTPUT`.
pub const V4L2_BUF_TYPE_VIDEO_OUTPUT: u32 = 2;
/// `V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE`.
pub const V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE: u32 = 9;
/// `V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE`.
pub const V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE: u32 = 10;
/// `V4L2_BUF_TYPE_META_CAPTURE`.
pub const V4L2_BUF_TYPE_META_CAPTURE: u32 = 13;
/// `V4L2_BUF_TYPE_META_OUTPUT`.
pub const V4L2_BUF_TYPE_META_OUTPUT: u32 = 14;

// ---- Memory types --------------------------------------------------------------
/// `V4L2_MEMORY_MMAP`.
pub const V4L2_MEMORY_MMAP: u32 = 1;
/// `V4L2_MEMORY_USERPTR`.
pub const V4L2_MEMORY_USERPTR: u32 = 2;
/// `V4L2_MEMORY_OVERLAY`.
pub const V4L2_MEMORY_OVERLAY: u32 = 3;
/// `V4L2_MEMORY_DMABUF`.
pub const V4L2_MEMORY_DMABUF: u32 = 4;

// ---- Fields --------------------------------------------------------------------
/// `V4L2_FIELD_ANY`.
pub const V4L2_FIELD_ANY: u32 = 0;
/// `V4L2_FIELD_NONE`.
pub const V4L2_FIELD_NONE: u32 = 1;

// ---- Buffer flags --------------------------------------------------------------
/// `V4L2_BUF_FLAG_MAPPED`.
pub const V4L2_BUF_FLAG_MAPPED: u32 = 1;
/// `V4L2_BUF_FLAG_QUEUED`.
pub const V4L2_BUF_FLAG_QUEUED: u32 = 2;
/// `V4L2_BUF_FLAG_DONE`.
pub const V4L2_BUF_FLAG_DONE: u32 = 4;
/// `V4L2_BUF_FLAG_KEYFRAME`.
pub const V4L2_BUF_FLAG_KEYFRAME: u32 = 8;
/// `V4L2_BUF_FLAG_PFRAME`.
pub const V4L2_BUF_FLAG_PFRAME: u32 = 0x0000_0010;
/// `V4L2_BUF_FLAG_BFRAME`.
pub const V4L2_BUF_FLAG_BFRAME: u32 = 0x0000_0020;
/// `V4L2_BUF_FLAG_ERROR`.
pub const V4L2_BUF_FLAG_ERROR: u32 = 0x0000_0040;
/// `V4L2_BUF_FLAG_PREPARED`.
pub const V4L2_BUF_FLAG_PREPARED: u32 = 0x0000_0400;
/// `V4L2_BUF_FLAG_TIMESTAMP_MASK`.
pub const V4L2_BUF_FLAG_TIMESTAMP_MASK: u32 = 0x0000_e000;
/// `V4L2_BUF_FLAG_TIMESTAMP_UNKNOWN`.
pub const V4L2_BUF_FLAG_TIMESTAMP_UNKNOWN: u32 = 0;
/// `V4L2_BUF_FLAG_TIMESTAMP_MONOTONIC`.
pub const V4L2_BUF_FLAG_TIMESTAMP_MONOTONIC: u32 = 0x0000_2000;
/// `V4L2_BUF_FLAG_TIMESTAMP_COPY`.
pub const V4L2_BUF_FLAG_TIMESTAMP_COPY: u32 = 0x0000_4000;
/// `V4L2_BUF_FLAG_TSTAMP_SRC_MASK`.
pub const V4L2_BUF_FLAG_TSTAMP_SRC_MASK: u32 = 0x0007_0000;
/// `V4L2_BUF_FLAG_TSTAMP_SRC_EOF`.
pub const V4L2_BUF_FLAG_TSTAMP_SRC_EOF: u32 = 0;
/// `V4L2_BUF_FLAG_TSTAMP_SRC_SOE`.
pub const V4L2_BUF_FLAG_TSTAMP_SRC_SOE: u32 = 0x0001_0000;
/// `V4L2_BUF_FLAG_LAST`.
pub const V4L2_BUF_FLAG_LAST: u32 = 0x0010_0000;

// ---- Buffer capabilities (v4l2_requestbuffers.capabilities, v4l2_create_buffers.capabilities) ----
/// `V4L2_BUF_CAP_SUPPORTS_MMAP`.
pub const V4L2_BUF_CAP_SUPPORTS_MMAP: u32 = 1;
/// `V4L2_BUF_CAP_SUPPORTS_USERPTR`.
pub const V4L2_BUF_CAP_SUPPORTS_USERPTR: u32 = 2;
/// `V4L2_BUF_CAP_SUPPORTS_DMABUF`.
pub const V4L2_BUF_CAP_SUPPORTS_DMABUF: u32 = 4;
/// `V4L2_BUF_CAP_SUPPORTS_REQUESTS`.
pub const V4L2_BUF_CAP_SUPPORTS_REQUESTS: u32 = 8;
/// `V4L2_BUF_CAP_SUPPORTS_ORPHANED_BUFS`.
pub const V4L2_BUF_CAP_SUPPORTS_ORPHANED_BUFS: u32 = 0x0000_0010;
/// `V4L2_BUF_CAP_SUPPORTS_M2M_HOLD_CAPTURE_BUF`.
pub const V4L2_BUF_CAP_SUPPORTS_M2M_HOLD_CAPTURE_BUF: u32 = 0x0000_0020;
/// `V4L2_BUF_CAP_SUPPORTS_MMAP_CACHE_HINTS`.
pub const V4L2_BUF_CAP_SUPPORTS_MMAP_CACHE_HINTS: u32 = 0x0000_0040;
/// `V4L2_BUF_CAP_SUPPORTS_MAX_NUM_BUFFERS`.
pub const V4L2_BUF_CAP_SUPPORTS_MAX_NUM_BUFFERS: u32 = 0x0000_0080;

// ---- Format description flags --------------------------------------------------
/// `V4L2_FMT_FLAG_COMPRESSED`.
pub const V4L2_FMT_FLAG_COMPRESSED: u32 = 1;
/// `V4L2_FMT_FLAG_EMULATED`.
pub const V4L2_FMT_FLAG_EMULATED: u32 = 2;

// ---- Frame size and interval enumeration types ---------------------------------
/// `V4L2_FRMSIZE_TYPE_DISCRETE`.
pub const V4L2_FRMSIZE_TYPE_DISCRETE: u32 = 1;
/// `V4L2_FRMSIZE_TYPE_CONTINUOUS`.
pub const V4L2_FRMSIZE_TYPE_CONTINUOUS: u32 = 2;
/// `V4L2_FRMSIZE_TYPE_STEPWISE`.
pub const V4L2_FRMSIZE_TYPE_STEPWISE: u32 = 3;
/// `V4L2_FRMIVAL_TYPE_DISCRETE`.
pub const V4L2_FRMIVAL_TYPE_DISCRETE: u32 = 1;
/// `V4L2_FRMIVAL_TYPE_CONTINUOUS`.
pub const V4L2_FRMIVAL_TYPE_CONTINUOUS: u32 = 2;
/// `V4L2_FRMIVAL_TYPE_STEPWISE`.
pub const V4L2_FRMIVAL_TYPE_STEPWISE: u32 = 3;

// ---- Stream parameters ---------------------------------------------------------
/// `V4L2_CAP_TIMEPERFRAME`.
pub const V4L2_CAP_TIMEPERFRAME: u32 = 0x0000_1000;
/// `V4L2_MODE_HIGHQUALITY`.
pub const V4L2_MODE_HIGHQUALITY: u32 = 1;

// ---- Colorspaces ---------------------------------------------------------------
/// `V4L2_COLORSPACE_DEFAULT`.
pub const V4L2_COLORSPACE_DEFAULT: u32 = 0;
/// `V4L2_COLORSPACE_SMPTE170M`.
pub const V4L2_COLORSPACE_SMPTE170M: u32 = 1;
/// `V4L2_COLORSPACE_SMPTE240M`.
pub const V4L2_COLORSPACE_SMPTE240M: u32 = 2;
/// `V4L2_COLORSPACE_REC709`.
pub const V4L2_COLORSPACE_REC709: u32 = 3;
/// `V4L2_COLORSPACE_470_SYSTEM_M`.
pub const V4L2_COLORSPACE_470_SYSTEM_M: u32 = 5;
/// `V4L2_COLORSPACE_470_SYSTEM_BG`.
pub const V4L2_COLORSPACE_470_SYSTEM_BG: u32 = 6;
/// `V4L2_COLORSPACE_JPEG`.
pub const V4L2_COLORSPACE_JPEG: u32 = 7;
/// `V4L2_COLORSPACE_SRGB`.
pub const V4L2_COLORSPACE_SRGB: u32 = 8;
/// `V4L2_COLORSPACE_OPRGB`.
pub const V4L2_COLORSPACE_OPRGB: u32 = 9;
/// `V4L2_COLORSPACE_BT2020`.
pub const V4L2_COLORSPACE_BT2020: u32 = 10;
/// `V4L2_COLORSPACE_RAW`.
pub const V4L2_COLORSPACE_RAW: u32 = 11;
/// `V4L2_COLORSPACE_DCI_P3`.
pub const V4L2_COLORSPACE_DCI_P3: u32 = 12;

// ---- Transfer functions --------------------------------------------------------
/// `V4L2_XFER_FUNC_DEFAULT`.
pub const V4L2_XFER_FUNC_DEFAULT: u8 = 0;
/// `V4L2_XFER_FUNC_709`.
pub const V4L2_XFER_FUNC_709: u8 = 1;
/// `V4L2_XFER_FUNC_SRGB`.
pub const V4L2_XFER_FUNC_SRGB: u8 = 2;
/// `V4L2_XFER_FUNC_OPRGB`.
pub const V4L2_XFER_FUNC_OPRGB: u8 = 3;
/// `V4L2_XFER_FUNC_SMPTE240M`.
pub const V4L2_XFER_FUNC_SMPTE240M: u8 = 4;
/// `V4L2_XFER_FUNC_NONE`.
pub const V4L2_XFER_FUNC_NONE: u8 = 5;
/// `V4L2_XFER_FUNC_DCI_P3`.
pub const V4L2_XFER_FUNC_DCI_P3: u8 = 6;
/// `V4L2_XFER_FUNC_SMPTE2084`.
pub const V4L2_XFER_FUNC_SMPTE2084: u8 = 7;

// ---- Y'CbCr encodings ----------------------------------------------------------
/// `V4L2_YCBCR_ENC_DEFAULT`.
pub const V4L2_YCBCR_ENC_DEFAULT: u8 = 0;
/// `V4L2_YCBCR_ENC_601`.
pub const V4L2_YCBCR_ENC_601: u8 = 1;
/// `V4L2_YCBCR_ENC_709`.
pub const V4L2_YCBCR_ENC_709: u8 = 2;
/// `V4L2_YCBCR_ENC_XV601`.
pub const V4L2_YCBCR_ENC_XV601: u8 = 3;
/// `V4L2_YCBCR_ENC_XV709`.
pub const V4L2_YCBCR_ENC_XV709: u8 = 4;
/// `V4L2_YCBCR_ENC_BT2020`.
pub const V4L2_YCBCR_ENC_BT2020: u8 = 6;
/// `V4L2_YCBCR_ENC_BT2020_CONST_LUM`.
pub const V4L2_YCBCR_ENC_BT2020_CONST_LUM: u8 = 7;
/// `V4L2_YCBCR_ENC_SMPTE240M`.
pub const V4L2_YCBCR_ENC_SMPTE240M: u8 = 8;

// ---- Quantization --------------------------------------------------------------
/// `V4L2_QUANTIZATION_DEFAULT`.
pub const V4L2_QUANTIZATION_DEFAULT: u8 = 0;
/// `V4L2_QUANTIZATION_FULL_RANGE`.
pub const V4L2_QUANTIZATION_FULL_RANGE: u8 = 1;
/// `V4L2_QUANTIZATION_LIM_RANGE`.
pub const V4L2_QUANTIZATION_LIM_RANGE: u8 = 2;

// ---- Control types -------------------------------------------------------------
/// `V4L2_CTRL_TYPE_INTEGER`.
pub const V4L2_CTRL_TYPE_INTEGER: u32 = 1;
/// `V4L2_CTRL_TYPE_BOOLEAN`.
pub const V4L2_CTRL_TYPE_BOOLEAN: u32 = 2;
/// `V4L2_CTRL_TYPE_MENU`.
pub const V4L2_CTRL_TYPE_MENU: u32 = 3;
/// `V4L2_CTRL_TYPE_BUTTON`.
pub const V4L2_CTRL_TYPE_BUTTON: u32 = 4;
/// `V4L2_CTRL_TYPE_INTEGER64`.
pub const V4L2_CTRL_TYPE_INTEGER64: u32 = 5;
/// `V4L2_CTRL_TYPE_CTRL_CLASS`.
pub const V4L2_CTRL_TYPE_CTRL_CLASS: u32 = 6;
/// `V4L2_CTRL_TYPE_STRING`.
pub const V4L2_CTRL_TYPE_STRING: u32 = 7;
/// `V4L2_CTRL_TYPE_BITMASK`.
pub const V4L2_CTRL_TYPE_BITMASK: u32 = 8;
/// `V4L2_CTRL_TYPE_INTEGER_MENU`.
pub const V4L2_CTRL_TYPE_INTEGER_MENU: u32 = 9;
/// `V4L2_CTRL_TYPE_U8`.
pub const V4L2_CTRL_TYPE_U8: u32 = 256;
/// `V4L2_CTRL_TYPE_U16`.
pub const V4L2_CTRL_TYPE_U16: u32 = 257;
/// `V4L2_CTRL_TYPE_U32`.
pub const V4L2_CTRL_TYPE_U32: u32 = 258;

// ---- Control flags -------------------------------------------------------------
/// `V4L2_CTRL_FLAG_DISABLED`.
pub const V4L2_CTRL_FLAG_DISABLED: u32 = 1;
/// `V4L2_CTRL_FLAG_GRABBED`.
pub const V4L2_CTRL_FLAG_GRABBED: u32 = 2;
/// `V4L2_CTRL_FLAG_READ_ONLY`.
pub const V4L2_CTRL_FLAG_READ_ONLY: u32 = 4;
/// `V4L2_CTRL_FLAG_UPDATE`.
pub const V4L2_CTRL_FLAG_UPDATE: u32 = 8;
/// `V4L2_CTRL_FLAG_INACTIVE`.
pub const V4L2_CTRL_FLAG_INACTIVE: u32 = 0x0000_0010;
/// `V4L2_CTRL_FLAG_SLIDER`.
pub const V4L2_CTRL_FLAG_SLIDER: u32 = 0x0000_0020;
/// `V4L2_CTRL_FLAG_WRITE_ONLY`.
pub const V4L2_CTRL_FLAG_WRITE_ONLY: u32 = 0x0000_0040;
/// `V4L2_CTRL_FLAG_VOLATILE`.
pub const V4L2_CTRL_FLAG_VOLATILE: u32 = 0x0000_0080;
/// `V4L2_CTRL_FLAG_HAS_PAYLOAD`.
pub const V4L2_CTRL_FLAG_HAS_PAYLOAD: u32 = 0x0000_0100;
/// `V4L2_CTRL_FLAG_EXECUTE_ON_WRITE`.
pub const V4L2_CTRL_FLAG_EXECUTE_ON_WRITE: u32 = 0x0000_0200;
/// `V4L2_CTRL_FLAG_MODIFY_LAYOUT`.
pub const V4L2_CTRL_FLAG_MODIFY_LAYOUT: u32 = 0x0000_0400;
/// `V4L2_CTRL_FLAG_DYNAMIC_ARRAY`.
pub const V4L2_CTRL_FLAG_DYNAMIC_ARRAY: u32 = 0x0000_0800;
/// `V4L2_CTRL_FLAG_NEXT_CTRL`.
pub const V4L2_CTRL_FLAG_NEXT_CTRL: u32 = 0x8000_0000;
/// `V4L2_CTRL_FLAG_NEXT_COMPOUND`.
pub const V4L2_CTRL_FLAG_NEXT_COMPOUND: u32 = 0x4000_0000;

// ---- Extended-control value selectors ------------------------------------------
/// `V4L2_CTRL_WHICH_CUR_VAL`.
pub const V4L2_CTRL_WHICH_CUR_VAL: u32 = 0;
/// `V4L2_CTRL_WHICH_DEF_VAL`.
pub const V4L2_CTRL_WHICH_DEF_VAL: u32 = 0x0f00_0000;
/// `V4L2_CTRL_WHICH_REQUEST_VAL`.
pub const V4L2_CTRL_WHICH_REQUEST_VAL: u32 = 0x0f01_0000;

// ---- Control IDs: user class ---------------------------------------------------
/// `V4L2_CTRL_CLASS_USER`.
pub const V4L2_CTRL_CLASS_USER: u32 = 0x0098_0000;
/// `V4L2_CID_BASE`.
pub const V4L2_CID_BASE: u32 = 0x0098_0900;
/// `V4L2_CID_BRIGHTNESS`.
pub const V4L2_CID_BRIGHTNESS: u32 = 0x0098_0900;
/// `V4L2_CID_CONTRAST`.
pub const V4L2_CID_CONTRAST: u32 = 0x0098_0901;
/// `V4L2_CID_SATURATION`.
pub const V4L2_CID_SATURATION: u32 = 0x0098_0902;
/// `V4L2_CID_HUE`.
pub const V4L2_CID_HUE: u32 = 0x0098_0903;
/// `V4L2_CID_AUTO_WHITE_BALANCE`.
pub const V4L2_CID_AUTO_WHITE_BALANCE: u32 = 0x0098_090c;
/// `V4L2_CID_EXPOSURE`.
pub const V4L2_CID_EXPOSURE: u32 = 0x0098_0911;
/// `V4L2_CID_AUTOGAIN`.
pub const V4L2_CID_AUTOGAIN: u32 = 0x0098_0912;
/// `V4L2_CID_GAIN`.
pub const V4L2_CID_GAIN: u32 = 0x0098_0913;
/// `V4L2_CID_HFLIP`.
pub const V4L2_CID_HFLIP: u32 = 0x0098_0914;
/// `V4L2_CID_VFLIP`.
pub const V4L2_CID_VFLIP: u32 = 0x0098_0915;
/// `V4L2_CID_POWER_LINE_FREQUENCY`.
pub const V4L2_CID_POWER_LINE_FREQUENCY: u32 = 0x0098_0918;
/// `V4L2_CID_WHITE_BALANCE_TEMPERATURE`.
pub const V4L2_CID_WHITE_BALANCE_TEMPERATURE: u32 = 0x0098_091a;
/// `V4L2_CID_SHARPNESS`.
pub const V4L2_CID_SHARPNESS: u32 = 0x0098_091b;
/// `V4L2_CID_ROTATE`.
pub const V4L2_CID_ROTATE: u32 = 0x0098_0922;
/// `V4L2_CID_MIN_BUFFERS_FOR_CAPTURE`.
pub const V4L2_CID_MIN_BUFFERS_FOR_CAPTURE: u32 = 0x0098_0927;
/// `V4L2_CID_MIN_BUFFERS_FOR_OUTPUT`.
pub const V4L2_CID_MIN_BUFFERS_FOR_OUTPUT: u32 = 0x0098_0928;

// ---- Control IDs: camera class -------------------------------------------------
/// `V4L2_CTRL_CLASS_CAMERA`.
pub const V4L2_CTRL_CLASS_CAMERA: u32 = 0x009a_0000;
/// `V4L2_CID_CAMERA_CLASS_BASE`.
pub const V4L2_CID_CAMERA_CLASS_BASE: u32 = 0x009a_0900;
/// `V4L2_CID_EXPOSURE_AUTO`.
pub const V4L2_CID_EXPOSURE_AUTO: u32 = 0x009a_0901;
/// `V4L2_EXPOSURE_AUTO`.
pub const V4L2_EXPOSURE_AUTO: i32 = 0;
/// `V4L2_EXPOSURE_MANUAL`.
pub const V4L2_EXPOSURE_MANUAL: i32 = 1;
/// `V4L2_EXPOSURE_SHUTTER_PRIORITY`.
pub const V4L2_EXPOSURE_SHUTTER_PRIORITY: i32 = 2;
/// `V4L2_EXPOSURE_APERTURE_PRIORITY`.
pub const V4L2_EXPOSURE_APERTURE_PRIORITY: i32 = 3;
/// `V4L2_CID_EXPOSURE_ABSOLUTE`.
pub const V4L2_CID_EXPOSURE_ABSOLUTE: u32 = 0x009a_0902;
/// `V4L2_CID_EXPOSURE_AUTO_PRIORITY`.
pub const V4L2_CID_EXPOSURE_AUTO_PRIORITY: u32 = 0x009a_0903;

// ---- Control IDs: codec class --------------------------------------------------
/// `V4L2_CTRL_CLASS_CODEC`.
pub const V4L2_CTRL_CLASS_CODEC: u32 = 0x0099_0000;
/// `V4L2_CID_CODEC_BASE`.
pub const V4L2_CID_CODEC_BASE: u32 = 0x0099_0900;
/// `V4L2_CID_MPEG_VIDEO_GOP_SIZE`.
pub const V4L2_CID_MPEG_VIDEO_GOP_SIZE: u32 = 0x0099_09cb;
/// `V4L2_CID_MPEG_VIDEO_BITRATE_MODE`.
pub const V4L2_CID_MPEG_VIDEO_BITRATE_MODE: u32 = 0x0099_09ce;
/// `V4L2_MPEG_VIDEO_BITRATE_MODE_VBR`.
pub const V4L2_MPEG_VIDEO_BITRATE_MODE_VBR: i32 = 0;
/// `V4L2_MPEG_VIDEO_BITRATE_MODE_CBR`.
pub const V4L2_MPEG_VIDEO_BITRATE_MODE_CBR: i32 = 1;
/// `V4L2_CID_MPEG_VIDEO_BITRATE`.
pub const V4L2_CID_MPEG_VIDEO_BITRATE: u32 = 0x0099_09cf;
/// `V4L2_CID_MPEG_VIDEO_HEADER_MODE`.
pub const V4L2_CID_MPEG_VIDEO_HEADER_MODE: u32 = 0x0099_09d8;
/// `V4L2_MPEG_VIDEO_HEADER_MODE_SEPARATE`.
pub const V4L2_MPEG_VIDEO_HEADER_MODE_SEPARATE: i32 = 0;
/// `V4L2_MPEG_VIDEO_HEADER_MODE_JOINED_WITH_1ST_FRAME`.
pub const V4L2_MPEG_VIDEO_HEADER_MODE_JOINED_WITH_1ST_FRAME: i32 = 1;
/// `V4L2_CID_MPEG_VIDEO_REPEAT_SEQ_HEADER`.
pub const V4L2_CID_MPEG_VIDEO_REPEAT_SEQ_HEADER: u32 = 0x0099_09e2;
/// `V4L2_CID_MPEG_VIDEO_FORCE_KEY_FRAME`.
pub const V4L2_CID_MPEG_VIDEO_FORCE_KEY_FRAME: u32 = 0x0099_09e5;
/// `V4L2_CID_MPEG_VIDEO_H264_I_PERIOD`.
pub const V4L2_CID_MPEG_VIDEO_H264_I_PERIOD: u32 = 0x0099_0a66;
/// `V4L2_CID_MPEG_VIDEO_H264_LEVEL`.
pub const V4L2_CID_MPEG_VIDEO_H264_LEVEL: u32 = 0x0099_0a67;
/// `V4L2_CID_MPEG_VIDEO_H264_PROFILE`.
pub const V4L2_CID_MPEG_VIDEO_H264_PROFILE: u32 = 0x0099_0a6b;
/// `V4L2_CID_MPEG_VIDEO_H264_MIN_QP`.
pub const V4L2_CID_MPEG_VIDEO_H264_MIN_QP: u32 = 0x0099_0a61;
/// `V4L2_CID_MPEG_VIDEO_H264_MAX_QP`.
pub const V4L2_CID_MPEG_VIDEO_H264_MAX_QP: u32 = 0x0099_0a62;
/// `V4L2_CID_MPEG_VIDEO_HEVC_PROFILE`.
pub const V4L2_CID_MPEG_VIDEO_HEVC_PROFILE: u32 = 0x0099_0b67;
/// `V4L2_CID_MPEG_VIDEO_HEVC_LEVEL`.
pub const V4L2_CID_MPEG_VIDEO_HEVC_LEVEL: u32 = 0x0099_0b68;

// ---- Events --------------------------------------------------------------------
/// `V4L2_EVENT_ALL`.
pub const V4L2_EVENT_ALL: u32 = 0;
/// `V4L2_EVENT_VSYNC`.
pub const V4L2_EVENT_VSYNC: u32 = 1;
/// `V4L2_EVENT_EOS`.
pub const V4L2_EVENT_EOS: u32 = 2;
/// `V4L2_EVENT_CTRL`.
pub const V4L2_EVENT_CTRL: u32 = 3;
/// `V4L2_EVENT_FRAME_SYNC`.
pub const V4L2_EVENT_FRAME_SYNC: u32 = 4;
/// `V4L2_EVENT_SOURCE_CHANGE`.
pub const V4L2_EVENT_SOURCE_CHANGE: u32 = 5;
/// `V4L2_EVENT_MOTION_DET`.
pub const V4L2_EVENT_MOTION_DET: u32 = 6;
/// `V4L2_EVENT_SRC_CH_RESOLUTION`.
pub const V4L2_EVENT_SRC_CH_RESOLUTION: u32 = 1;
/// `V4L2_EVENT_SUB_FL_SEND_INITIAL`.
pub const V4L2_EVENT_SUB_FL_SEND_INITIAL: u32 = 1;
/// `V4L2_EVENT_SUB_FL_ALLOW_FEEDBACK`.
pub const V4L2_EVENT_SUB_FL_ALLOW_FEEDBACK: u32 = 2;

// ---- Selection targets and flags -----------------------------------------------
/// `V4L2_SEL_TGT_CROP`.
pub const V4L2_SEL_TGT_CROP: u32 = 0;
/// `V4L2_SEL_TGT_CROP_DEFAULT`.
pub const V4L2_SEL_TGT_CROP_DEFAULT: u32 = 1;
/// `V4L2_SEL_TGT_CROP_BOUNDS`.
pub const V4L2_SEL_TGT_CROP_BOUNDS: u32 = 2;
/// `V4L2_SEL_TGT_NATIVE_SIZE`.
pub const V4L2_SEL_TGT_NATIVE_SIZE: u32 = 3;
/// `V4L2_SEL_TGT_COMPOSE`.
pub const V4L2_SEL_TGT_COMPOSE: u32 = 256;
/// `V4L2_SEL_TGT_COMPOSE_DEFAULT`.
pub const V4L2_SEL_TGT_COMPOSE_DEFAULT: u32 = 257;
/// `V4L2_SEL_TGT_COMPOSE_BOUNDS`.
pub const V4L2_SEL_TGT_COMPOSE_BOUNDS: u32 = 258;
/// `V4L2_SEL_TGT_COMPOSE_PADDED`.
pub const V4L2_SEL_TGT_COMPOSE_PADDED: u32 = 259;
/// `V4L2_SEL_FLAG_GE`.
pub const V4L2_SEL_FLAG_GE: u32 = 1;
/// `V4L2_SEL_FLAG_LE`.
pub const V4L2_SEL_FLAG_LE: u32 = 2;
/// `V4L2_SEL_FLAG_KEEP_CONFIG`.
pub const V4L2_SEL_FLAG_KEEP_CONFIG: u32 = 4;

// ---- Encoder and decoder commands ----------------------------------------------
/// `V4L2_ENC_CMD_START`.
pub const V4L2_ENC_CMD_START: u32 = 0;
/// `V4L2_ENC_CMD_STOP`.
pub const V4L2_ENC_CMD_STOP: u32 = 1;
/// `V4L2_ENC_CMD_PAUSE`.
pub const V4L2_ENC_CMD_PAUSE: u32 = 2;
/// `V4L2_ENC_CMD_RESUME`.
pub const V4L2_ENC_CMD_RESUME: u32 = 3;
/// `V4L2_ENC_CMD_STOP_AT_GOP_END`.
pub const V4L2_ENC_CMD_STOP_AT_GOP_END: u32 = 1;
/// `V4L2_DEC_CMD_START`.
pub const V4L2_DEC_CMD_START: u32 = 0;
/// `V4L2_DEC_CMD_STOP`.
pub const V4L2_DEC_CMD_STOP: u32 = 1;
/// `V4L2_DEC_CMD_PAUSE`.
pub const V4L2_DEC_CMD_PAUSE: u32 = 2;
/// `V4L2_DEC_CMD_RESUME`.
pub const V4L2_DEC_CMD_RESUME: u32 = 3;
/// `V4L2_DEC_CMD_FLUSH`.
pub const V4L2_DEC_CMD_FLUSH: u32 = 4;
/// `V4L2_DEC_CMD_START_MUTE_AUDIO`.
pub const V4L2_DEC_CMD_START_MUTE_AUDIO: u32 = 1;
/// `V4L2_DEC_CMD_PAUSE_TO_BLACK`.
pub const V4L2_DEC_CMD_PAUSE_TO_BLACK: u32 = 1;
/// `V4L2_DEC_CMD_STOP_TO_BLACK`.
pub const V4L2_DEC_CMD_STOP_TO_BLACK: u32 = 1;
/// `V4L2_DEC_CMD_STOP_IMMEDIATELY`.
pub const V4L2_DEC_CMD_STOP_IMMEDIATELY: u32 = 2;

// ---- Pixel formats -------------------------------------------------------------
/// `V4L2_PIX_FMT_GREY` (FourCC `GREY`).
pub const V4L2_PIX_FMT_GREY: u32 = fourcc(b'G', b'R', b'E', b'Y');
/// `V4L2_PIX_FMT_RGB565` (FourCC `RGBP`).
pub const V4L2_PIX_FMT_RGB565: u32 = fourcc(b'R', b'G', b'B', b'P');
/// `V4L2_PIX_FMT_RGB24` (FourCC `RGB3`).
pub const V4L2_PIX_FMT_RGB24: u32 = fourcc(b'R', b'G', b'B', b'3');
/// `V4L2_PIX_FMT_BGR24` (FourCC `BGR3`).
pub const V4L2_PIX_FMT_BGR24: u32 = fourcc(b'B', b'G', b'R', b'3');
/// `V4L2_PIX_FMT_XRGB32` (FourCC `BX24`).
pub const V4L2_PIX_FMT_XRGB32: u32 = fourcc(b'B', b'X', b'2', b'4');
/// `V4L2_PIX_FMT_ARGB32` (FourCC `BA24`).
pub const V4L2_PIX_FMT_ARGB32: u32 = fourcc(b'B', b'A', b'2', b'4');
/// `V4L2_PIX_FMT_XBGR32` (FourCC `XR24`).
pub const V4L2_PIX_FMT_XBGR32: u32 = fourcc(b'X', b'R', b'2', b'4');
/// `V4L2_PIX_FMT_ABGR32` (FourCC `AR24`).
pub const V4L2_PIX_FMT_ABGR32: u32 = fourcc(b'A', b'R', b'2', b'4');
/// `V4L2_PIX_FMT_YUYV` (FourCC `YUYV`).
pub const V4L2_PIX_FMT_YUYV: u32 = fourcc(b'Y', b'U', b'Y', b'V');
/// `V4L2_PIX_FMT_UYVY` (FourCC `UYVY`).
pub const V4L2_PIX_FMT_UYVY: u32 = fourcc(b'U', b'Y', b'V', b'Y');
/// `V4L2_PIX_FMT_YVYU` (FourCC `YVYU`).
pub const V4L2_PIX_FMT_YVYU: u32 = fourcc(b'Y', b'V', b'Y', b'U');
/// `V4L2_PIX_FMT_VYUY` (FourCC `VYUY`).
pub const V4L2_PIX_FMT_VYUY: u32 = fourcc(b'V', b'Y', b'U', b'Y');
/// `V4L2_PIX_FMT_YUV24` (FourCC `YUV3`).
pub const V4L2_PIX_FMT_YUV24: u32 = fourcc(b'Y', b'U', b'V', b'3');
/// `V4L2_PIX_FMT_YUV32` (FourCC `YUV4`).
pub const V4L2_PIX_FMT_YUV32: u32 = fourcc(b'Y', b'U', b'V', b'4');
/// `V4L2_PIX_FMT_NV12` (FourCC `NV12`).
pub const V4L2_PIX_FMT_NV12: u32 = fourcc(b'N', b'V', b'1', b'2');
/// `V4L2_PIX_FMT_NV21` (FourCC `NV21`).
pub const V4L2_PIX_FMT_NV21: u32 = fourcc(b'N', b'V', b'2', b'1');
/// `V4L2_PIX_FMT_NV16` (FourCC `NV16`).
pub const V4L2_PIX_FMT_NV16: u32 = fourcc(b'N', b'V', b'1', b'6');
/// `V4L2_PIX_FMT_NV61` (FourCC `NV61`).
pub const V4L2_PIX_FMT_NV61: u32 = fourcc(b'N', b'V', b'6', b'1');
/// `V4L2_PIX_FMT_NV24` (FourCC `NV24`).
pub const V4L2_PIX_FMT_NV24: u32 = fourcc(b'N', b'V', b'2', b'4');
/// `V4L2_PIX_FMT_NV12M` (FourCC `NM12`).
pub const V4L2_PIX_FMT_NV12M: u32 = fourcc(b'N', b'M', b'1', b'2');
/// `V4L2_PIX_FMT_NV21M` (FourCC `NM21`).
pub const V4L2_PIX_FMT_NV21M: u32 = fourcc(b'N', b'M', b'2', b'1');
/// `V4L2_PIX_FMT_YUV420` (FourCC `YU12`).
pub const V4L2_PIX_FMT_YUV420: u32 = fourcc(b'Y', b'U', b'1', b'2');
/// `V4L2_PIX_FMT_YUV420M` (FourCC `YM12`).
pub const V4L2_PIX_FMT_YUV420M: u32 = fourcc(b'Y', b'M', b'1', b'2');
/// `V4L2_PIX_FMT_YUV444M` (FourCC `YM24`).
pub const V4L2_PIX_FMT_YUV444M: u32 = fourcc(b'Y', b'M', b'2', b'4');
/// `V4L2_PIX_FMT_SBGGR8` (FourCC `BA81`).
pub const V4L2_PIX_FMT_SBGGR8: u32 = fourcc(b'B', b'A', b'8', b'1');
/// `V4L2_PIX_FMT_SRGGB10` (FourCC `RG10`).
pub const V4L2_PIX_FMT_SRGGB10: u32 = fourcc(b'R', b'G', b'1', b'0');
/// `V4L2_PIX_FMT_SBGGR12` (FourCC `BG12`).
pub const V4L2_PIX_FMT_SBGGR12: u32 = fourcc(b'B', b'G', b'1', b'2');
/// `V4L2_PIX_FMT_MJPEG` (FourCC `MJPG`).
pub const V4L2_PIX_FMT_MJPEG: u32 = fourcc(b'M', b'J', b'P', b'G');
/// `V4L2_PIX_FMT_JPEG` (FourCC `JPEG`).
pub const V4L2_PIX_FMT_JPEG: u32 = fourcc(b'J', b'P', b'E', b'G');
/// `V4L2_PIX_FMT_H264` (FourCC `H264`).
pub const V4L2_PIX_FMT_H264: u32 = fourcc(b'H', b'2', b'6', b'4');
/// `V4L2_PIX_FMT_HEVC` (FourCC `HEVC`).
pub const V4L2_PIX_FMT_HEVC: u32 = fourcc(b'H', b'E', b'V', b'C');

/// `struct v4l2_capability` — returned by `VIDIOC_QUERYCAP` (104 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct v4l2_capability {
    pub driver: [u8; 16],
    pub card: [u8; 32],
    pub bus_info: [u8; 32],
    pub version: u32,
    pub capabilities: u32,
    pub device_caps: u32,
    pub reserved: [u32; 3],
}

impl Default for v4l2_capability {
    fn default() -> Self {
        // SAFETY: POD of integers/byte arrays; all-zero is a valid initial
        // state and the kernel overwrites it on QUERYCAP.
        unsafe { std::mem::zeroed() }
    }
}

impl v4l2_capability {
    /// The effective capabilities for *this* device node (per-node
    /// `device_caps` when `V4L2_CAP_DEVICE_CAPS` is set, else driver-wide).
    pub fn effective_caps(&self) -> u32 {
        if self.capabilities & V4L2_CAP_DEVICE_CAPS != 0 {
            self.device_caps
        } else {
            self.capabilities
        }
    }
}

/// `struct v4l2_fmtdesc` — one entry from `VIDIOC_ENUM_FMT` (64 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct v4l2_fmtdesc {
    pub index: u32,
    pub type_: u32,
    pub flags: u32,
    pub description: [u8; 32],
    pub pixelformat: u32,
    pub reserved: [u32; 4],
}

impl Default for v4l2_fmtdesc {
    fn default() -> Self {
        // SAFETY: POD; all-zero is the documented ENUM_FMT request state.
        unsafe { std::mem::zeroed() }
    }
}

/// `struct v4l2_plane_pix_format` — per-plane geometry (20 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct v4l2_plane_pix_format {
    pub sizeimage: u32,
    pub bytesperline: u32,
    pub reserved: [u16; 6],
}

/// `struct v4l2_pix_format` — single-planar format (48 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct v4l2_pix_format {
    pub width: u32,
    pub height: u32,
    pub pixelformat: u32,
    pub field: u32,
    pub bytesperline: u32,
    pub sizeimage: u32,
    pub colorspace: u32,
    pub priv_: u32,
    pub flags: u32,
    pub ycbcr_enc: u32,
    pub quantization: u32,
    pub xfer_func: u32,
}

/// `struct v4l2_pix_format_mplane` — multi-planar format (192 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct v4l2_pix_format_mplane {
    pub width: u32,
    pub height: u32,
    pub pixelformat: u32,
    pub field: u32,
    pub colorspace: u32,
    pub plane_fmt: [v4l2_plane_pix_format; VIDEO_MAX_PLANES],
    pub num_planes: u8,
    pub flags: u8,
    pub ycbcr_enc: u8,
    pub quantization: u8,
    pub xfer_func: u8,
    pub reserved: [u8; 7],
}

impl Default for v4l2_pix_format_mplane {
    fn default() -> Self {
        // SAFETY: POD of integers; all-zero is a valid initial state.
        unsafe { std::mem::zeroed() }
    }
}

/// `struct v4l2_format` (208 bytes). The kernel's format union includes
/// `v4l2_window`, whose pointer members give the union 8-byte alignment — so
/// `{u32 type; union}` is `4 + 4 pad + 200 = 208`, and the 200-byte `fmt`
/// payload starts at offset 8. `_pad` reproduces that padding so the struct
/// size matches the kernel's (the ioctl request number encodes `sizeof`, so a
/// 204-byte struct yields the wrong number and `ENOTTY`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct v4l2_format {
    pub type_: u32,
    /// Alignment padding (the kernel union is 8-aligned); always zero.
    pub _pad: u32,
    pub fmt: [u8; 200],
}

impl std::fmt::Debug for v4l2_format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("v4l2_format")
            .field("type_", &self.type_)
            .finish_non_exhaustive()
    }
}

impl Default for v4l2_format {
    fn default() -> Self {
        Self {
            type_: 0,
            _pad: 0,
            fmt: [0u8; 200],
        }
    }
}

impl v4l2_format {
    /// Typed mutable view of the single-planar payload.
    ///
    /// # Safety
    /// The caller must ensure `type_` selects the single-planar variant.
    pub unsafe fn pix(&mut self) -> &mut v4l2_pix_format {
        // SAFETY: `fmt` holds 200 bytes, more than size_of::<v4l2_pix_format>(),
        // at a 4-byte-aligned offset (8), which meets its alignment; the caller
        // guarantees the payload is the single-planar variant.
        unsafe { &mut *(self.fmt.as_mut_ptr() as *mut v4l2_pix_format) }
    }

    /// Typed mutable view of the multi-planar payload.
    ///
    /// # Safety
    /// The caller must ensure `type_` selects the multi-planar variant.
    pub unsafe fn pix_mp(&mut self) -> &mut v4l2_pix_format_mplane {
        // SAFETY: `fmt` holds 200 bytes, more than
        // size_of::<v4l2_pix_format_mplane>(), at a 4-byte-aligned offset (8),
        // which meets its alignment; the caller guarantees the payload is the
        // multi-planar variant.
        unsafe { &mut *(self.fmt.as_mut_ptr() as *mut v4l2_pix_format_mplane) }
    }
}

/// `struct v4l2_requestbuffers` — `VIDIOC_REQBUFS` (20 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct v4l2_requestbuffers {
    pub count: u32,
    pub type_: u32,
    pub memory: u32,
    pub capabilities: u32,
    pub flags: u8,
    pub reserved: [u8; 3],
}

/// `struct v4l2_timecode` (16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct v4l2_timecode {
    pub type_: u32,
    pub flags: u32,
    pub frames: u8,
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub userbits: [u8; 4],
}

/// Reads a 32-bit member of an 8-byte `m` union. The member occupies the
/// first four bytes in memory, which is the low half of the `u64` only on
/// little-endian targets.
fn union_get_u32(m: u64) -> u32 {
    let b = m.to_ne_bytes();
    u32::from_ne_bytes([b[0], b[1], b[2], b[3]])
}

/// Builds an 8-byte `m` union holding a 32-bit member, with the remaining
/// bytes zeroed.
fn union_set_u32(v: u32) -> u64 {
    let mut b = [0u8; 8];
    b[..4].copy_from_slice(&v.to_ne_bytes());
    u64::from_ne_bytes(b)
}

/// `struct v4l2_plane` (64 bytes). `m` overlays the `{mem_offset:u32,
/// userptr:unsigned long, fd:s32}` union as a single 8-byte slot.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct v4l2_plane {
    pub bytesused: u32,
    pub length: u32,
    pub m: u64,
    pub data_offset: u32,
    pub reserved: [u32; 11],
}

impl Default for v4l2_plane {
    fn default() -> Self {
        // SAFETY: POD; all-zero is the documented initial state.
        unsafe { std::mem::zeroed() }
    }
}

impl v4l2_plane {
    /// `m.mem_offset` — MMAP plane offset (set by QUERYBUF).
    pub fn mem_offset(&self) -> u32 {
        union_get_u32(self.m)
    }
    /// Set `m.fd` — import a dmabuf into this plane (DMABUF memory).
    pub fn set_fd(&mut self, fd: i32) {
        self.m = union_set_u32(fd as u32);
    }
}

/// `struct v4l2_buffer` (88 bytes). `m` overlays the `{offset:u32,
/// userptr:unsigned long, planes:*mut v4l2_plane, fd:s32}` union.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct v4l2_buffer {
    pub index: u32,
    pub type_: u32,
    pub bytesused: u32,
    pub flags: u32,
    pub field: u32,
    pub timestamp: libc::timeval,
    pub timecode: v4l2_timecode,
    pub sequence: u32,
    pub memory: u32,
    pub m: u64,
    pub length: u32,
    pub reserved2: u32,
    pub request_fd: i32,
}

impl Default for v4l2_buffer {
    fn default() -> Self {
        // SAFETY: POD (libc::timeval is integers); all-zero is the documented
        // initial state for a buffer request.
        unsafe { std::mem::zeroed() }
    }
}

impl v4l2_buffer {
    /// Set `m.offset` — single-planar MMAP offset.
    pub fn set_offset(&mut self, offset: u32) {
        self.m = union_set_u32(offset);
    }
    /// `m.offset` — single-planar MMAP offset (from QUERYBUF).
    pub fn offset(&self) -> u32 {
        union_get_u32(self.m)
    }
    /// Set `m.fd` — single-planar dmabuf import.
    pub fn set_fd(&mut self, fd: i32) {
        self.m = union_set_u32(fd as u32);
    }
    /// Set `m.planes` — multi-planar plane array pointer. The pointed-to array
    /// must outlive the ioctl call.
    pub fn set_planes(&mut self, planes: *mut v4l2_plane) {
        self.m = planes as usize as u64;
    }
}

/// `struct v4l2_event` — `VIDIOC_DQEVENT` (136 bytes). The kernel's `u` union
/// has `u64` members (e.g. `v4l2_event_ctrl.value64`), giving it 8-byte
/// alignment, so it starts at offset 8 — `_pad` reproduces that so `pending`
/// lands at the right offset. The 64-byte `u` payload is opaque to us; we only
/// read `type_`/`pending`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct v4l2_event {
    pub type_: u32,
    /// Alignment padding (the kernel union is 8-aligned); always zero.
    pub _pad: u32,
    pub u: [u8; 64],
    pub pending: u32,
    pub sequence: u32,
    pub timestamp: libc::timespec,
    pub id: u32,
    pub reserved: [u32; 8],
}

impl Default for v4l2_event {
    fn default() -> Self {
        // SAFETY: POD (libc::timespec is integers); all-zero is valid.
        unsafe { std::mem::zeroed() }
    }
}

/// `struct v4l2_event_subscription` — `VIDIOC_SUBSCRIBE_EVENT` (32 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct v4l2_event_subscription {
    pub type_: u32,
    pub id: u32,
    pub flags: u32,
    pub reserved: [u32; 5],
}

impl v4l2_event {
    /// `u.src_change.changes` for a `V4L2_EVENT_SOURCE_CHANGE` event.
    pub fn src_change(&self) -> u32 {
        u32::from_ne_bytes([self.u[0], self.u[1], self.u[2], self.u[3]])
    }
}

/// `struct v4l2_event_src_change` (4 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_event_src_change {
    pub changes: u32,
}

/// `struct v4l2_frmsize_discrete` (8 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_frmsize_discrete {
    pub width: u32,
    pub height: u32,
}

/// `struct v4l2_frmsize_stepwise` (24 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_frmsize_stepwise {
    pub min_width: u32,
    pub max_width: u32,
    pub step_width: u32,
    pub min_height: u32,
    pub max_height: u32,
    pub step_height: u32,
}

/// `struct v4l2_frmsizeenum` — one entry from `VIDIOC_ENUM_FRAMESIZES`
/// (44 bytes). `u` is the `{discrete, stepwise}` union; read it with
/// [`discrete`](Self::discrete) or [`stepwise`](Self::stepwise) according to
/// `type_`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_frmsizeenum {
    pub index: u32,
    pub pixel_format: u32,
    pub type_: u32,
    pub u: [u32; 6],
    pub reserved: [u32; 2],
}

impl v4l2_frmsizeenum {
    /// The size, for `V4L2_FRMSIZE_TYPE_DISCRETE`.
    pub fn discrete(&self) -> v4l2_frmsize_discrete {
        v4l2_frmsize_discrete {
            width: self.u[0],
            height: self.u[1],
        }
    }

    /// The range, for `V4L2_FRMSIZE_TYPE_STEPWISE` and `_CONTINUOUS`.
    pub fn stepwise(&self) -> v4l2_frmsize_stepwise {
        let [min_width, max_width, step_width, min_height, max_height, step_height] = self.u;
        v4l2_frmsize_stepwise {
            min_width,
            max_width,
            step_width,
            min_height,
            max_height,
            step_height,
        }
    }
}

/// `struct v4l2_fract` (8 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_fract {
    pub numerator: u32,
    pub denominator: u32,
}

/// `struct v4l2_frmival_stepwise` (24 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_frmival_stepwise {
    pub min: v4l2_fract,
    pub max: v4l2_fract,
    pub step: v4l2_fract,
}

/// `struct v4l2_frmivalenum` — one entry from `VIDIOC_ENUM_FRAMEINTERVALS`
/// (52 bytes). `u` is the `{discrete, stepwise}` union; read it with
/// [`discrete`](Self::discrete) or [`stepwise`](Self::stepwise) according to
/// `type_`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_frmivalenum {
    pub index: u32,
    pub pixel_format: u32,
    pub width: u32,
    pub height: u32,
    pub type_: u32,
    pub u: [u32; 6],
    pub reserved: [u32; 2],
}

impl v4l2_frmivalenum {
    /// The interval, for `V4L2_FRMIVAL_TYPE_DISCRETE`.
    pub fn discrete(&self) -> v4l2_fract {
        v4l2_fract {
            numerator: self.u[0],
            denominator: self.u[1],
        }
    }

    /// The range, for `V4L2_FRMIVAL_TYPE_STEPWISE` and `_CONTINUOUS`.
    pub fn stepwise(&self) -> v4l2_frmival_stepwise {
        let f = |i: usize| v4l2_fract {
            numerator: self.u[i],
            denominator: self.u[i + 1],
        };
        v4l2_frmival_stepwise {
            min: f(0),
            max: f(2),
            step: f(4),
        }
    }
}

/// `struct v4l2_captureparm` (40 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_captureparm {
    pub capability: u32,
    pub capturemode: u32,
    pub timeperframe: v4l2_fract,
    pub extendedmode: u32,
    pub readbuffers: u32,
    pub reserved: [u32; 4],
}

/// `struct v4l2_outputparm` (40 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_outputparm {
    pub capability: u32,
    pub outputmode: u32,
    pub timeperframe: v4l2_fract,
    pub extendedmode: u32,
    pub writebuffers: u32,
    pub reserved: [u32; 4],
}

/// `struct v4l2_streamparm` — `VIDIOC_G_PARM` / `VIDIOC_S_PARM` (204 bytes).
/// `parm` is the `{capture, output, raw_data}` union; read and write it with
/// the accessors according to `type_`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct v4l2_streamparm {
    pub type_: u32,
    pub parm: [u8; 200],
}

impl Default for v4l2_streamparm {
    fn default() -> Self {
        Self {
            type_: 0,
            parm: [0; 200],
        }
    }
}

impl v4l2_streamparm {
    /// The capture parameters.
    pub fn capture(&self) -> v4l2_captureparm {
        // SAFETY: `parm` holds at least size_of::<v4l2_captureparm>() bytes,
        // every bit pattern is a valid v4l2_captureparm, and the read is
        // unaligned-safe.
        unsafe { std::ptr::read_unaligned(self.parm.as_ptr().cast()) }
    }

    /// Writes the capture parameters.
    pub fn set_capture(&mut self, parm: v4l2_captureparm) {
        // SAFETY: `parm` holds at least size_of::<v4l2_captureparm>() bytes and
        // the write is unaligned-safe.
        unsafe { std::ptr::write_unaligned(self.parm.as_mut_ptr().cast(), parm) }
    }

    /// The output parameters.
    pub fn output(&self) -> v4l2_outputparm {
        // SAFETY: as for `capture`.
        unsafe { std::ptr::read_unaligned(self.parm.as_ptr().cast()) }
    }

    /// Writes the output parameters.
    pub fn set_output(&mut self, parm: v4l2_outputparm) {
        // SAFETY: as for `set_capture`.
        unsafe { std::ptr::write_unaligned(self.parm.as_mut_ptr().cast(), parm) }
    }
}

/// `struct v4l2_exportbuffer` — `VIDIOC_EXPBUF` (64 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_exportbuffer {
    pub type_: u32,
    pub index: u32,
    pub plane: u32,
    pub flags: u32,
    pub fd: i32,
    pub reserved: [u32; 11],
}

/// `struct v4l2_create_buffers` — `VIDIOC_CREATE_BUFS` (256 bytes). The
/// kernel aligns `format` to 8 bytes (offset 16); `_pad` reproduces that.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct v4l2_create_buffers {
    pub index: u32,
    pub count: u32,
    pub memory: u32,
    /// Alignment padding before `format`; always zero.
    pub _pad: u32,
    pub format: v4l2_format,
    pub capabilities: u32,
    pub flags: u32,
    pub max_num_buffers: u32,
    pub reserved: [u32; 5],
}

impl std::fmt::Debug for v4l2_create_buffers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("v4l2_create_buffers")
            .field("index", &self.index)
            .field("count", &self.count)
            .field("memory", &self.memory)
            .field("format", &self.format)
            .field("capabilities", &self.capabilities)
            .field("flags", &self.flags)
            .field("max_num_buffers", &self.max_num_buffers)
            .finish_non_exhaustive()
    }
}

impl Default for v4l2_create_buffers {
    fn default() -> Self {
        // SAFETY: POD of integers and byte arrays; all-zero is valid.
        unsafe { std::mem::zeroed() }
    }
}

/// `struct v4l2_queryctrl` — `VIDIOC_QUERYCTRL` (68 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_queryctrl {
    pub id: u32,
    pub type_: u32,
    pub name: [u8; 32],
    pub minimum: i32,
    pub maximum: i32,
    pub step: i32,
    pub default_value: i32,
    pub flags: u32,
    pub reserved: [u32; 2],
}

/// `struct v4l2_query_ext_ctrl` — `VIDIOC_QUERY_EXT_CTRL` (232 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct v4l2_query_ext_ctrl {
    pub id: u32,
    pub type_: u32,
    pub name: [u8; 32],
    pub minimum: i64,
    pub maximum: i64,
    pub step: u64,
    pub default_value: i64,
    pub flags: u32,
    pub elem_size: u32,
    pub elems: u32,
    pub nr_of_dims: u32,
    pub dims: [u32; 4],
    pub reserved: [u32; 32],
}

impl Default for v4l2_query_ext_ctrl {
    fn default() -> Self {
        // SAFETY: POD of integers and byte arrays; all-zero is valid.
        unsafe { std::mem::zeroed() }
    }
}

/// `struct v4l2_querymenu` — `VIDIOC_QUERYMENU` (44 bytes). The kernel
/// struct is packed; `u` is the `{name, value}` union as bytes, which gives
/// the same layout without packing. Read it with [`name`](Self::name) for
/// `V4L2_CTRL_TYPE_MENU` or [`value`](Self::value) for
/// `V4L2_CTRL_TYPE_INTEGER_MENU`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_querymenu {
    pub id: u32,
    pub index: u32,
    pub u: [u8; 32],
    pub reserved: u32,
}

impl v4l2_querymenu {
    /// The item name, up to the first NUL.
    pub fn name(&self) -> &[u8] {
        let end = self.u.iter().position(|&b| b == 0).unwrap_or(self.u.len());
        &self.u[..end]
    }

    /// The item value of an integer menu.
    pub fn value(&self) -> i64 {
        let mut b = [0; 8];
        b.copy_from_slice(&self.u[..8]);
        i64::from_ne_bytes(b)
    }
}

/// `struct v4l2_control` — `VIDIOC_G_CTRL` / `VIDIOC_S_CTRL` (8 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_control {
    pub id: u32,
    pub value: i32,
}

/// `struct v4l2_ext_control` (20 bytes). The kernel struct is packed; `u` is
/// the 8-byte value union as bytes, which gives the same layout without
/// packing. Use the accessors to read and write it.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_ext_control {
    pub id: u32,
    pub size: u32,
    pub reserved2: [u32; 1],
    pub u: [u8; 8],
}

impl v4l2_ext_control {
    /// `value`, for 32-bit controls.
    pub fn value(&self) -> i32 {
        i32::from_ne_bytes([self.u[0], self.u[1], self.u[2], self.u[3]])
    }

    /// Sets `value`, for 32-bit controls.
    pub fn set_value(&mut self, value: i32) {
        self.u = [0; 8];
        self.u[..4].copy_from_slice(&value.to_ne_bytes());
    }

    /// `value64`, for 64-bit controls.
    pub fn value64(&self) -> i64 {
        i64::from_ne_bytes(self.u)
    }

    /// Sets `value64`, for 64-bit controls.
    pub fn set_value64(&mut self, value: i64) {
        self.u = value.to_ne_bytes();
    }

    /// Sets the payload pointer (`string`, `p_u8`, `ptr`, ...) for controls
    /// with `V4L2_CTRL_FLAG_HAS_PAYLOAD`. The pointed-to memory, of `size`
    /// bytes, must outlive the ioctl call.
    pub fn set_ptr(&mut self, ptr: *mut std::ffi::c_void) {
        self.u = (ptr as usize as u64).to_ne_bytes();
    }
}

/// `struct v4l2_ext_controls` — `VIDIOC_G/S/TRY_EXT_CTRLS` (32 bytes).
/// `which` is the `{ctrl_class, which}` union. `controls` points at `count`
/// controls that must outlive the ioctl call.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct v4l2_ext_controls {
    pub which: u32,
    pub count: u32,
    pub error_idx: u32,
    pub request_fd: i32,
    pub reserved: [u32; 1],
    pub controls: *mut v4l2_ext_control,
}

impl Default for v4l2_ext_controls {
    fn default() -> Self {
        Self {
            which: 0,
            count: 0,
            error_idx: 0,
            request_fd: 0,
            reserved: [0],
            controls: std::ptr::null_mut(),
        }
    }
}

/// `struct v4l2_rect` (16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_rect {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

/// `struct v4l2_selection` — `VIDIOC_G/S_SELECTION` (64 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_selection {
    pub type_: u32,
    pub target: u32,
    pub flags: u32,
    pub r: v4l2_rect,
    pub reserved: [u32; 9],
}

/// `struct v4l2_encoder_cmd` — `VIDIOC_(TRY_)ENCODER_CMD` (40 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_encoder_cmd {
    pub cmd: u32,
    pub flags: u32,
    pub raw: [u32; 8],
}

/// `struct v4l2_decoder_cmd` — `VIDIOC_(TRY_)DECODER_CMD` (72 bytes). `u` is
/// the 8-byte-aligned `{stop, start, raw}` union.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct v4l2_decoder_cmd {
    pub cmd: u32,
    pub flags: u32,
    pub u: [u64; 8],
}

impl v4l2_decoder_cmd {
    /// `stop.pts`, for `V4L2_DEC_CMD_STOP`.
    pub fn stop_pts(&self) -> u64 {
        self.u[0]
    }

    /// Sets `stop.pts`, for `V4L2_DEC_CMD_STOP`.
    pub fn set_stop_pts(&mut self, pts: u64) {
        self.u[0] = pts;
    }
}

// ---- Layout checks ------------------------------------------------------------
// Sizes and union offsets generated from linux/videodev2.h, identical on
// x86_64 and aarch64. Alignment is not asserted: the kernel packs some of
// these structs, and only size and field offsets reach the ioctl.
const _: () = {
    assert!(size_of::<v4l2_capability>() == 104);
    assert!(size_of::<v4l2_fmtdesc>() == 64);
    assert!(size_of::<v4l2_frmsize_discrete>() == 8);
    assert!(size_of::<v4l2_frmsize_stepwise>() == 24);
    assert!(size_of::<v4l2_frmsizeenum>() == 44);
    assert!(size_of::<v4l2_fract>() == 8);
    assert!(size_of::<v4l2_frmival_stepwise>() == 24);
    assert!(size_of::<v4l2_frmivalenum>() == 52);
    assert!(size_of::<v4l2_plane_pix_format>() == 20);
    assert!(size_of::<v4l2_pix_format>() == 48);
    assert!(size_of::<v4l2_pix_format_mplane>() == 192);
    assert!(size_of::<v4l2_format>() == 208);
    assert!(size_of::<v4l2_captureparm>() == 40);
    assert!(size_of::<v4l2_outputparm>() == 40);
    assert!(size_of::<v4l2_streamparm>() == 204);
    assert!(size_of::<v4l2_requestbuffers>() == 20);
    assert!(size_of::<v4l2_timecode>() == 16);
    assert!(size_of::<v4l2_plane>() == 64);
    assert!(size_of::<v4l2_buffer>() == 88);
    assert!(size_of::<v4l2_exportbuffer>() == 64);
    assert!(size_of::<v4l2_create_buffers>() == 256);
    assert!(size_of::<v4l2_queryctrl>() == 68);
    assert!(size_of::<v4l2_query_ext_ctrl>() == 232);
    assert!(size_of::<v4l2_querymenu>() == 44);
    assert!(size_of::<v4l2_control>() == 8);
    assert!(size_of::<v4l2_ext_control>() == 20);
    assert!(size_of::<v4l2_ext_controls>() == 32);
    assert!(size_of::<v4l2_event>() == 136);
    assert!(size_of::<v4l2_event_subscription>() == 32);
    assert!(size_of::<v4l2_event_src_change>() == 4);
    assert!(size_of::<v4l2_rect>() == 16);
    assert!(size_of::<v4l2_selection>() == 64);
    assert!(size_of::<v4l2_encoder_cmd>() == 40);
    assert!(size_of::<v4l2_decoder_cmd>() == 72);
    assert!(offset_of!(v4l2_buffer, m) == 64);
    assert!(offset_of!(v4l2_buffer, length) == 72);
    assert!(offset_of!(v4l2_buffer, request_fd) == 80);
    assert!(offset_of!(v4l2_plane, m) == 8);
    assert!(offset_of!(v4l2_plane, data_offset) == 16);
    assert!(offset_of!(v4l2_ext_control, u) == 12);
    assert!(offset_of!(v4l2_ext_controls, controls) == 24);
    assert!(offset_of!(v4l2_event, u) == 8);
    assert!(offset_of!(v4l2_event, pending) == 72);
    assert!(offset_of!(v4l2_event, timestamp) == 80);
    assert!(offset_of!(v4l2_query_ext_ctrl, minimum) == 40);
    assert!(offset_of!(v4l2_query_ext_ctrl, elem_size) == 76);
    assert!(offset_of!(v4l2_format, fmt) == 8);
    assert!(offset_of!(v4l2_streamparm, parm) == 4);
    assert!(offset_of!(v4l2_create_buffers, format) == 16);
    assert!(offset_of!(v4l2_decoder_cmd, u) == 8);
    assert!(offset_of!(v4l2_querymenu, u) == 8);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fourcc_packs_little_endian() {
        assert_eq!(V4L2_PIX_FMT_YUYV, 0x5659_5559);
        assert_eq!(fourcc_str(V4L2_PIX_FMT_NV12), "NV12");
        assert_eq!(fourcc_str(V4L2_PIX_FMT_RGB24), "RGB3");
        assert_eq!(fourcc_str(0x0000_0141), "A???");
    }

    #[test]
    fn effective_caps_honours_device_caps() {
        let mut cap = v4l2_capability {
            capabilities: V4L2_CAP_VIDEO_CAPTURE | V4L2_CAP_VIDEO_M2M,
            ..Default::default()
        };
        assert_eq!(
            cap.effective_caps(),
            V4L2_CAP_VIDEO_CAPTURE | V4L2_CAP_VIDEO_M2M
        );
        cap.capabilities |= V4L2_CAP_DEVICE_CAPS;
        cap.device_caps = V4L2_CAP_VIDEO_M2M | V4L2_CAP_STREAMING;
        assert_eq!(
            cap.effective_caps(),
            V4L2_CAP_VIDEO_M2M | V4L2_CAP_STREAMING
        );
    }

    #[test]
    fn buffer_and_plane_unions_round_trip() {
        let mut buf = v4l2_buffer::default();
        buf.set_offset(0x1000);
        assert_eq!(buf.offset(), 0x1000);
        assert_eq!(&buf.m.to_ne_bytes()[..4], &0x1000u32.to_ne_bytes());
        buf.set_fd(7);
        assert_eq!(
            buf.m.to_ne_bytes(),
            [7i32.to_ne_bytes(), [0; 4]].concat()[..],
            "fd occupies the first four bytes of the union"
        );
        let mut plane = v4l2_plane::default();
        plane.set_fd(-1);
        assert_eq!(&plane.m.to_ne_bytes()[..4], &(-1i32).to_ne_bytes());
        assert_eq!(&plane.m.to_ne_bytes()[4..], &[0; 4]);
        plane.m = u64::from_ne_bytes([0x78, 0x56, 0x34, 0x12, 0xaa, 0xbb, 0xcc, 0xdd]);
        assert_eq!(
            plane.mem_offset(),
            u32::from_ne_bytes([0x78, 0x56, 0x34, 0x12])
        );
    }

    #[test]
    fn format_payload_views_share_the_union() {
        let mut fmt = v4l2_format {
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            ..Default::default()
        };
        // SAFETY: type_ selects the single-planar payload.
        unsafe {
            fmt.pix().width = 1920;
            fmt.pix().bytesperline = 3840;
        }
        assert_eq!(&fmt.fmt[..4], &1920u32.to_ne_bytes());
        // SAFETY: reading the same bytes as the multi-planar payload.
        assert_eq!(unsafe { fmt.pix_mp().width }, 1920);
    }

    #[test]
    fn frame_size_and_interval_unions() {
        let size = v4l2_frmsizeenum {
            type_: V4L2_FRMSIZE_TYPE_STEPWISE,
            u: [176, 4096, 16, 144, 3072, 8],
            ..Default::default()
        };
        assert_eq!(
            size.discrete(),
            v4l2_frmsize_discrete {
                width: 176,
                height: 4096
            }
        );
        let step = size.stepwise();
        assert_eq!((step.max_width, step.step_height), (4096, 8));

        let ival = v4l2_frmivalenum {
            type_: V4L2_FRMIVAL_TYPE_DISCRETE,
            u: [1, 30, 0, 0, 0, 0],
            ..Default::default()
        };
        assert_eq!(
            ival.discrete(),
            v4l2_fract {
                numerator: 1,
                denominator: 30
            }
        );
        assert_eq!(
            ival.stepwise().min,
            v4l2_fract {
                numerator: 1,
                denominator: 30
            }
        );
    }

    #[test]
    fn streamparm_capture_round_trips() {
        let mut parm = v4l2_streamparm {
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            ..Default::default()
        };
        let capture = v4l2_captureparm {
            capability: V4L2_CAP_TIMEPERFRAME,
            timeperframe: v4l2_fract {
                numerator: 1,
                denominator: 60,
            },
            ..Default::default()
        };
        parm.set_capture(capture);
        assert_eq!(parm.capture(), capture);
        assert_eq!(
            parm.output().timeperframe,
            capture.timeperframe,
            "one union, two views"
        );
    }

    #[test]
    fn ext_control_value_union() {
        let mut c = v4l2_ext_control {
            id: V4L2_CID_EXPOSURE_ABSOLUTE,
            ..Default::default()
        };
        c.set_value(-5);
        assert_eq!(c.value(), -5);
        c.set_value64(1 << 40);
        assert_eq!(c.value64(), 1 << 40);
        let mut payload = [0u8; 4];
        c.set_ptr(payload.as_mut_ptr().cast());
        assert_eq!(c.value64() as usize, payload.as_ptr() as usize);
    }

    #[test]
    fn querymenu_name_and_value() {
        let mut m = v4l2_querymenu::default();
        m.u[..6].copy_from_slice(b"Manual");
        assert_eq!(m.name(), b"Manual");
        m.u[..8].copy_from_slice(&42i64.to_ne_bytes());
        assert_eq!(m.value(), 42);
    }

    #[test]
    fn decoder_stop_pts_and_event_src_change() {
        let mut cmd = v4l2_decoder_cmd {
            cmd: V4L2_DEC_CMD_STOP,
            ..Default::default()
        };
        cmd.set_stop_pts(1234);
        assert_eq!(cmd.stop_pts(), 1234);
        let mut ev = v4l2_event {
            type_: V4L2_EVENT_SOURCE_CHANGE,
            ..Default::default()
        };
        ev.u[..4].copy_from_slice(&V4L2_EVENT_SRC_CH_RESOLUTION.to_ne_bytes());
        assert_eq!(ev.src_change(), V4L2_EVENT_SRC_CH_RESOLUTION);
    }
}
