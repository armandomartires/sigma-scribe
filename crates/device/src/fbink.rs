//! Kobo backend: FBInk for the framebuffer and EPDC refreshes, evdev for
//! touch and buttons. Built only for `target_arch = "arm"`.
#![allow(unsafe_code)]

use crate::evdev::{
    decode_arm32, Decoder, Transform, ABS_MT_POSITION_X, ABS_MT_POSITION_Y, EVENT_SIZE_ARM32,
};
use crate::{Device, Event, Refresh};
use sigma_render::{Canvas, Rect};
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::sync::mpsc::{channel, Receiver};
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
        for n in 0..4 {
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
        Ok(Self {
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
            return Err(io::Error::other(format!("fbink refresh failed ({rc})")));
        }
        Ok(())
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
        // SAFETY: closes FBInk's own fd.
        unsafe {
            sg_close();
        }
    }
}
