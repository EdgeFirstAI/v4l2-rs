// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! `nix` bindings for every V4L2 ioctl the crate uses.
//!
//! `nix` encodes each request number from (direction, `'V'`, number,
//! `size_of` of the argument) at compile time, so each binding is paired with
//! its exact [`uapi`](crate::uapi) struct. This is the only place request
//! numbers are built. Each function is `unsafe`: the caller passes a valid
//! file descriptor and a pointer to a correctly initialised argument that
//! outlives the call.

use nix::{ioctl_read, ioctl_readwrite, ioctl_write_ptr};

#[allow(clippy::wildcard_imports)]
use crate::uapi::*;

ioctl_read!(
    /// `VIDIOC_QUERYCAP` = `_IOR('V', 0, struct v4l2_capability)`.
    vidioc_querycap,
    b'V',
    0,
    v4l2_capability
);
ioctl_readwrite!(
    /// `VIDIOC_ENUM_FMT` = `_IOWR('V', 2, struct v4l2_fmtdesc)`.
    vidioc_enum_fmt,
    b'V',
    2,
    v4l2_fmtdesc
);
ioctl_readwrite!(
    /// `VIDIOC_G_FMT` = `_IOWR('V', 4, struct v4l2_format)`.
    vidioc_g_fmt,
    b'V',
    4,
    v4l2_format
);
ioctl_readwrite!(
    /// `VIDIOC_S_FMT` = `_IOWR('V', 5, struct v4l2_format)`.
    vidioc_s_fmt,
    b'V',
    5,
    v4l2_format
);
ioctl_readwrite!(
    /// `VIDIOC_REQBUFS` = `_IOWR('V', 8, struct v4l2_requestbuffers)`.
    vidioc_reqbufs,
    b'V',
    8,
    v4l2_requestbuffers
);
ioctl_readwrite!(
    /// `VIDIOC_QUERYBUF` = `_IOWR('V', 9, struct v4l2_buffer)`.
    vidioc_querybuf,
    b'V',
    9,
    v4l2_buffer
);
ioctl_readwrite!(
    /// `VIDIOC_QBUF` = `_IOWR('V', 15, struct v4l2_buffer)`.
    vidioc_qbuf,
    b'V',
    15,
    v4l2_buffer
);
ioctl_readwrite!(
    /// `VIDIOC_EXPBUF` = `_IOWR('V', 16, struct v4l2_exportbuffer)`.
    vidioc_expbuf,
    b'V',
    16,
    v4l2_exportbuffer
);
ioctl_readwrite!(
    /// `VIDIOC_DQBUF` = `_IOWR('V', 17, struct v4l2_buffer)`.
    vidioc_dqbuf,
    b'V',
    17,
    v4l2_buffer
);
ioctl_write_ptr!(
    /// `VIDIOC_STREAMON` = `_IOW('V', 18, int)`.
    vidioc_streamon,
    b'V',
    18,
    std::os::raw::c_int
);
ioctl_write_ptr!(
    /// `VIDIOC_STREAMOFF` = `_IOW('V', 19, int)`.
    vidioc_streamoff,
    b'V',
    19,
    std::os::raw::c_int
);
ioctl_readwrite!(
    /// `VIDIOC_G_PARM` = `_IOWR('V', 21, struct v4l2_streamparm)`.
    vidioc_g_parm,
    b'V',
    21,
    v4l2_streamparm
);
ioctl_readwrite!(
    /// `VIDIOC_S_PARM` = `_IOWR('V', 22, struct v4l2_streamparm)`.
    vidioc_s_parm,
    b'V',
    22,
    v4l2_streamparm
);
ioctl_readwrite!(
    /// `VIDIOC_G_CTRL` = `_IOWR('V', 27, struct v4l2_control)`.
    vidioc_g_ctrl,
    b'V',
    27,
    v4l2_control
);
ioctl_readwrite!(
    /// `VIDIOC_S_CTRL` = `_IOWR('V', 28, struct v4l2_control)`.
    vidioc_s_ctrl,
    b'V',
    28,
    v4l2_control
);
ioctl_readwrite!(
    /// `VIDIOC_QUERYCTRL` = `_IOWR('V', 36, struct v4l2_queryctrl)`.
    vidioc_queryctrl,
    b'V',
    36,
    v4l2_queryctrl
);
ioctl_readwrite!(
    /// `VIDIOC_QUERYMENU` = `_IOWR('V', 37, struct v4l2_querymenu)`.
    vidioc_querymenu,
    b'V',
    37,
    v4l2_querymenu
);
ioctl_readwrite!(
    /// `VIDIOC_TRY_FMT` = `_IOWR('V', 64, struct v4l2_format)`.
    vidioc_try_fmt,
    b'V',
    64,
    v4l2_format
);
ioctl_readwrite!(
    /// `VIDIOC_G_EXT_CTRLS` = `_IOWR('V', 71, struct v4l2_ext_controls)`.
    vidioc_g_ext_ctrls,
    b'V',
    71,
    v4l2_ext_controls
);
ioctl_readwrite!(
    /// `VIDIOC_S_EXT_CTRLS` = `_IOWR('V', 72, struct v4l2_ext_controls)`.
    vidioc_s_ext_ctrls,
    b'V',
    72,
    v4l2_ext_controls
);
ioctl_readwrite!(
    /// `VIDIOC_TRY_EXT_CTRLS` = `_IOWR('V', 73, struct v4l2_ext_controls)`.
    vidioc_try_ext_ctrls,
    b'V',
    73,
    v4l2_ext_controls
);
ioctl_readwrite!(
    /// `VIDIOC_ENUM_FRAMESIZES` = `_IOWR('V', 74, struct v4l2_frmsizeenum)`.
    vidioc_enum_framesizes,
    b'V',
    74,
    v4l2_frmsizeenum
);
ioctl_readwrite!(
    /// `VIDIOC_ENUM_FRAMEINTERVALS` = `_IOWR('V', 75, struct v4l2_frmivalenum)`.
    vidioc_enum_frameintervals,
    b'V',
    75,
    v4l2_frmivalenum
);
ioctl_readwrite!(
    /// `VIDIOC_ENCODER_CMD` = `_IOWR('V', 77, struct v4l2_encoder_cmd)`.
    vidioc_encoder_cmd,
    b'V',
    77,
    v4l2_encoder_cmd
);
ioctl_readwrite!(
    /// `VIDIOC_TRY_ENCODER_CMD` = `_IOWR('V', 78, struct v4l2_encoder_cmd)`.
    vidioc_try_encoder_cmd,
    b'V',
    78,
    v4l2_encoder_cmd
);
ioctl_read!(
    /// `VIDIOC_DQEVENT` = `_IOR('V', 89, struct v4l2_event)`.
    vidioc_dqevent,
    b'V',
    89,
    v4l2_event
);
ioctl_write_ptr!(
    /// `VIDIOC_SUBSCRIBE_EVENT` = `_IOW('V', 90, struct v4l2_event_subscription)`.
    vidioc_subscribe_event,
    b'V',
    90,
    v4l2_event_subscription
);
ioctl_write_ptr!(
    /// `VIDIOC_UNSUBSCRIBE_EVENT` = `_IOW('V', 91, struct v4l2_event_subscription)`.
    vidioc_unsubscribe_event,
    b'V',
    91,
    v4l2_event_subscription
);
ioctl_readwrite!(
    /// `VIDIOC_CREATE_BUFS` = `_IOWR('V', 92, struct v4l2_create_buffers)`.
    vidioc_create_bufs,
    b'V',
    92,
    v4l2_create_buffers
);
ioctl_readwrite!(
    /// `VIDIOC_PREPARE_BUF` = `_IOWR('V', 93, struct v4l2_buffer)`.
    vidioc_prepare_buf,
    b'V',
    93,
    v4l2_buffer
);
ioctl_readwrite!(
    /// `VIDIOC_G_SELECTION` = `_IOWR('V', 94, struct v4l2_selection)`.
    vidioc_g_selection,
    b'V',
    94,
    v4l2_selection
);
ioctl_readwrite!(
    /// `VIDIOC_S_SELECTION` = `_IOWR('V', 95, struct v4l2_selection)`.
    vidioc_s_selection,
    b'V',
    95,
    v4l2_selection
);
ioctl_readwrite!(
    /// `VIDIOC_DECODER_CMD` = `_IOWR('V', 96, struct v4l2_decoder_cmd)`.
    vidioc_decoder_cmd,
    b'V',
    96,
    v4l2_decoder_cmd
);
ioctl_readwrite!(
    /// `VIDIOC_TRY_DECODER_CMD` = `_IOWR('V', 97, struct v4l2_decoder_cmd)`.
    vidioc_try_decoder_cmd,
    b'V',
    97,
    v4l2_decoder_cmd
);
ioctl_readwrite!(
    /// `VIDIOC_QUERY_EXT_CTRL` = `_IOWR('V', 103, struct v4l2_query_ext_ctrl)`.
    vidioc_query_ext_ctrl,
    b'V',
    103,
    v4l2_query_ext_ctrl
);

#[cfg(test)]
mod tests {
    use super::*;
    use nix::{request_code_read, request_code_readwrite, request_code_write};
    use std::mem::size_of;

    /// Request numbers generated from linux/videodev2.h, identical on x86_64
    /// and aarch64.
    #[test]
    fn request_codes_match_videodev2() {
        let codes: &[(&str, u64, u64)] = &[
            (
                "VIDIOC_QUERYCAP",
                request_code_read!(b'V', 0, size_of::<v4l2_capability>()),
                0x80685600,
            ),
            (
                "VIDIOC_ENUM_FMT",
                request_code_readwrite!(b'V', 2, size_of::<v4l2_fmtdesc>()),
                0xc0405602,
            ),
            (
                "VIDIOC_G_FMT",
                request_code_readwrite!(b'V', 4, size_of::<v4l2_format>()),
                0xc0d05604,
            ),
            (
                "VIDIOC_S_FMT",
                request_code_readwrite!(b'V', 5, size_of::<v4l2_format>()),
                0xc0d05605,
            ),
            (
                "VIDIOC_REQBUFS",
                request_code_readwrite!(b'V', 8, size_of::<v4l2_requestbuffers>()),
                0xc0145608,
            ),
            (
                "VIDIOC_QUERYBUF",
                request_code_readwrite!(b'V', 9, size_of::<v4l2_buffer>()),
                0xc0585609,
            ),
            (
                "VIDIOC_QBUF",
                request_code_readwrite!(b'V', 15, size_of::<v4l2_buffer>()),
                0xc058560f,
            ),
            (
                "VIDIOC_EXPBUF",
                request_code_readwrite!(b'V', 16, size_of::<v4l2_exportbuffer>()),
                0xc0405610,
            ),
            (
                "VIDIOC_DQBUF",
                request_code_readwrite!(b'V', 17, size_of::<v4l2_buffer>()),
                0xc0585611,
            ),
            (
                "VIDIOC_STREAMON",
                request_code_write!(b'V', 18, size_of::<std::os::raw::c_int>()),
                0x40045612,
            ),
            (
                "VIDIOC_STREAMOFF",
                request_code_write!(b'V', 19, size_of::<std::os::raw::c_int>()),
                0x40045613,
            ),
            (
                "VIDIOC_G_PARM",
                request_code_readwrite!(b'V', 21, size_of::<v4l2_streamparm>()),
                0xc0cc5615,
            ),
            (
                "VIDIOC_S_PARM",
                request_code_readwrite!(b'V', 22, size_of::<v4l2_streamparm>()),
                0xc0cc5616,
            ),
            (
                "VIDIOC_G_CTRL",
                request_code_readwrite!(b'V', 27, size_of::<v4l2_control>()),
                0xc008561b,
            ),
            (
                "VIDIOC_S_CTRL",
                request_code_readwrite!(b'V', 28, size_of::<v4l2_control>()),
                0xc008561c,
            ),
            (
                "VIDIOC_QUERYCTRL",
                request_code_readwrite!(b'V', 36, size_of::<v4l2_queryctrl>()),
                0xc0445624,
            ),
            (
                "VIDIOC_QUERYMENU",
                request_code_readwrite!(b'V', 37, size_of::<v4l2_querymenu>()),
                0xc02c5625,
            ),
            (
                "VIDIOC_TRY_FMT",
                request_code_readwrite!(b'V', 64, size_of::<v4l2_format>()),
                0xc0d05640,
            ),
            (
                "VIDIOC_G_EXT_CTRLS",
                request_code_readwrite!(b'V', 71, size_of::<v4l2_ext_controls>()),
                0xc0205647,
            ),
            (
                "VIDIOC_S_EXT_CTRLS",
                request_code_readwrite!(b'V', 72, size_of::<v4l2_ext_controls>()),
                0xc0205648,
            ),
            (
                "VIDIOC_TRY_EXT_CTRLS",
                request_code_readwrite!(b'V', 73, size_of::<v4l2_ext_controls>()),
                0xc0205649,
            ),
            (
                "VIDIOC_ENUM_FRAMESIZES",
                request_code_readwrite!(b'V', 74, size_of::<v4l2_frmsizeenum>()),
                0xc02c564a,
            ),
            (
                "VIDIOC_ENUM_FRAMEINTERVALS",
                request_code_readwrite!(b'V', 75, size_of::<v4l2_frmivalenum>()),
                0xc034564b,
            ),
            (
                "VIDIOC_ENCODER_CMD",
                request_code_readwrite!(b'V', 77, size_of::<v4l2_encoder_cmd>()),
                0xc028564d,
            ),
            (
                "VIDIOC_TRY_ENCODER_CMD",
                request_code_readwrite!(b'V', 78, size_of::<v4l2_encoder_cmd>()),
                0xc028564e,
            ),
            (
                "VIDIOC_DQEVENT",
                request_code_read!(b'V', 89, size_of::<v4l2_event>()),
                0x80885659,
            ),
            (
                "VIDIOC_SUBSCRIBE_EVENT",
                request_code_write!(b'V', 90, size_of::<v4l2_event_subscription>()),
                0x4020565a,
            ),
            (
                "VIDIOC_UNSUBSCRIBE_EVENT",
                request_code_write!(b'V', 91, size_of::<v4l2_event_subscription>()),
                0x4020565b,
            ),
            (
                "VIDIOC_CREATE_BUFS",
                request_code_readwrite!(b'V', 92, size_of::<v4l2_create_buffers>()),
                0xc100565c,
            ),
            (
                "VIDIOC_PREPARE_BUF",
                request_code_readwrite!(b'V', 93, size_of::<v4l2_buffer>()),
                0xc058565d,
            ),
            (
                "VIDIOC_G_SELECTION",
                request_code_readwrite!(b'V', 94, size_of::<v4l2_selection>()),
                0xc040565e,
            ),
            (
                "VIDIOC_S_SELECTION",
                request_code_readwrite!(b'V', 95, size_of::<v4l2_selection>()),
                0xc040565f,
            ),
            (
                "VIDIOC_DECODER_CMD",
                request_code_readwrite!(b'V', 96, size_of::<v4l2_decoder_cmd>()),
                0xc0485660,
            ),
            (
                "VIDIOC_TRY_DECODER_CMD",
                request_code_readwrite!(b'V', 97, size_of::<v4l2_decoder_cmd>()),
                0xc0485661,
            ),
            (
                "VIDIOC_QUERY_EXT_CTRL",
                request_code_readwrite!(b'V', 103, size_of::<v4l2_query_ext_ctrl>()),
                0xc0e85667,
            ),
        ];
        for (name, ours, kernel) in codes {
            assert_eq!(ours, kernel, "{name}: 0x{ours:08x} != 0x{kernel:08x}");
        }
    }
}
