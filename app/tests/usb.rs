//! v0.8 USB keyboard: raw evdev codes → keymap → the app's key paths.

use sigma_device::evdev::{RawEvent, EV_KEY};
use sigma_device::usbkbd::{code_for_name, raw_for_char, KeyMapper};
use sigma_device::{Event, HwKey, Mods};
use sigma_scribe::App;

fn app(tag: &str) -> App {
    let d = std::env::temp_dir().join(format!("sigma-usb-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let mut a = App::new(1440, 1920, d);
    a.buffer.select_all();
    a.buffer.backspace();
    a
}

fn send(a: &mut App, m: &mut KeyMapper, raws: Vec<RawEvent>) {
    for r in raws {
        if let Some(e) = m.feed(r) {
            a.handle(e);
            a.settle();
        }
    }
}

fn typ(a: &mut App, m: &mut KeyMapper, text: &str) {
    send(a, m, text.chars().flat_map(raw_for_char).collect());
}

fn chord(a: &mut App, m: &mut KeyMapper, spec: &str) {
    let codes: Vec<u16> = spec.split('+').map(|n| code_for_name(n).unwrap()).collect();
    let ev = |code, value| RawEvent {
        kind: EV_KEY,
        code,
        value,
    };
    let raws = codes
        .iter()
        .map(|&c| ev(c, 1))
        .chain(codes.iter().rev().map(|&c| ev(c, 0)))
        .collect();
    send(a, m, raws);
}

#[test]
fn types_and_shortcuts_match_the_on_screen_ctrl() {
    let (mut a, mut m) = (app("short"), KeyMapper::default());
    a.handle(Event::Usb(true));
    assert!(a.usb_live && a.screen.usb_live);
    typ(&mut a, &mut m, "Hello World");
    assert_eq!(a.buffer.text(), "Hello World");
    chord(&mut a, &mut m, "shift+left");
    chord(&mut a, &mut m, "shift+left");
    assert_eq!(a.buffer.selected_text(), Some("ld"));
    chord(&mut a, &mut m, "ctrl+x");
    assert_eq!(a.buffer.text(), "Hello Wor");
    chord(&mut a, &mut m, "ctrl+home");
    chord(&mut a, &mut m, "ctrl+v");
    assert_eq!(a.buffer.text(), "ldHello Wor");
    chord(&mut a, &mut m, "ctrl+z");
    assert_eq!(a.buffer.text(), "Hello Wor");
    chord(&mut a, &mut m, "ctrl+y");
    assert_eq!(a.buffer.text(), "ldHello Wor");
    chord(&mut a, &mut m, "end");
    chord(&mut a, &mut m, "ctrl+left");
    chord(&mut a, &mut m, "ctrl+a");
    assert_eq!(a.buffer.selected_text(), Some("ldHello Wor"));
    // The on-screen modifiers are untouched by chords.
    assert!(!a.keyboard.ctrl && !a.keyboard.shift && !a.keyboard.alt);
}

#[test]
fn osk_one_shot_shift_survives_a_usb_chord() {
    let (mut a, mut m) = (app("osk"), KeyMapper::default());
    a.keyboard.shift = true;
    typ(&mut a, &mut m, "ab");
    chord(&mut a, &mut m, "ctrl+a");
    assert!(a.keyboard.shift, "one-shot Shift still armed");
    assert_eq!(
        a.buffer.text(),
        "ab",
        "USB letters are not shifted by the OSK"
    );
}

#[test]
fn ctrl_f_opens_find_and_typing_goes_into_the_field() {
    let (mut a, mut m) = (app("find"), KeyMapper::default());
    typ(&mut a, &mut m, "one two one");
    chord(&mut a, &mut m, "ctrl+f");
    assert!(a.find.is_some());
    typ(&mut a, &mut m, "one");
    assert_eq!(a.find.as_ref().unwrap().query.text(), "one");
    assert_eq!(a.find_label(), "1 of 2");
    chord(&mut a, &mut m, "esc");
    assert!(a.find.is_none());
    assert_eq!(a.buffer.text(), "one two one");
}

#[test]
fn scratch_gets_usb_typing() {
    let (mut a, mut m) = (app("scratch"), KeyMapper::default());
    let (x, y) = a.control_centre("Scratch").unwrap();
    a.handle(Event::Tap { x, y });
    let (x, y) = a.control_centre("ScratchPane").unwrap();
    a.handle(Event::Tap { x, y });
    typ(&mut a, &mut m, "jot");
    chord(&mut a, &mut m, "ctrl+a");
    chord(&mut a, &mut m, "backspace");
    typ(&mut a, &mut m, "ok");
    assert_eq!(a.scratch.text(), "ok");
    assert_eq!(a.buffer.text(), "");
}

#[test]
fn unplug_and_sheets_are_safe() {
    let (mut a, mut m) = (app("unplug"), KeyMapper::default());
    a.handle(Event::Usb(true));
    // Ctrl held when the cable goes: the app sees Usb(false) and goes on.
    send(
        &mut a,
        &mut m,
        vec![RawEvent {
            kind: EV_KEY,
            code: code_for_name("ctrl").unwrap(),
            value: 1,
        }],
    );
    a.handle(Event::Usb(false));
    assert!(!a.usb_live);
    let mut m = KeyMapper::default();
    typ(&mut a, &mut m, "x");
    assert_eq!(a.buffer.text(), "x");
    // Chords do nothing on sheets.
    a.open_options();
    a.handle(Event::Chord {
        key: HwKey::Char('z'),
        mods: Mods {
            ctrl: true,
            ..Mods::default()
        },
    });
    assert_eq!(a.buffer.text(), "x");
}

#[test]
fn host_mode_follows_the_setting_and_is_given_back_on_exit() {
    use sigma_core::settings::Change;
    use sigma_device::desktop::DesktopDevice;
    let mut a = app("otg");
    let mut dev = DesktopDevice::new(1440, 1920);
    dev.events.push_back(Event::Quit);
    a.run(&mut dev).unwrap();
    assert!(dev.usb_host.is_empty(), "setting Off: port untouched");
    let mut a = app("otg2");
    a.settings.apply(Change::Usb(true));
    dev.events.push_back(Event::Usb(true));
    dev.events.push_back(Event::Quit);
    a.run(&mut dev).unwrap();
    assert_eq!(dev.usb_host, vec![true, false]);
}
