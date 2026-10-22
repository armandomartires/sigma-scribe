//! v1.5.5 (S15.5, decision 57): Settings → Language → Override system
//! language. Off (default) follows the Kobo's CurrentLocale, read-only;
//! on enables the language list and saves the choice in `.sigma-settings`.
//! One test function: the interface language is process-global.

use sigma_core::settings::{Change, Settings, UiLang};
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Refresh};
use sigma_render::i18n::{self, Lang};
use sigma_render::options::Kind;
use sigma_scribe::App;

fn fresh(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-lo-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn tap(app: &mut App, name: &str) {
    let (x, y) = app
        .control_centre(name)
        .unwrap_or_else(|| panic!("no control {name}"));
    app.handle(Event::Tap { x, y });
}

#[test]
fn override_system_language() {
    // Keys: backward-compatible defaults, round trip, bad values ignored.
    let d = Settings::default();
    assert!(!d.lang_override);
    assert_eq!(d.ui_lang, UiLang::En);
    let old = Settings::parse("paper=night\nink_boxes=off\n");
    assert!(!old.lang_override, "1.5.4 files load with the override off");
    let mut s = Settings::default();
    s.apply(Change::LangOverride(true));
    s.apply(Change::UiLang(UiLang::Pt));
    let text = s.serialize();
    assert!(text.contains("ui_lang_override=on\n") && text.contains("ui_lang=pt\n"));
    assert_eq!(Settings::parse(&text), s);
    assert_eq!(Settings::parse("ui_lang=xx\n").ui_lang, UiLang::En);

    // Off: follows the system language; the list is greyed, not a control.
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/kobo-ereader-fr.conf"
    );
    let before = std::fs::read(fixture).unwrap();
    let dir = fresh("app");
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    app.set_language(Lang::Pt);
    assert_eq!(i18n::lang(), Lang::Pt);
    tap(&mut app, "Options");
    tap(&mut app, "Language");
    app.render(&mut dev, Refresh::Full).unwrap();
    assert!(app.control_centre("Interface language").is_none(), "greyed");
    let m = app.options_model();
    let row = sigma_render::options::display_row(&m, "Interface language").unwrap();
    assert_eq!(row.kind, Kind::Muted("Português".into()));

    // On: starts from the language in use, then the list picks.
    tap(&mut app, "Override system language/On");
    assert!(app.settings.lang_override);
    assert_eq!(app.settings.ui_lang, UiLang::Pt);
    assert_eq!(i18n::lang(), Lang::Pt, "turning it on changes nothing");
    tap(&mut app, "Interface language");
    tap(&mut app, "English");
    assert_eq!(i18n::lang(), Lang::En);
    assert_eq!(app.settings.ui_lang, UiLang::En);

    // A changed system language does not win while the override is on.
    app.set_language(Lang::Fr);
    assert_eq!(i18n::lang(), Lang::En);

    // Saved, and read back by a restarted app.
    app.render(&mut dev, Refresh::Full).unwrap();
    drop(app);
    let saved = std::fs::read_to_string(dir.join("sigma-scribe/.sigma-settings"))
        .or_else(|_| std::fs::read_to_string(dir.join(".sigma-settings")))
        .unwrap_or_default();
    if !saved.is_empty() {
        assert!(saved.contains("ui_lang_override=on") && saved.contains("ui_lang=en"));
    }
    let mut app = App::new(1440, 1920, &dir);
    app.set_language(Lang::Pt);
    assert!(app.settings.lang_override);
    assert_eq!(i18n::lang(), Lang::En, "the choice survives a restart");

    // Off again: back to the system language; the choice is kept.
    tap(&mut app, "Options");
    tap(&mut app, "Language");
    tap(&mut app, "Override system language/Off");
    assert_eq!(i18n::lang(), Lang::Pt);
    assert_eq!(app.settings.ui_lang, UiLang::En);
    assert_eq!(
        std::fs::read(fixture).unwrap(),
        before,
        "CurrentLocale never written"
    );
    i18n::set_lang(Lang::En);
    let _ = std::fs::remove_dir_all(dir);
}
