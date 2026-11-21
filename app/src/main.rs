//! `sigma-scribe`: on the Kobo (`--features kobo`), FBInk + evdev; on a
//! desktop (the default `desktop` feature), a live window (see
//! `scripts/sim.sh`).

#[cfg(all(feature = "kobo", feature = "desktop"))]
compile_error!(
    "features `kobo` and `desktop` are mutually exclusive; \
     build the device binary with --no-default-features --features kobo"
);

#[cfg(not(any(feature = "kobo", feature = "desktop")))]
compile_error!("building sigma-scribe requires the `kobo` or `desktop` feature");

#[cfg(feature = "desktop")]
#[path = "window.rs"]
mod window;

use sigma_device::Device;
use sigma_scribe::{notes_dir, App};
use std::process::ExitCode;

fn open() -> std::io::Result<Box<dyn Device>> {
    #[cfg(feature = "kobo")]
    {
        Ok(Box::new(sigma_device::fbink::KoboDevice::open()?))
    }
    #[cfg(feature = "desktop")]
    {
        Ok(Box::new(window::WindowDevice::open(1440, 1920)?))
    }
}

/// The radio: the Kobo's own, or on a desktop a pretend one so the Wi-Fi
/// menu can be tried.
fn wifi() -> sigma_device::wifi::Service {
    sigma_device::platform::current().wifi()
}

fn main() -> ExitCode {
    // A panic goes to the log file too (when logging is on); the default hook
    // still prints it to stderr, which the launcher keeps.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        sigma_core::log::file_only(
            sigma_core::log::Level::Critical,
            "panic",
            format_args!("{info}"),
        );
        default_hook(info);
    }));
    let mut dev = match open() {
        Ok(d) => d,
        Err(e) => {
            sigma_core::critical!("sigma-scribe", "{e}");
            return ExitCode::FAILURE;
        }
    };
    let notes = notes_dir();
    let mut splash = sigma_scribe::Splash::show(dev.as_mut(), &notes);
    let (w, h) = dev.size();
    splash.progress(dev.as_mut(), 0.3);
    let mut app = App::new(w, h, notes);
    app.set_wifi(wifi());
    splash.progress(dev.as_mut(), 1.0);
    match app.run(dev.as_mut()) {
        Ok(()) => {
            sigma_core::info!("app", "exit");
            ExitCode::SUCCESS
        }
        Err(e) => {
            sigma_core::critical!("sigma-scribe", "{e}");
            ExitCode::FAILURE
        }
    }
}
