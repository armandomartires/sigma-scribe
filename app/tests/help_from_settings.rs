//! Settings → Help & about opens each Help page with its text: the page
//! used to open with an empty body until Back and a second open.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Refresh};
use sigma_render::HelpPage;
use sigma_scribe::App;

fn tap(app: &mut App, dev: &mut DesktopDevice, name: &str) {
    let (x, y) = app
        .control_centre(name)
        .unwrap_or_else(|| panic!("no control {name}"));
    if let Some(r) = app.handle(Event::Tap { x, y }) {
        app.render(dev, r).unwrap();
    }
}

/// Pixels in the page's text column that differ from the paper.
fn inked(app: &App, dev: &DesktopDevice) -> usize {
    let a = app.screen.help_layout(&app.keyboard, true).area;
    let w = dev.panel.width as usize;
    let paper = dev.panel.pixels[(a.y as usize + 1) * w + 1];
    (a.y..a.y + a.h)
        .flat_map(|y| (a.x..a.x + a.w).map(move |x| y as usize * w + x as usize))
        .filter(|&i| dev.panel.pixels[i] != paper)
        .count()
}

#[test]
fn each_help_row_in_settings_opens_its_page_with_text() {
    for (row, page) in [
        ("App info", HelpPage::About),
        ("Markdown reference", HelpPage::Markdown),
        ("USB keyboard, mouse & export", HelpPage::Keyboard),
    ] {
        let dir = std::env::temp_dir().join(format!(
            "sigma-help-settings-{:?}-{}",
            page,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = App::new(1440, 1920, dir.clone());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        tap(&mut app, &mut dev, "Options");
        tap(&mut app, &mut dev, "Help & about");
        tap(&mut app, &mut dev, row);
        let h = app.help.as_ref().expect("Help is open");
        assert_eq!(h.page, page, "{row}");
        let doc = h
            .doc
            .as_ref()
            .unwrap_or_else(|| panic!("{row}: no document"));
        assert!(!doc.pages.is_empty(), "{row}: no pages");
        let n = inked(&app, &dev);
        assert!(n > 2000, "{row}: the page body is blank ({n} inked pixels)");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
