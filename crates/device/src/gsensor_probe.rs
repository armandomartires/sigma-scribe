//! Log-only g-sensor probe (1.4.6-probe build only; plan decision 39 /
//! S12 device check list item 1). It **only writes a log**: it finds the
//! input node that reports `MSC_RAW`, opens it read-only (no grab, so
//! nothing else loses events) and appends every event to
//! `/mnt/onboard/.adds/sigma-scribe/gsensor-probe.log`. No event reaches
//! the app; `gsensor::ENABLED` stays `false`, so rotation is unchanged.

use crate::evdev::RawEvent;
use crate::gsensor::{Tilt, EV_MSC, MSC_RAW};

/// This build carries the probe.
pub const PROBE: bool = true;
/// Where the log goes on the Kobo.
pub const LOG: &str = "/mnt/onboard/.adds/sigma-scribe/gsensor-probe.log";
/// The log stops growing at this size (a session of turning makes ~10 KB).
pub const MAX_BYTES: u64 = 2 * 1024 * 1024;

/// One log line for an event, with the current decoder's guess for
/// `MSC_RAW` values (to check against what the user did).
#[must_use]
pub fn line(t_ms: u64, e: RawEvent) -> String {
    let guess = if e.kind == EV_MSC && e.code == MSC_RAW {
        match Tilt::from_msc_raw(e.value) {
            Some(t) => format!(" guess={t:?}"),
            None => " guess=ignored(face up/down or unknown)".to_owned(),
        }
    } else {
        String::new()
    };
    format!(
        "t={t_ms}ms type=0x{:02x} code=0x{:02x} value={} (0x{:x}){guess}\n",
        e.kind, e.code, e.value, e.value
    )
}

/// The header: probe version and every `/proc/bus/input/devices` block
/// that has an `MSC` bitmap, plus the node chosen.
#[must_use]
pub fn header(version: &str, devices: &str, node: Option<&str>) -> String {
    let mut s = format!("# sigma-scribe {version} g-sensor probe (log only; rotation unchanged)\n");
    for block in devices.split("\n\n") {
        if block.lines().any(|l| l.starts_with("B: MSC=")) {
            s.push_str("# --- device with MSC bits ---\n");
            for l in block.lines() {
                s.push_str("# ");
                s.push_str(l);
                s.push('\n');
            }
        }
    }
    match node {
        Some(n) => s.push_str(&format!("# reading {n} (read-only, no grab)\n")),
        None => s.push_str("# no MSC_RAW input node found\n"),
    }
    s
}

#[cfg(target_arch = "arm")]
pub fn spawn(version: &'static str, epoch: std::time::Instant) {
    use std::io::{Read, Write};
    std::thread::spawn(move || {
        let devices = std::fs::read_to_string("/proc/bus/input/devices").unwrap_or_default();
        let node = crate::gsensor::find_node(&devices);
        let Ok(mut log) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOG)
        else {
            return;
        };
        let _ = log.write_all(header(version, &devices, node.as_deref()).as_bytes());
        let Some(node) = node else { return };
        let Ok(mut f) = std::fs::File::open(&node) else {
            let _ = writeln!(log, "# could not open {node}");
            return;
        };
        let mut buf = [0u8; crate::evdev::EVENT_SIZE_ARM32];
        while f.read_exact(&mut buf).is_ok() {
            if log.metadata().map_or(0, |m| m.len()) > MAX_BYTES {
                let _ = writeln!(log, "# size cap reached; probe stopped");
                return;
            }
            let e = crate::evdev::decode_arm32(&buf);
            let t = epoch.elapsed().as_millis() as u64;
            let _ = log.write_all(line(t, e).as_bytes());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_name_the_guess_and_ignore_other_codes() {
        let e = RawEvent {
            kind: EV_MSC,
            code: MSC_RAW,
            value: 0x19,
        };
        assert_eq!(
            line(1234, e),
            "t=1234ms type=0x04 code=0x03 value=25 (0x19) guess=Clockwise\n"
        );
        let up = RawEvent {
            kind: EV_MSC,
            code: MSC_RAW,
            value: 0x1b,
        };
        assert!(line(1, up).contains("ignored"));
        let syn = RawEvent {
            kind: 0,
            code: 0,
            value: 0,
        };
        assert!(!line(1, syn).contains("guess"));
    }

    #[test]
    fn header_lists_msc_devices_and_the_node() {
        let d = "N: Name=\"touch\"\nB: EV=b\n\nN: Name=\"mma8x5x\"\nH: Handlers=event2\nB: MSC=8\n";
        let h = header("1.4.6-probe", d, Some("/dev/input/event2"));
        assert!(h.contains("mma8x5x") && !h.contains("touch"));
        assert!(h.contains("reading /dev/input/event2"));
        assert!(header("x", "", None).contains("no MSC_RAW"));
    }

    #[test]
    fn rotation_stays_off_in_the_probe_build() {
        const { assert!(PROBE && !crate::gsensor::ENABLED) };
    }
}
