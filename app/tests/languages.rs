//! v1.4 UI languages: the Kobo locale picks EN/FR/PT; every screen renders
//! and keeps its targets on screen in each language and the long pseudo
//! locale at 200 % in both orientations.

use sigma_core::settings::{Change, Orientation};
use sigma_device::desktop::DesktopDevice;
use sigma_device::Refresh;
use sigma_render::i18n::{self, Lang};
use sigma_scribe::App;

fn fresh(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-lang-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn the_kobo_locale_fixture_selects_french_and_is_not_modified() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/kobo-ereader-fr.conf"
    );
    let before = std::fs::read(fixture).unwrap();
    // Only this test in this binary touches the variable.
    std::env::set_var("SIGMA_KOBO_CONF", fixture);
    let dir = fresh("fixture");
    let app = App::new(1440, 1920, &dir);
    std::env::remove_var("SIGMA_KOBO_CONF");
    assert_eq!(i18n::lang(), Lang::Fr);
    assert_eq!(i18n::tr("Paper"), "Papier");
    assert!(app.about().iter().any(|(k, _)| k == "Langue"));
    assert_eq!(
        std::fs::read(fixture).unwrap(),
        before,
        "config is read-only"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn every_screen_fits_in_each_language_at_200_percent() {
    for l in [
        Lang::Fr,
        Lang::Pt,
        Lang::PseudoLong,
        Lang::PseudoRtl,
        Lang::En,
    ] {
        let dir = fresh(&format!("{l:?}"));
        let mut dev = DesktopDevice::new(1440, 1920);
        let mut app = App::new(1440, 1920, &dir);
        app.set_language(l);
        for o in [Orientation::Portrait, Orientation::Landscape] {
            app.change(Change::Orientation(o));
            let screens: [fn(&mut App); 4] = [
                |a| a.open_options(),
                |a| a.open_files(),
                |a| a.open_help(),
                |a| {
                    a.open_find();
                },
            ];
            for open in screens {
                open(&mut app);
                app.render(&mut dev, Refresh::Full).unwrap();
                let (w, h) = (app.canvas.width as i32, app.canvas.height as i32);
                let min = app.min_target();
                for (n, r) in app.touch_targets() {
                    assert!(r.w >= min && r.h >= min, "{l:?} {o:?}: {n} {r:?}");
                    assert!(
                        r.x >= 0 && r.y >= 0 && r.x + r.w <= w && r.y + r.h <= h,
                        "{l:?} {o:?}: {n} {r:?} off screen"
                    );
                }
            }
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
