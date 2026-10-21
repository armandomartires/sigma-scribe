//! Kobo backend: FBInk for the framebuffer and EPDC refreshes, evdev for
//! touch and buttons. Built only for `target_arch = "arm"`.
#![allow(unsafe_code)]

use crate::evdev::{
    decode_arm32, Decoder, Transform, ABS_MT_POSITION_X, ABS_MT_POSITION_Y, EVENT_SIZE_ARM32,
};
use crate::otg::{OtgSwitch, CHIPIDEA_ROLE};
use crate::usbkbd::{parse_devices, KeyMapper};
use crate::{Device, Event, Refresh};
use sigma_render::{Canvas, Rect};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

extern "C" {
    fn sg_open(w: *mut u32, h: *mut u32) -> i32;
    fn sg_blit(data: *const u8, w: i32, h: i32, x: i32, y: i32) -> i32;
    fn sg_refresh(x: u32, y: u32, w: u32, h: u32, mode: i32) -> i32;
    fn sg_abs_max(fd: i32, code: i32) -> i32;
    fn sg_close() -> i32;
}

pub struct KoboDevice {
    width: u32,
    height: u32,
    events: Receiver<Event>,
    otg: OtgSwitch,
    epoch: Instant,
}

impl KoboDevice {
    /// Opens the framebuffer and every `/dev/input/event0..3`.
    ///
    /// # Errors
    /// FBInk could not open or initialise the framebuffer.
    pub fn open() -> io::Result<Self> {
        let (mut w, mut h) = (0u32, 0u32);
        // SAFETY: out-pointers to live locals; FBInk owns its own state.
        let rc = unsafe { sg_open(&mut w, &mut h) };
        if rc != 0 {
            return Err(io::Error::other(format!("fbink init failed ({rc})")));
        }
        let spec = std::env::var("SIGMA_TOUCH").unwrap_or_else(|_| "swap,mirror_x".to_owned());
        let (tx, rx) = channel();
        let epoch = Instant::now();
        // Keyboards (v0.8) are handled by the USB watcher, not the touch
        // decoder, even when one already sits on event0..3 at launch.
        let keyboards = |t: &str| -> HashSet<u32> {
            parse_devices(t)
                .into_iter()
                .filter(|d| d.keyboard)
                .filter_map(|d| d.event)
                .collect()
        };
        let at_start = std::fs::read_to_string(PROC_DEVICES)
            .map(|t| keyboards(&t))
            .unwrap_or_default();
        for n in 0..4 {
            if at_start.contains(&n) {
                continue;
            }
            let Ok(mut f) = File::open(format!("/dev/input/event{n}")) else {
                continue;
            };
            let fd = f.as_raw_fd();
            // SAFETY: fd is open for the duration of these calls.
            let (mx, my) = unsafe {
                (
                    sg_abs_max(fd, i32::from(ABS_MT_POSITION_X)),
                    sg_abs_max(fd, i32::from(ABS_MT_POSITION_Y)),
                )
            };
            let raw_max = if spec.contains("swap") {
                (my, mx)
            } else {
                (mx, my)
            };
            let mut dec = Decoder::new(Transform::from_spec(&spec, raw_max, (w as i32, h as i32)));
            let tx = tx.clone();
            thread::spawn(move || {
                let mut buf = [0u8; EVENT_SIZE_ARM32];
                while f.read_exact(&mut buf).is_ok() {
                    let now = epoch.elapsed().as_millis() as u64;
                    if let Some(e) = dec.feed(decode_arm32(&buf), now) {
                        if tx.send(e).is_err() {
                            break;
                        }
                    }
                }
            });
        }
        spawn_usb_watcher(tx);
        Ok(Self {
            otg: OtgSwitch::new(CHIPIDEA_ROLE),
            width: w,
            height: h,
            events: rx,
            epoch,
        })
    }
}

impl Device for KoboDevice {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn present(&mut self, canvas: &Canvas, region: Rect, refresh: Refresh) -> io::Result<()> {
        let (r, px) = canvas.crop(region);
        if r.w <= 0 || r.h <= 0 {
            return Ok(());
        }
        let mode = match refresh {
            Refresh::Fast => 0,
            Refresh::Partial => 1,
            Refresh::Full => 2,
        };
        // SAFETY: px holds exactly r.w*r.h bytes for the duration of the call.
        let rc = unsafe { sg_blit(px.as_ptr(), r.w, r.h, r.x, r.y) };
        if rc < 0 {
            return Err(io::Error::other(format!("fbink blit failed ({rc})")));
        }
        // SAFETY: plain integers.
        let rc = unsafe { sg_refresh(r.x as u32, r.y as u32, r.w as u32, r.h as u32, mode) };
        if rc < 0 {
            // Not fatal: the pixels are already in the framebuffer and the
            // next refresh shows them. Exiting here lost the session once.
            eprintln!(
                "sigma-scribe: warning: fbink refresh {}x{}@{},{} failed ({rc})",
                r.w, r.h, r.x, r.y
            );
        }
        Ok(())
    }

    fn usb_host(&mut self, on: bool) -> io::Result<()> {
        if on && !std::path::Path::new(CHIPIDEA_ROLE).exists() {
            // debugfs is not always mounted; this is what KOReader does.
            let _ = std::process::Command::new("mount")
                .args(["-t", "debugfs", "none", "/sys/kernel/debug"])
                .status();
        }
        let r = self.otg.set_host(on);
        eprintln!(
            "usb: host mode {} -> {:?} (role now {:?})",
            if on { "on" } else { "off" },
            r,
            self.otg.role()
        );
        r
    }

    fn next_event(&mut self) -> Option<Event> {
        // Ticks every 50 ms so a resting finger becomes a long-press.
        match self.events.recv_timeout(Duration::from_millis(50)) {
            Ok(e) => Some(e),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Some(Event::Tick {
                t: self.epoch.elapsed().as_millis() as u64,
            }),
            Err(_) => None,
        }
    }
}

impl Drop for KoboDevice {
    fn drop(&mut self) {
        // Give the port back to Nickel (USB storage) if we took it.
        if let Err(e) = self.otg.set_host(false) {
            eprintln!("usb: could not restore gadget mode: {e}");
        }
        // SAFETY: closes FBInk's own fd.
        unsafe {
            sg_close();
        }
    }
}

const PROC_DEVICES: &str = "/proc/bus/input/devices";

/// Polls `/proc/bus/input/devices` once a second for USB keyboards (hot
/// plug), opens each new one and feeds it through a `KeyMapper`. A read
/// error (unplug) closes that reader. Sends `Event::Usb(any connected)`
/// on every change. Any failure (no /proc, no permission) just means no
/// keyboard; touch and buttons are unaffected.
fn spawn_usb_watcher(tx: Sender<Event>) {
    let open: Arc<Mutex<HashSet<u32>>> = Arc::default();
    thread::spawn(move || loop {
        if let Ok(t) = std::fs::read_to_string(PROC_DEVICES) {
            for d in parse_devices(&t).into_iter().filter(|d| d.keyboard) {
                let Some(n) = d.event else { continue };
                if open.lock().map_or(true, |o| o.contains(&n)) {
                    continue;
                }
                let Ok(mut f) = File::open(format!("/dev/input/event{n}")) else {
                    continue;
                };
                eprintln!("usb: keyboard \"{}\" on event{n}", d.name);
                if let Ok(mut o) = open.lock() {
                    o.insert(n);
                }
                if tx.send(Event::Usb(true)).is_err() {
                    return;
                }
                let (tx, open) = (tx.clone(), Arc::clone(&open));
                thread::spawn(move || {
                    let mut map = KeyMapper::default();
                    let mut buf = [0u8; EVENT_SIZE_ARM32];
                    while f.read_exact(&mut buf).is_ok() {
                        if let Some(e) = map.feed(decode_arm32(&buf)) {
                            if tx.send(e).is_err() {
                                return;
                            }
                        }
                    }
                    eprintln!("usb: event{n} gone");
                    let any = open.lock().map_or(false, |mut o| {
                        o.remove(&n);
                        !o.is_empty()
                    });
                    let _ = tx.send(Event::Usb(any));
                });
            }
        }
        thread::sleep(Duration::from_secs(1));
    });
}
