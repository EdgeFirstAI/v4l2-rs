// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! Checks every constant in `uapi` against the installed kernel headers.
//!
//! The test writes a C program that includes `<linux/videodev2.h>` and
//! `<linux/v4l2-controls.h>` and prints the value of each name in
//! [`CONSTANTS`], compiles it with the system `cc`, and compares the output
//! with the Rust values. A second test parses `src/uapi.rs` and fails when a
//! `pub const` is missing from [`CONSTANTS`], so a new constant cannot go
//! unchecked.
//!
//! Without `cc` or the headers (`linux-libc-dev` on Debian and Ubuntu) the
//! comparison prints `SKIPPED` and passes, unless
//! `EDGEFIRST_V4L2_REQUIRE_UAPI_HEADERS=1` is set (CI), which turns a missing
//! toolchain, missing headers or a name the headers lack into a failure.

#![cfg(target_os = "linux")]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

#[allow(clippy::wildcard_imports)]
use edgefirst_v4l2::uapi::*;

const REQUIRE_ENV: &str = "EDGEFIRST_V4L2_REQUIRE_UAPI_HEADERS";

/// Builds the `(name, value)` table from the constant identifiers, so a name
/// and its value cannot disagree.
macro_rules! constants {
    ($($name:ident),* $(,)?) => {
        &[$((stringify!($name), $name as i128)),*]
    };
}

/// Every kernel-derived constant in `uapi`.
const CONSTANTS: &[(&str, i128)] = constants![
    VIDEO_MAX_PLANES,
    V4L2_CAP_VIDEO_CAPTURE,
    V4L2_CAP_VIDEO_OUTPUT,
    V4L2_CAP_VIDEO_CAPTURE_MPLANE,
    V4L2_CAP_VIDEO_OUTPUT_MPLANE,
    V4L2_CAP_VIDEO_M2M_MPLANE,
    V4L2_CAP_VIDEO_M2M,
    V4L2_CAP_META_CAPTURE,
    V4L2_CAP_READWRITE,
    V4L2_CAP_STREAMING,
    V4L2_CAP_IO_MC,
    V4L2_CAP_DEVICE_CAPS,
    V4L2_BUF_TYPE_VIDEO_CAPTURE,
    V4L2_BUF_TYPE_VIDEO_OUTPUT,
    V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE,
    V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
    V4L2_BUF_TYPE_META_CAPTURE,
    V4L2_BUF_TYPE_META_OUTPUT,
    V4L2_MEMORY_MMAP,
    V4L2_MEMORY_USERPTR,
    V4L2_MEMORY_OVERLAY,
    V4L2_MEMORY_DMABUF,
    V4L2_FIELD_ANY,
    V4L2_FIELD_NONE,
    V4L2_BUF_FLAG_MAPPED,
    V4L2_BUF_FLAG_QUEUED,
    V4L2_BUF_FLAG_DONE,
    V4L2_BUF_FLAG_KEYFRAME,
    V4L2_BUF_FLAG_PFRAME,
    V4L2_BUF_FLAG_BFRAME,
    V4L2_BUF_FLAG_ERROR,
    V4L2_BUF_FLAG_PREPARED,
    V4L2_BUF_FLAG_TIMESTAMP_MASK,
    V4L2_BUF_FLAG_TIMESTAMP_UNKNOWN,
    V4L2_BUF_FLAG_TIMESTAMP_MONOTONIC,
    V4L2_BUF_FLAG_TIMESTAMP_COPY,
    V4L2_BUF_FLAG_TSTAMP_SRC_MASK,
    V4L2_BUF_FLAG_TSTAMP_SRC_EOF,
    V4L2_BUF_FLAG_TSTAMP_SRC_SOE,
    V4L2_BUF_FLAG_LAST,
    V4L2_BUF_CAP_SUPPORTS_MMAP,
    V4L2_BUF_CAP_SUPPORTS_USERPTR,
    V4L2_BUF_CAP_SUPPORTS_DMABUF,
    V4L2_BUF_CAP_SUPPORTS_REQUESTS,
    V4L2_BUF_CAP_SUPPORTS_ORPHANED_BUFS,
    V4L2_BUF_CAP_SUPPORTS_M2M_HOLD_CAPTURE_BUF,
    V4L2_BUF_CAP_SUPPORTS_MMAP_CACHE_HINTS,
    V4L2_BUF_CAP_SUPPORTS_MAX_NUM_BUFFERS,
    V4L2_FMT_FLAG_COMPRESSED,
    V4L2_FMT_FLAG_EMULATED,
    V4L2_FRMSIZE_TYPE_DISCRETE,
    V4L2_FRMSIZE_TYPE_CONTINUOUS,
    V4L2_FRMSIZE_TYPE_STEPWISE,
    V4L2_FRMIVAL_TYPE_DISCRETE,
    V4L2_FRMIVAL_TYPE_CONTINUOUS,
    V4L2_FRMIVAL_TYPE_STEPWISE,
    V4L2_CAP_TIMEPERFRAME,
    V4L2_MODE_HIGHQUALITY,
    V4L2_COLORSPACE_DEFAULT,
    V4L2_COLORSPACE_SMPTE170M,
    V4L2_COLORSPACE_SMPTE240M,
    V4L2_COLORSPACE_REC709,
    V4L2_COLORSPACE_470_SYSTEM_M,
    V4L2_COLORSPACE_470_SYSTEM_BG,
    V4L2_COLORSPACE_JPEG,
    V4L2_COLORSPACE_SRGB,
    V4L2_COLORSPACE_OPRGB,
    V4L2_COLORSPACE_BT2020,
    V4L2_COLORSPACE_RAW,
    V4L2_COLORSPACE_DCI_P3,
    V4L2_XFER_FUNC_DEFAULT,
    V4L2_XFER_FUNC_709,
    V4L2_XFER_FUNC_SRGB,
    V4L2_XFER_FUNC_OPRGB,
    V4L2_XFER_FUNC_SMPTE240M,
    V4L2_XFER_FUNC_NONE,
    V4L2_XFER_FUNC_DCI_P3,
    V4L2_XFER_FUNC_SMPTE2084,
    V4L2_YCBCR_ENC_DEFAULT,
    V4L2_YCBCR_ENC_601,
    V4L2_YCBCR_ENC_709,
    V4L2_YCBCR_ENC_XV601,
    V4L2_YCBCR_ENC_XV709,
    V4L2_YCBCR_ENC_BT2020,
    V4L2_YCBCR_ENC_BT2020_CONST_LUM,
    V4L2_YCBCR_ENC_SMPTE240M,
    V4L2_QUANTIZATION_DEFAULT,
    V4L2_QUANTIZATION_FULL_RANGE,
    V4L2_QUANTIZATION_LIM_RANGE,
    V4L2_CTRL_TYPE_INTEGER,
    V4L2_CTRL_TYPE_BOOLEAN,
    V4L2_CTRL_TYPE_MENU,
    V4L2_CTRL_TYPE_BUTTON,
    V4L2_CTRL_TYPE_INTEGER64,
    V4L2_CTRL_TYPE_CTRL_CLASS,
    V4L2_CTRL_TYPE_STRING,
    V4L2_CTRL_TYPE_BITMASK,
    V4L2_CTRL_TYPE_INTEGER_MENU,
    V4L2_CTRL_TYPE_U8,
    V4L2_CTRL_TYPE_U16,
    V4L2_CTRL_TYPE_U32,
    V4L2_CTRL_FLAG_DISABLED,
    V4L2_CTRL_FLAG_GRABBED,
    V4L2_CTRL_FLAG_READ_ONLY,
    V4L2_CTRL_FLAG_UPDATE,
    V4L2_CTRL_FLAG_INACTIVE,
    V4L2_CTRL_FLAG_SLIDER,
    V4L2_CTRL_FLAG_WRITE_ONLY,
    V4L2_CTRL_FLAG_VOLATILE,
    V4L2_CTRL_FLAG_HAS_PAYLOAD,
    V4L2_CTRL_FLAG_EXECUTE_ON_WRITE,
    V4L2_CTRL_FLAG_MODIFY_LAYOUT,
    V4L2_CTRL_FLAG_DYNAMIC_ARRAY,
    V4L2_CTRL_FLAG_NEXT_CTRL,
    V4L2_CTRL_FLAG_NEXT_COMPOUND,
    V4L2_CTRL_WHICH_CUR_VAL,
    V4L2_CTRL_WHICH_DEF_VAL,
    V4L2_CTRL_WHICH_REQUEST_VAL,
    V4L2_CTRL_CLASS_USER,
    V4L2_CID_BASE,
    V4L2_CID_BRIGHTNESS,
    V4L2_CID_CONTRAST,
    V4L2_CID_SATURATION,
    V4L2_CID_HUE,
    V4L2_CID_AUTO_WHITE_BALANCE,
    V4L2_CID_EXPOSURE,
    V4L2_CID_AUTOGAIN,
    V4L2_CID_GAIN,
    V4L2_CID_HFLIP,
    V4L2_CID_VFLIP,
    V4L2_CID_POWER_LINE_FREQUENCY,
    V4L2_CID_WHITE_BALANCE_TEMPERATURE,
    V4L2_CID_SHARPNESS,
    V4L2_CID_ROTATE,
    V4L2_CID_MIN_BUFFERS_FOR_CAPTURE,
    V4L2_CID_MIN_BUFFERS_FOR_OUTPUT,
    V4L2_CTRL_CLASS_CAMERA,
    V4L2_CID_CAMERA_CLASS_BASE,
    V4L2_CID_EXPOSURE_AUTO,
    V4L2_EXPOSURE_AUTO,
    V4L2_EXPOSURE_MANUAL,
    V4L2_EXPOSURE_SHUTTER_PRIORITY,
    V4L2_EXPOSURE_APERTURE_PRIORITY,
    V4L2_CID_EXPOSURE_ABSOLUTE,
    V4L2_CID_EXPOSURE_AUTO_PRIORITY,
    V4L2_CTRL_CLASS_CODEC,
    V4L2_CID_CODEC_BASE,
    V4L2_CID_MPEG_VIDEO_GOP_SIZE,
    V4L2_CID_MPEG_VIDEO_BITRATE_MODE,
    V4L2_MPEG_VIDEO_BITRATE_MODE_VBR,
    V4L2_MPEG_VIDEO_BITRATE_MODE_CBR,
    V4L2_CID_MPEG_VIDEO_BITRATE,
    V4L2_CID_MPEG_VIDEO_HEADER_MODE,
    V4L2_MPEG_VIDEO_HEADER_MODE_SEPARATE,
    V4L2_MPEG_VIDEO_HEADER_MODE_JOINED_WITH_1ST_FRAME,
    V4L2_CID_MPEG_VIDEO_REPEAT_SEQ_HEADER,
    V4L2_CID_MPEG_VIDEO_FORCE_KEY_FRAME,
    V4L2_CID_MPEG_VIDEO_H264_I_PERIOD,
    V4L2_CID_MPEG_VIDEO_H264_LEVEL,
    V4L2_CID_MPEG_VIDEO_H264_PROFILE,
    V4L2_CID_MPEG_VIDEO_H264_MIN_QP,
    V4L2_CID_MPEG_VIDEO_H264_MAX_QP,
    V4L2_CID_MPEG_VIDEO_HEVC_PROFILE,
    V4L2_CID_MPEG_VIDEO_HEVC_LEVEL,
    V4L2_EVENT_ALL,
    V4L2_EVENT_VSYNC,
    V4L2_EVENT_EOS,
    V4L2_EVENT_CTRL,
    V4L2_EVENT_FRAME_SYNC,
    V4L2_EVENT_SOURCE_CHANGE,
    V4L2_EVENT_MOTION_DET,
    V4L2_EVENT_SRC_CH_RESOLUTION,
    V4L2_EVENT_SUB_FL_SEND_INITIAL,
    V4L2_EVENT_SUB_FL_ALLOW_FEEDBACK,
    V4L2_EVENT_CTRL_CH_VALUE,
    V4L2_EVENT_CTRL_CH_FLAGS,
    V4L2_EVENT_CTRL_CH_RANGE,
    V4L2_EVENT_CTRL_CH_DIMENSIONS,
    V4L2_SEL_TGT_CROP,
    V4L2_SEL_TGT_CROP_DEFAULT,
    V4L2_SEL_TGT_CROP_BOUNDS,
    V4L2_SEL_TGT_NATIVE_SIZE,
    V4L2_SEL_TGT_COMPOSE,
    V4L2_SEL_TGT_COMPOSE_DEFAULT,
    V4L2_SEL_TGT_COMPOSE_BOUNDS,
    V4L2_SEL_TGT_COMPOSE_PADDED,
    V4L2_SEL_FLAG_GE,
    V4L2_SEL_FLAG_LE,
    V4L2_SEL_FLAG_KEEP_CONFIG,
    V4L2_ENC_CMD_START,
    V4L2_ENC_CMD_STOP,
    V4L2_ENC_CMD_PAUSE,
    V4L2_ENC_CMD_RESUME,
    V4L2_ENC_CMD_STOP_AT_GOP_END,
    V4L2_DEC_CMD_START,
    V4L2_DEC_CMD_STOP,
    V4L2_DEC_CMD_PAUSE,
    V4L2_DEC_CMD_RESUME,
    V4L2_DEC_CMD_FLUSH,
    V4L2_DEC_CMD_START_MUTE_AUDIO,
    V4L2_DEC_CMD_PAUSE_TO_BLACK,
    V4L2_DEC_CMD_STOP_TO_BLACK,
    V4L2_DEC_CMD_STOP_IMMEDIATELY,
    V4L2_PIX_FMT_GREY,
    V4L2_PIX_FMT_RGB565,
    V4L2_PIX_FMT_RGB24,
    V4L2_PIX_FMT_BGR24,
    V4L2_PIX_FMT_XRGB32,
    V4L2_PIX_FMT_ARGB32,
    V4L2_PIX_FMT_XBGR32,
    V4L2_PIX_FMT_ABGR32,
    V4L2_PIX_FMT_RGBA32,
    V4L2_PIX_FMT_YUYV,
    V4L2_PIX_FMT_UYVY,
    V4L2_PIX_FMT_YVYU,
    V4L2_PIX_FMT_VYUY,
    V4L2_PIX_FMT_YUV24,
    V4L2_PIX_FMT_YUV32,
    V4L2_PIX_FMT_NV12,
    V4L2_PIX_FMT_NV21,
    V4L2_PIX_FMT_NV16,
    V4L2_PIX_FMT_NV61,
    V4L2_PIX_FMT_NV24,
    V4L2_PIX_FMT_NV12M,
    V4L2_PIX_FMT_NV21M,
    V4L2_PIX_FMT_NV16M,
    V4L2_PIX_FMT_YUV420,
    V4L2_PIX_FMT_YUV420M,
    V4L2_PIX_FMT_YUV444M,
    V4L2_PIX_FMT_SBGGR8,
    V4L2_PIX_FMT_SRGGB10,
    V4L2_PIX_FMT_SBGGR12,
    V4L2_PIX_FMT_MJPEG,
    V4L2_PIX_FMT_JPEG,
    V4L2_PIX_FMT_H264,
    V4L2_PIX_FMT_HEVC,
];

fn required() -> bool {
    std::env::var_os(REQUIRE_ENV).is_some()
}

fn skip(why: &str) {
    if required() {
        panic!("the kernel header check is required ({REQUIRE_ENV} is set) but {why}");
    }
    eprintln!("SKIPPED: uapi constants not checked against the kernel headers: {why}");
}

/// Names of the `pub const` items in `src/uapi.rs`.
fn declared_names() -> BTreeSet<String> {
    let source = include_str!("../src/uapi.rs");
    source
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("pub const "))
        .filter(|rest| !rest.starts_with("fn "))
        .filter_map(|rest| rest.split(':').next())
        .map(|name| name.trim().to_owned())
        .collect()
}

#[test]
fn every_uapi_constant_is_checked() {
    let listed: BTreeSet<String> = CONSTANTS.iter().map(|(n, _)| (*n).to_owned()).collect();
    assert_eq!(
        listed.len(),
        CONSTANTS.len(),
        "CONSTANTS lists a name twice"
    );
    let declared = declared_names();
    let unlisted: Vec<_> = declared.difference(&listed).collect();
    assert!(
        unlisted.is_empty(),
        "constants in src/uapi.rs missing from CONSTANTS in tests/uapi_headers.rs: {unlisted:?}"
    );
}

/// Writes the C program. Names in `absent` print `MISSING` instead of their
/// value, so the program still compiles when the headers lack them.
fn c_program(absent: &BTreeSet<String>) -> String {
    let mut c = String::from(
        "#include <stdio.h>\n\
         #include <linux/videodev2.h>\n\
         #include <linux/v4l2-controls.h>\n\
         int main(void) {\n",
    );
    for (name, _) in CONSTANTS {
        if absent.contains(*name) {
            writeln!(c, "    puts(\"{name} MISSING\");").unwrap();
        } else {
            writeln!(c, "    printf(\"{name} %lld\\n\", (long long)({name}));").unwrap();
        }
    }
    c.push_str("    return 0;\n}\n");
    c
}

/// Identifiers the compiler reports as undeclared, in gcc's or clang's
/// wording under the C locale.
fn undeclared(stderr: &str) -> BTreeSet<String> {
    let known: BTreeSet<&str> = CONSTANTS.iter().map(|(n, _)| *n).collect();
    stderr
        .lines()
        .filter(|l| l.contains("undeclared"))
        .filter_map(|l| {
            let start = l.find('\'')? + 1;
            let len = l[start..].find('\'')?;
            Some(&l[start..start + len])
        })
        .filter(|name| known.contains(name))
        .map(str::to_owned)
        .collect()
}

enum Build {
    Built,
    NoCompiler(String),
    NoHeaders(String),
    Undeclared(BTreeSet<String>),
}

fn build(dir: &Path, absent: &BTreeSet<String>) -> Build {
    let src = dir.join("uapi_headers.c");
    let bin = dir.join("uapi_headers");
    std::fs::write(&src, c_program(absent)).expect("write the C program");
    let out = match Command::new("cc")
        .env("LC_ALL", "C")
        .arg("-std=gnu11")
        .arg("-o")
        .arg(&bin)
        .arg(&src)
        .output()
    {
        Ok(out) => out,
        Err(e) => return Build::NoCompiler(format!("`cc` could not be run ({e})")),
    };
    if out.status.success() {
        return Build::Built;
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.contains("No such file") || stderr.contains("file not found") {
        return Build::NoHeaders(format!("a header is not installed:\n{stderr}"));
    }
    let names = undeclared(&stderr);
    assert!(
        !names.is_empty() && names.is_disjoint(absent),
        "the C program failed to compile:\n{stderr}"
    );
    Build::Undeclared(names)
}

#[test]
fn uapi_constants_match_kernel_headers() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let mut absent = BTreeSet::new();
    loop {
        match build(dir, &absent) {
            Build::Built => break,
            Build::NoCompiler(why) | Build::NoHeaders(why) => return skip(&why),
            Build::Undeclared(names) => absent.extend(names),
        }
    }

    let out = Command::new(dir.join("uapi_headers"))
        .output()
        .expect("run the C program");
    assert!(out.status.success(), "the C program failed: {out:?}");
    let stdout = String::from_utf8(out.stdout).expect("ASCII output");
    let header: BTreeMap<&str, &str> = stdout.lines().filter_map(|l| l.split_once(' ')).collect();

    let mut mismatched = Vec::new();
    let mut missing = Vec::new();
    for &(name, value) in CONSTANTS {
        match header.get(name) {
            Some(&"MISSING") => missing.push(name),
            Some(v) => {
                let expected: i128 = v.parse().expect("integer value");
                if expected != value {
                    mismatched.push(format!("{name}: uapi.rs {value:#x}, headers {expected:#x}"));
                }
            }
            None => panic!("the C program printed nothing for {name}"),
        }
    }
    assert!(
        mismatched.is_empty(),
        "uapi.rs disagrees with the kernel headers:\n  {}",
        mismatched.join("\n  ")
    );
    if !missing.is_empty() {
        let why = format!("the installed headers lack {missing:?}");
        if required() {
            panic!("{why} ({REQUIRE_ENV} is set); install newer kernel headers");
        }
        eprintln!("WARNING: {why}; those constants were not checked");
    }
    eprintln!(
        "checked {} uapi constants against the kernel headers",
        CONSTANTS.len() - missing.len()
    );
}
