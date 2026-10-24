//! v1.13.1: the splash screen is on the panel before the app has loaded,
//! in the saved paper and rotation, and it does not touch the notes.

use sigma_device::desktop::DesktopDevice;
use sigma_device::Refresh;

fn root(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-splash-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_splash_shows_the_name_in_paper_and_ink() {
    let d = root("day");
    let mut dev = DesktopDevice::new(1440, 1920);
    let _ = sigma_scribe::Splash::show(&mut dev, &d);
    assert_eq!(dev.refreshes.last().map(|r| r.1), Some(Refresh::Full));
    let ink = dev.panel.pixels.iter().filter(|&&p| p < 100).count();
    assert!(ink > 2000, "text drawn: {ink}");
    assert_eq!(dev.panel.pixels[0], 240, "day paper");
    assert!(
        std::fs::read_dir(&d).unwrap().next().is_none(),
        "no files written"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_splash_follows_night_and_rotation() {
    let d = root("night");
    std::fs::write(d.join(".sigma-settings"), "paper=night\nrotation=90\n").unwrap();
    let mut dev = DesktopDevice::new(1440, 1920);
    let _ = sigma_scribe::Splash::show(&mut dev, &d);
    assert_eq!(dev.panel.pixels[0], 17, "night paper");
    assert!(dev.panel.pixels.iter().filter(|&&p| p > 150).count() > 2000);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_bar_fills_from_the_left_and_only_the_bar_refreshes() {
    let d = root("bar");
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut sp = sigma_scribe::Splash::show(&mut dev, &d);
    let dark = |dev: &DesktopDevice| dev.panel.pixels.iter().filter(|&&p| p < 100).count();
    let empty = dark(&dev);
    sp.progress(&mut dev, 0.5);
    let half = dark(&dev);
    sp.progress(&mut dev, 1.0);
    let full = dark(&dev);
    assert!(empty < half && half < full, "{empty} < {half} < {full}");
    let (r, _) = *dev.refreshes.last().unwrap();
    assert!(r.w < 900 && r.h < 100, "only the bar: {r:?}");
    let _ = std::fs::remove_dir_all(&d);
}
