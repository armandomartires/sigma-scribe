//! Kobo backend: FBInk for the framebuffer and EPDC refreshes, evdev for
//! touch and buttons. Built only for `target_arch = "arm"`.
#![allow(unsafe_code)]

use crate::evdev::{
    decode_arm32, Decoder, Transform, ABS_MT_POSITION_X, ABS_MT_POSITION_Y, EVENT_SIZE_ARM32,
};
use crate::otg::{OtgSwitch, CHIPIDEA_ROLE};
use crate::pointer::PointerDecoder;
use crate::usbkbd::{parse_devices, KeyMapper};
use crate::{Device, Event, Refresh};
use sigma_render::{Canvas, Rect};
use std::collections::{HashMap, HashSet};
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
    fn sg_wait() -> i32;
}

pub struct KoboDevice {
    pointer_updates: u64,
    width: u32,
    height: u32,
    events: Receiver<Event>,
    otg: OtgSwitch,
    epoch: Instant,
    gsensor: bool,
    /// The front light (v1.12), when sysfs shows one.
    light: Option<crate::light::Light>,
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
                .filter(|d| d.keyboard || d.pointer)
                .filter_map(|d| d.event)
                .collect()
        };
        let at_start = std::fs::read_to_string(PROC_DEVICES)
            .map(|t| keyboards(&t))
            .unwrap_or_default();
        // The accelerometer (v1.12) is read by its own thread, not the touch decoder.
        let accel = std::fs::read_to_string(PROC_DEVICES)
            .ok()
            .and_then(|t| crate::gsensor::find_node(&t));
        let accel_n: Option<u32> = accel
            .as_deref()
            .and_then(|p| p.strip_prefix("/dev/input/event"))
            .and_then(|n| n.parse().ok());
        let mut gsensor = false;
        if crate::gsensor::ENABLED {
            if let Some(path) = accel.as_deref() {
                if let Ok(f) = File::open(path) {
                    spawn_accel_reader(f, tx.clone(), epoch);
                    gsensor = true;
                }
            }
        }
        for n in 0..4 {
            if at_start.contains(&n) || (gsensor && accel_n == Some(n)) {
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
        spawn_usb_watcher(tx, epoch);
        Ok(Self {
            otg: OtgSwitch::new(CHIPIDEA_ROLE),
            width: w,
            height: h,
            events: rx,
            epoch,
            pointer_updates: 0,
            gsensor,
            light: open_light(),
        })
    }
}

/// Finds the front light and logs what sysfs shows, so a wrong pick is easy
/// to see in the log (v1.12).
fn open_light() -> Option<crate::light::Light> {
    let root = std::path::Path::new(crate::light::ROOT);
    for line in crate::light::describe(root) {
        eprintln!("light: sysfs {line}");
    }
    let l = crate::light::Light::find(root);
    match &l {
        Some(l) => eprintln!("light: using {} (step {:?})", l.name(), l.step()),
        None => eprintln!("light: no front light found"),
    }
    l
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
        let mode = refresh.fbink_mode();
        let started = std::time::Instant::now();
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
        // v1.0.3 (S10.3-D2): never wait for completion here; this runs on
        // the input loop. Log the submit time: every 20th pointer move and
        // every cleanup pass.
        let log = match refresh {
            Refresh::Pointer => {
                self.pointer_updates += 1;
                self.pointer_updates % 20 == 1
            }
            Refresh::PointerCleanup => true,
            _ => false,
        };
        if log {
            eprintln!(
                "perf pointer: {} {}x{} submit {} ms",
                refresh.perf_name(),
                r.w,
                r.h,
                started.elapsed().as_millis()
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

    fn light_step(&mut self) -> Option<u8> {
        self.light.as_ref()?.step()
    }

    fn set_light(&mut self, step: u8) -> io::Result<()> {
        let l = self
            .light
            .as_ref()
            .ok_or_else(|| io::Error::other("no front light"))?;
        let r = l.set_step(step);
        eprintln!("light: {} -> step {step}: {r:?}", l.name());
        r
    }

    fn has_gsensor(&self) -> bool {
        self.gsensor
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

/// Reads the accelerometer's X/Y/Z, and on every report that changes the
/// way the device is held sends [`Event::Tilt`] (flat / diagonal sends
/// nothing, so the last rotation stays).
fn spawn_accel_reader(mut f: File, tx: Sender<Event>, epoch: Instant) {
    use crate::gsensor::{Tilt, ABS_X, ABS_Y, EV_ABS};
    thread::spawn(move || {
        let (mut x, mut y) = (0, 0);
        let mut last: Option<Tilt> = None;
        let mut buf = [0u8; EVENT_SIZE_ARM32];
        while f.read_exact(&mut buf).is_ok() {
            let e = decode_arm32(&buf);
            if e.kind == EV_ABS && e.code == ABS_X {
                x = e.value;
            } else if e.kind == EV_ABS && e.code == ABS_Y {
                y = e.value;
            } else if e.kind == 0 {
                if let Some(tilt) = Tilt::from_accel(x, y) {
                    if last != Some(tilt) {
                        last = Some(tilt);
                        let t = epoch.elapsed().as_millis() as u64;
                        if tx.send(Event::Tilt { tilt, t }).is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });
}

const PROC_DEVICES: &str = "/proc/bus/input/devices";

/// Polls `/proc/bus/input/devices` once a second for USB keyboards and
/// pointers (hot plug), opens each new one and feeds it through a
/// `KeyMapper` and, for pointers, a `PointerDecoder` (a combined device
/// such as a Unifying receiver gets both from one reader). A read error
/// (unplug) closes that reader. Sends `Usb(any keyboard)` and
/// `PointerPlug(any pointer)` on every change. Any failure (no /proc, no
/// permission) just means no device; touch and buttons are unaffected.
fn spawn_usb_watcher(tx: Sender<Event>, epoch: Instant) {
    // eventN -> (keyboard, pointer)
    let open: Arc<Mutex<HashMap<u32, (bool, bool)>>> = Arc::default();
    thread::spawn(move || loop {
        if let Ok(t) = std::fs::read_to_string(PROC_DEVICES) {
            for d in parse_devices(&t)
                .into_iter()
                .filter(|d| d.keyboard || d.pointer)
            {
                let Some(n) = d.event else { continue };
                if open.lock().map_or(true, |o| o.contains_key(&n)) {
                    continue;
                }
                let Ok(mut f) = File::open(format!("/dev/input/event{n}")) else {
                    continue;
                };
                let (kbd, ptr) = (d.keyboard, d.pointer);
                eprintln!(
                    "usb: \"{}\" on event{n} (keyboard {kbd}, pointer {ptr})",
                    d.name
                );
                if let Ok(mut o) = open.lock() {
                    o.insert(n, (kbd, ptr));
                }
                if (kbd && tx.send(Event::Usb(true)).is_err())
                    || (ptr && tx.send(Event::PointerPlug(true)).is_err())
                {
                    return;
                }
                let (tx, open) = (tx.clone(), Arc::clone(&open));
                thread::spawn(move || {
                    let mut map = KeyMapper::default();
                    let mut mouse = PointerDecoder::default();
                    let mut buf = [0u8; EVENT_SIZE_ARM32];
                    while f.read_exact(&mut buf).is_ok() {
                        let raw = decode_arm32(&buf);
                        let now = epoch.elapsed().as_millis() as u64;
                        let mut out = Vec::new();
                        if kbd {
                            out.extend(map.feed(raw));
                        }
                        if ptr {
                            out.extend(mouse.feed(raw, now));
                        }
                        for e in out {
                            if tx.send(e).is_err() {
                                return;
                            }
                        }
                    }
                    eprintln!("usb: event{n} gone");
                    let (k, p) = open.lock().map_or((false, false), |mut o| {
                        o.remove(&n);
                        (o.values().any(|v| v.0), o.values().any(|v| v.1))
                    });
                    if kbd {
                        let _ = tx.send(Event::Usb(k));
                    }
                    if ptr {
                        let _ = tx.send(Event::PointerPlug(p));
                    }
                });
            }
        }
        thread::sleep(Duration::from_secs(1));
    });
}
