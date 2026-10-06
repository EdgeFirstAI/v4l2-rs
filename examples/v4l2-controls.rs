// SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
// SPDX-License-Identifier: Apache-2.0

//! Lists the controls of a V4L2 node with their ranges and current values,
//! after applying any `name=value` or `0xID=value` assignments given.
//!
//! ```sh
//! cargo run --example v4l2-controls -- /dev/video0
//! cargo run --example v4l2-controls -- /dev/video0 brightness=128 0x00980914=1
//! ```
//!
//! A name matches the driver's control name in lower case with spaces as
//! underscores, so `Exposure Time, Absolute` is `exposure_time,_absolute`.

#[cfg(target_os = "linux")]
fn main() {
    use edgefirst_v4l2::controls::{self, ControlInfo, ControlType, ControlValue};
    use edgefirst_v4l2::device::Device;

    fn key(name: &str) -> String {
        name.to_lowercase().replace(' ', "_")
    }

    fn parse(info: &ControlInfo, text: &str) -> Option<ControlValue> {
        match info.control_type {
            ControlType::Integer64 => text.parse().ok().map(ControlValue::Integer64),
            ControlType::String => Some(ControlValue::String(text.to_owned())),
            _ if info.flags.has_payload() => None,
            _ => text.parse().ok().map(ControlValue::Integer),
        }
    }

    fn show(value: &ControlValue) -> String {
        match value {
            ControlValue::Integer(v) => v.to_string(),
            ControlValue::Integer64(v) => v.to_string(),
            ControlValue::String(s) => format!("{s:?}"),
            ControlValue::Payload(b) => format!("[{} bytes]", b.len()),
        }
    }

    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: v4l2-controls <device> [name=value | 0xID=value]...");
        std::process::exit(2);
    };
    let dev = match Device::open(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1);
        }
    };
    let all = match controls::query_all(&dev) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1);
        }
    };

    let mut failed = false;
    for assignment in args {
        let Some((name, text)) = assignment.split_once('=') else {
            eprintln!("{assignment}: expected name=value");
            failed = true;
            continue;
        };
        let id = name
            .strip_prefix("0x")
            .and_then(|h| u32::from_str_radix(h, 16).ok());
        let Some(info) = all
            .iter()
            .find(|c| Some(c.id) == id || key(&c.name) == key(name))
        else {
            eprintln!("{name}: no such control");
            failed = true;
            continue;
        };
        let Some(value) = parse(info, text) else {
            eprintln!(
                "{name}: cannot set {text:?} on a {:?} control",
                info.control_type
            );
            failed = true;
            continue;
        };
        match controls::set(&dev, info, &value) {
            Ok(applied) => println!("{} = {} (applied)", info.name, show(&applied)),
            Err(e) => {
                eprintln!("{}: {e}", info.name);
                failed = true;
            }
        }
    }

    for info in &all {
        if info.control_type == ControlType::CtrlClass {
            println!("\n{}", info.name);
            continue;
        }
        let value = if info.flags.is_write_only() {
            String::from("(write-only)")
        } else {
            controls::get(&dev, info).map_or_else(|e| format!("({e})"), |v| show(&v))
        };
        let mut flags = Vec::new();
        if info.flags.is_read_only() {
            flags.push("read-only");
        }
        if info.flags.is_inactive() {
            flags.push("inactive");
        }
        if info.flags.is_volatile() {
            flags.push("volatile");
        }
        if info.flags.is_disabled() {
            flags.push("disabled");
        }
        println!(
            "  {:#010x} {:<32} {:?} [{}..{} step {} default {}] = {value}{}",
            info.id,
            key(&info.name),
            info.control_type,
            info.minimum,
            info.maximum,
            info.step,
            info.default_value,
            if flags.is_empty() {
                String::new()
            } else {
                format!(" ({})", flags.join(", "))
            }
        );
        for item in &info.menu {
            match (&item.name, item.value) {
                (Some(name), _) => println!("      {}: {name}", item.index),
                (None, Some(v)) => println!("      {}: {v}", item.index),
                _ => {}
            }
        }
    }
    if failed {
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("V4L2 is Linux only");
}
