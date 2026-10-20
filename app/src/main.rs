//! `sigma-scribe`: on the Kobo, FBInk + evdev; on a desktop, a live window
//! (see `scripts/sim.sh`).

#[cfg(not(target_arch = "arm"))]
#[path = "window.rs"]
mod window;

use sigma_device::Device;
use sigma_scribe::{notes_dir, App};
use std::process::ExitCode;

fn open() -> std::io::Result<Box<dyn Device>> {
    #[cfg(target_arch = "arm")]
    {
        Ok(Box::new(sigma_device::fbink::KoboDevice::open()?))
    }
    #[cfg(not(target_arch = "arm"))]
    {
        Ok(Box::new(window::WindowDevice::open(1440, 1920)?))
    }
}

fn main() -> ExitCode {
    let mut dev = match open() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("sigma-scribe: {e}");
            return ExitCode::FAILURE;
        }
    };
    let (w, h) = dev.size();
    let mut app = App::new(w, h, notes_dir());
    match app.run(dev.as_mut()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sigma-scribe: {e}");
            ExitCode::FAILURE
        }
    }
}
