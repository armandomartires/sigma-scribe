//! S23 wiki-link autocomplete in the app: when the bar shows, what a pick
//! writes, and where it stays off.

use sigma_core::layouts::KeyLayout;
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Refresh};
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-suggest-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut app = App::new(1440, 1920, d.clone());
        for (n, body) in [
            ("work.md", ""),
            ("chapter-01.md", "# Intro\n\nText.\n\n## Details\n\nMore.\n"),
            ("chapter-02.md", "# Two\n"),
            ("zeta.md", "# Zeta\n"),
        ] {
            std::fs::write(d.join(n), body).unwrap();
        }
        app.open(&sigma_core::NotePath::parse("work.md").unwrap());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev }
    }

    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(k) = r {
            self.app.render(&mut self.dev, k).unwrap();
        }
        r
    }

    fn type_str(&mut self, s: &str) {
        for c in s.chars() {
            self.send(Event::Key(HwKey::Char(c)));
        }
    }

    fn key(&mut self, k: HwKey) -> Option<Refresh> {
        self.send(Event::Key(k))
    }

    fn tap_on(&mut self, label: &str) -> Option<Refresh> {
        let (x, y) = self.app.control_centre(label).expect(label);
        self.send(Event::Tap { x, y })
    }

    /// A look with no input, to let the app notice a state change.
    fn tick(&mut self) {
        self.send(Event::Tick { t: 5000 });
    }

    fn labels(&self) -> Vec<String> {
        self.app
            .suggest
            .as_ref()
            .map(|s| s.labels.clone())
            .unwrap_or_default()
    }
}

#[test]
fn typing_after_open_brackets_shows_matching_notes() {
    let mut r = Rig::new("show");
    r.type_str("see [[ch");
    // The seed notes add `drafts/chapter-01` and `scratch`.
    assert_eq!(&r.labels()[..2], ["chapter-01", "chapter-02"]);
    let n = r.labels().len();
    assert!(r.app.control_centre("Suggest0").is_some());
    assert!(r.app.control_centre("Suggest1").is_some());
    assert!(r.app.control_centre(&format!("Suggest{n}")).is_none());
    // Every chip is at least the minimum target on both sides.
    let min = r.app.min_target();
    let mut shown = 0;
    for (name, rect) in r.app.touch_targets() {
        if name.starts_with("Suggest") {
            shown += 1;
            assert!(
                rect.w >= min && rect.h >= min,
                "{name} {}x{}",
                rect.w,
                rect.h
            );
        }
    }
    assert!(shown >= 2);
    // A closed link, or no link, shows nothing.
    r.type_str("]] and ch");
    assert!(r.app.suggest.is_none());
}

#[test]
fn tapping_a_chip_completes_the_link_and_one_undo_takes_it_back() {
    let mut r = Rig::new("tap");
    r.type_str("[[ch");
    assert_eq!(r.tap_on("Suggest0"), Some(Refresh::Partial));
    assert_eq!(r.app.buffer.text(), "[[chapter-01]]");
    assert_eq!(r.app.buffer.caret(), "[[chapter-01]]".len());
    assert!(r.app.suggest.is_none(), "closed until the next edit");
    assert!(r.app.buffer.undo());
    assert_eq!(r.app.buffer.text(), "[[ch");
    // The bar is back at `[[ch`.
    r.tick();
    assert_eq!(&r.labels()[..2], ["chapter-01", "chapter-02"]);
}

#[test]
fn a_pick_is_saved() {
    let mut r = Rig::new("save");
    r.type_str("[[ch");
    r.tap_on("Suggest1");
    assert_eq!(r.app.buffer.text(), "[[chapter-02]]");
    r.app.open(&sigma_core::NotePath::parse("zeta.md").unwrap());
    let saved = std::fs::read_to_string(r.app.library.root().join("work.md")).unwrap();
    assert_eq!(saved, "[[chapter-02]]");
}

#[test]
fn after_a_hash_the_notes_headings_are_offered() {
    let mut r = Rig::new("head");
    r.type_str("[[chapter-01#");
    assert_eq!(r.labels(), ["Intro", "Details"]);
    r.type_str("de");
    assert_eq!(r.labels(), ["Details"]);
    r.key(HwKey::Backspace);
    r.key(HwKey::Backspace);
    r.tap_on("Suggest0");
    assert_eq!(r.app.buffer.text(), "[[chapter-01#Intro]]");
    assert_eq!(r.app.buffer.caret(), r.app.buffer.text().len());
    // A note that does not exist: no bar.
    r.type_str(" [[nowhere#");
    assert!(r.app.suggest.is_none());
    // A bare `#` is this note's own headings.
    let mut r = Rig::new("head-self");
    r.type_str("# Mine\n[[#");
    assert_eq!(r.labels(), ["Mine"]);
}

#[test]
fn an_existing_closing_pair_is_not_doubled() {
    let mut r = Rig::new("close");
    r.type_str("[[]]");
    r.key(HwKey::Left);
    r.key(HwKey::Left);
    r.type_str("ch");
    assert!(r.app.suggest.is_some());
    r.tap_on("Suggest0");
    assert_eq!(r.app.buffer.text(), "[[chapter-01]]");
    assert_eq!(r.app.buffer.caret(), r.app.buffer.text().len());
}

#[test]
fn esc_and_moving_the_caret_away_close_the_bar() {
    let mut r = Rig::new("close-bar");
    r.type_str("[[ch");
    assert!(r.app.suggest.is_some());
    assert_eq!(r.key(HwKey::Escape), Some(Refresh::Partial));
    assert!(r.app.suggest.is_none());
    assert_eq!(r.app.buffer.text(), "[[ch", "Esc left the text alone");
    r.tick();
    assert!(r.app.suggest.is_none(), "and it stays closed");
    // Typing on brings it back.
    r.type_str("a");
    assert!(r.app.suggest.is_some());
    // The caret leaves the link.
    r.app.buffer.set_caret(0);
    r.tick();
    assert!(r.app.suggest.is_none());
    r.app.buffer.set_caret(r.app.buffer.text().len());
    r.tick();
    assert!(r.app.suggest.is_some());
    r.key(HwKey::Up);
    assert!(r.app.suggest.is_none());
}

#[test]
fn no_bar_in_preview() {
    let mut r = Rig::new("preview");
    r.type_str("[[ch");
    assert!(r.app.suggest.is_some());
    r.app.enter_preview();
    r.tick();
    assert!(r.app.suggest.is_none());
    r.app.exit_preview();
    r.tick();
    assert!(r.app.suggest.is_some(), "back in the editor it returns");
}

#[test]
fn no_bar_while_renaming() {
    let mut r = Rig::new("rename");
    r.type_str("[[ch");
    assert!(r.app.suggest.is_some());
    r.tap_on("Title");
    assert!(r.app.suggest.is_none());
    r.type_str("[[ch");
    assert!(r.app.suggest.is_none(), "not for the name field either");
}

#[test]
fn no_bar_while_finding() {
    let mut r = Rig::new("find");
    r.type_str("[[ch");
    assert!(r.app.suggest.is_some());
    r.tap_on("Find");
    assert!(r.app.find.is_some());
    assert!(r.app.suggest.is_none());
    r.type_str("[[ch");
    assert!(r.app.suggest.is_none());
}

#[test]
fn no_bar_in_the_scratch_pad() {
    let mut r = Rig::new("scratch");
    r.type_str("[[ch");
    assert!(r.app.suggest.is_some());
    r.app.toggle_pane(sigma_render::Pane::Scratch);
    assert!(r.app.scratch_focus);
    r.tick();
    assert!(r.app.suggest.is_none());
    r.type_str("[[ch");
    assert!(r.app.suggest.is_none());
}

#[test]
fn no_bar_with_japanese_candidates_up() {
    let mut r = Rig::new("ja");
    r.type_str("[[ch");
    assert!(r.app.suggest.is_some());
    r.app.settings.kb_layout = KeyLayout::Ja;
    for c in "watashi ".chars() {
        r.app
            .keyboard
            .composer
            .type_char(&mut r.app.buffer, KeyLayout::Ja, c);
    }
    assert!(r.app.keyboard.composer.candidates().is_some());
    r.tick();
    assert!(r.app.suggest.is_none());
}

#[test]
fn usb_enter_and_tab_pick_and_arrows_choose() {
    for k in [HwKey::Return, HwKey::Tab] {
        let mut r = Rig::new(&format!("usb-{k:?}"));
        r.type_str("[[ch");
        r.key(k);
        assert_eq!(r.app.buffer.text(), "[[chapter-01]]", "{k:?}");
        assert!(r.app.suggest.is_none());
    }
    let mut r = Rig::new("usb-arrows");
    r.type_str("[[ch");
    let last = r.labels().len() - 1;
    r.key(HwKey::Right);
    assert_eq!(r.app.suggest.as_ref().map(|s| s.sel), Some(1));
    for _ in 0..last + 2 {
        r.key(HwKey::Right);
    }
    assert_eq!(r.app.suggest.as_ref().map(|s| s.sel), Some(last), "clamped");
    for _ in 0..last {
        r.key(HwKey::Left);
    }
    assert_eq!(r.app.suggest.as_ref().map(|s| s.sel), Some(0));
    r.key(HwKey::Left);
    assert_eq!(r.app.suggest.as_ref().map(|s| s.sel), Some(0), "clamped");
    r.key(HwKey::Right);
    r.key(HwKey::Return);
    assert_eq!(r.app.buffer.text(), "[[chapter-02]]");
}

#[test]
fn without_the_bar_usb_keys_behave_as_before() {
    let mut r = Rig::new("usb-plain");
    r.type_str("ab");
    r.key(HwKey::Left);
    assert_eq!(r.app.buffer.caret(), 1);
    r.key(HwKey::Return);
    assert_eq!(r.app.buffer.text(), "a\nb");
    r.key(HwKey::Tab);
    assert!(r.app.buffer.text().starts_with("a\n"));
    assert!(r.app.suggest.is_none());
}

#[test]
fn an_empty_query_lists_recent_notes_first() {
    let mut r = Rig::new("recent");
    r.app.follow_link("chapter-02");
    r.app.exit_preview();
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    r.app.follow_link("zeta");
    r.app.exit_preview();
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    // Back history: work, chapter-02. Current: zeta.
    r.type_str("[[");
    let l = r.labels();
    assert_eq!(&l[..2], ["chapter-02", "work"], "{l:?}");
    assert!(l.contains(&"chapter-01".to_owned()));
    // The open note is never offered.
    assert!(!l.contains(&"zeta".to_owned()), "{l:?}");
    r.type_str("ze");
    assert!(!r.labels().contains(&"zeta".to_owned()));
}

#[test]
fn no_bar_inside_a_fenced_code_block() {
    let mut r = Rig::new("fence");
    r.app.buffer.replace_text("```\ncode\n".to_owned(), 9);
    r.type_str("[[ch");
    assert!(r.app.suggest.is_none());
    let mut r = Rig::new("fence-closed");
    r.app.buffer.replace_text("```\ncode\n```\n".to_owned(), 13);
    r.type_str("[[ch");
    assert!(r.app.suggest.is_some());
    let mut r = Rig::new("fence-tilde");
    r.app.buffer.replace_text("~~~\n".to_owned(), 4);
    r.type_str("[[ch");
    assert!(r.app.suggest.is_none());
}
