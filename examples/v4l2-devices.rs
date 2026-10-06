// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! Lists every V4L2 video node with its capabilities, formats, frame sizes
//! and frame intervals.
//!
//! ```sh
//! cargo run --example v4l2-devices
//! ```

#[cfg(target_os = "linux")]
fn main() {
    use edgefirst_v4l2::device::{self, Device, FrameIntervals, FrameSizes, Size};
    use edgefirst_v4l2::uapi::fourcc_str;

    let nodes = match device::enumerate() {
        Ok(n) => n,
        Err(e) => {
            eprintln!("cannot list /dev: {e}");
            std::process::exit(1);
        }
    };
    if nodes.is_empty() {
        println!("no /dev/video* nodes");
    }
    for node in nodes {
        let caps = match &node.capabilities {
            Ok(c) => c,
            Err(e) => {
                println!("{}: {e}", node.path.display());
                continue;
            }
        };
        let mut kinds = Vec::new();
        if caps.is_capture() {
            kinds.push("capture");
        }
        if caps.is_output() {
            kinds.push("output");
        }
        if caps.is_m2m() {
            kinds.push("m2m");
        }
        if caps.has_streaming() {
            kinds.push("streaming");
        }
        println!(
            "{}: {} ({}, {}) [{}]",
            node.path.display(),
            caps.card,
            caps.driver,
            caps.bus_info,
            kinds.join(", ")
        );
        let Ok(dev) = Device::open(&node.path) else {
            continue;
        };
        for buf_type in [caps.capture_buf_type(), caps.output_buf_type()]
            .into_iter()
            .flatten()
        {
            let formats = dev.formats(buf_type).unwrap_or_default();
            println!("  {buf_type:?}: {} formats", formats.len());
            for f in formats {
                let tag = if f.is_compressed() {
                    " (compressed)"
                } else {
                    ""
                };
                println!("    {} {}{tag}", fourcc_str(f.fourcc), f.description);
                let sizes = match dev.frame_sizes(f.fourcc) {
                    Ok(FrameSizes::Discrete(s)) => s,
                    Ok(FrameSizes::Stepwise(r) | FrameSizes::Continuous(r)) => {
                        println!(
                            "      {}x{} to {}x{} in steps of {}x{}",
                            r.min.width,
                            r.min.height,
                            r.max.width,
                            r.max.height,
                            r.step.width,
                            r.step.height
                        );
                        continue;
                    }
                    Err(_) => continue,
                };
                for Size { width, height } in sizes {
                    let rates = match dev.frame_intervals(f.fourcc, Size { width, height }) {
                        Ok(FrameIntervals::Discrete(list)) => list
                            .iter()
                            .filter_map(|i| i.fps())
                            .map(|fps| format!("{fps:.2}"))
                            .collect::<Vec<_>>()
                            .join(", "),
                        Ok(
                            FrameIntervals::Stepwise { min, max, .. }
                            | FrameIntervals::Continuous { min, max },
                        ) => {
                            format!(
                                "{:.2} to {:.2}",
                                max.fps().unwrap_or(0.0),
                                min.fps().unwrap_or(0.0)
                            )
                        }
                        Err(_) => String::from("?"),
                    };
                    println!("      {width}x{height} @ {rates} fps");
                }
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("V4L2 is Linux only");
}
