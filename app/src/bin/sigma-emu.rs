//! Headless emulator: runs a script against a 1440×1920 (or `--size WxH`)
//! in-memory panel and writes PNG screenshots. Used by tests and docs.
//!
//! Script steps, one per line (`#` comments):
//!   tap X Y | key LABEL | type TEXT | hw TEXT | page fwd|back
//!   shot NAME | expect TEXT | expect-status WORD | tap-on CONTROL | esc
//!   press X Y | move X Y | release | wait MS | press-word W | move-word W
//!   long-press-word W | double-tap-word W | grab-handle start|end
//!   move-handle-end W | move-handle-start W | expect-bar A,B|none
//!   expect-clipboard T | expect-caret-after W |
//!   age-trash N SECS | expect-not TEXT | expect-selection TEXT | clear-field | expect-file PATH | expect-no-file PATH | expect-open PATH
//!   v0.4: tap-on Options | tap-on Row/Cell (in Options, e.g. Dark mode/On, Size/+)
//!   tap-on Split|Full|Centre|Arrows|123|Pad left|Pad right (landscape bar)
//!   expect-setting KEY=VALUE | expect-words N | expect-screen WxH | restart
//!   v0.5: tap-on Contents|To-dos|Scratch (v1.16: rows of the drawer menu,
//!   opened by tap-on Panels) | pane Contents|To-dos|Scratch (open the
//!   menu, tap the row, close it) | tap-on Heading:TEXT |
//!   tap-on Todo:TEXT | tap-on TodoBox:TEXT | tap-on Filter | tap-on ScratchPane
//!   expect-scratch TEXT | expect-file-contains PATH|TEXT | expect-line N
//!   v0.6: tap-on Find|Graph (icons) | tap-on FindField|ReplaceField|Prev|Next|
//!   Replace|All|Aa|FindClose | tap-on Link:LABEL (preview) | tap-on
//!   Graph:PATH | expect-find LABEL | expect-mode write|preview|find|graph
//!   v0.7: tap-on Help | tap-on App info|Markdown reference|Back|Close (Help)
//!   | perf-type N (time N key taps) | expect-help TITLE | expect-mode help | expect-page N (Help pages too)
//!   v0.8: usb plug|unplug (hot-plug event) | usb-type TEXT (raw evdev key
//!   codes through the USB keymap, US layout) | usb-key CHORD (e.g. ctrl+f,
//!   shift+left, ctrl+shift+end, pgdn) | expect-usb on|off
//!   v0.8.1: pointer plug|unplug | pointer-to X Y | pointer-to-word W |
//!   pointer-before W | pointer-after W | pointer-to-control NAME | pointer-move DX DY | click | double-click |
//!   right-click | mouse-down | mouse-up | wheel N (+ = up) |
//!   expect-pointer visible|hidden (all through the evdev pointer decoder)
//! The background to-do index is settled after every step (deterministic).
//! Coordinates are screen coordinates (landscape is 1920×1440); shots are
//! written the way the reader sees them, turned upright.

use sigma_device::desktop::DesktopDevice;
use sigma_device::evdev::{RawEvent, EV_KEY};
use sigma_device::pointer::{raw_button, raw_move, raw_wheel, PointerDecoder, BTN_LEFT, BTN_RIGHT};
use sigma_device::usbkbd::{code_for_name, raw_for_char_in, KeyMapper};
use sigma_device::Phase;
use sigma_device::{Device, Event, HwKey};
use sigma_scribe::App;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (mut script, mut shots, mut notes, mut size) = (
        None,
        PathBuf::from("target/shots"),
        None,
        (1440u32, 1920u32),
    );
    while let Some(a) = args.next() {
        match a.as_str() {
            "--script" => script = args.next(),
            "--shots" => shots = args.next().unwrap_or_default().into(),
            "--notes" => notes = args.next().map(PathBuf::from),
            "--size" => {
                let s = args.next().unwrap_or_default();
                let (w, h) = s.split_once('x').unwrap_or(("1440", "1920"));
                size = (w.parse().unwrap_or(1440), h.parse().unwrap_or(1920));
            }
            _ => {
                eprintln!(
                    "usage: sigma-emu --script FILE [--shots DIR] [--notes DIR] [--size WxH]"
                );
                return ExitCode::from(2);
            }
        }
    }
    let Some(script) = script else {
        eprintln!("sigma-emu: --script is required");
        return ExitCode::from(2);
    };
    let text = match std::fs::read_to_string(&script) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("sigma-emu: {script}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let notes = notes.unwrap_or_else(sigma_scribe::notes_dir);
    let root = notes.clone();
    let _ = std::fs::create_dir_all(&shots);
    let mut dev = DesktopDevice::new(size.0, size.1);
    let mut app = App::new(size.0, size.1, notes);
    // Scripts tap "Graph": with no Graph plugin in $SIGMA_PLUGINS the
    // emulator shows the built-in one (the app itself hides the icon).
    app.set_builtin_graph(true);
    app.set_builtin_panes(true);
    if let Err(e) = app.render(&mut dev, sigma_device::Refresh::Full) {
        eprintln!("sigma-emu: {e}");
        return ExitCode::FAILURE;
    }
    // Virtual clock for touch gestures (ms); every step advances it.
    let mut clock: u64 = 0;
    let mut finger: (i32, i32) = (0, 0);
    // The USB keyboard's modifier state, as on the device (v0.8).
    let mut usb = KeyMapper::default();
    let mut mouse = PointerDecoder::default();
    for (n, line) in text.lines().enumerate() {
        clock += 16;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (cmd, arg) = line.split_once(' ').unwrap_or((line, ""));
        let fail = |why: String| {
            eprintln!("sigma-emu: line {}: {line}: {why}", n + 1);
            ExitCode::FAILURE
        };
        let mut events = Vec::new();
        match cmd {
            "tap" => {
                let v: Vec<i32> = arg
                    .split_whitespace()
                    .filter_map(|x| x.parse().ok())
                    .collect();
                if v.len() != 2 {
                    return fail("expected X Y".into());
                }
                events.push(Event::Tap { x: v[0], y: v[1] });
            }
            "tap-on" => {
                // v1.4.2: Settings controls on another page are reached
                // through the pages (Display -> Orientation -> Landscape).
                // v1.13.1: Help has no title-bar icon; it opens from Settings →
                // Help & about. Scripts keep `tap-on Help` for "open Help".
                if arg == "Help" && app.control_centre(arg).is_none() {
                    if app.preview.is_some() {
                        app.exit_preview();
                    }
                    app.open_help();
                    let _ = app.render(&mut dev, sigma_device::Refresh::Full);
                    continue;
                }
                reach(&mut app, &mut dev, arg);
                match app.control_centre(arg) {
                    Some((x, y)) => events.push(Event::Tap { x, y }),
                    None => return fail(format!("no control {arg:?} on this screen")),
                }
            }
            // v1.16: `pane Contents|To-dos|Scratch` shows or hides a drawer
            // pane through the drawer menu: open it, tap the row, close it.
            "pane" => {
                for name in app.control_steps(arg) {
                    match app.control_centre(&name) {
                        Some((x, y)) => step(&mut app, &mut dev, Event::Tap { x, y }),
                        None => return fail(format!("no control {name:?} on this screen")),
                    }
                }
            }
            "expect-file" => {
                if !app.library.root().join(arg).exists() {
                    return fail("file missing".into());
                }
            }
            "expect-no-file" => {
                if app.library.root().join(arg).exists() {
                    return fail("file still exists".into());
                }
            }
            // `age-trash N SECS`: the Nth newest item in the trash (0 is the
            // newest) looks SECS seconds old, so the screenshots show more
            // than "Just now".
            "age-trash" => {
                let v: Vec<u64> = arg
                    .split_whitespace()
                    .filter_map(|x| x.parse().ok())
                    .collect();
                let (Some(&n), Some(&secs)) = (v.first(), v.get(1)) else {
                    return fail("expected N SECS".into());
                };
                let Some(t) = app
                    .library
                    .list_trash()
                    .ok()
                    .and_then(|l| l.into_iter().nth(n as usize))
                else {
                    return fail("no such trash item".into());
                };
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                let base = app.library.root().join(sigma_core::library::TRASH);
                let to = base.join(now.saturating_sub(secs).to_string());
                if to.exists() {
                    return fail("another item has that time".into());
                }
                if let Err(e) = std::fs::rename(base.join(&t.id), to) {
                    return fail(e.to_string());
                }
            }
            "expect-open" => {
                if app.current.as_str() != arg {
                    return fail(format!("open note is {:?}", app.current.as_str()));
                }
            }
            "clear-field" => {
                let back = app.keyboard.label(sigma_core::keyboard::Key::Back);
                // Caret to the end, then erase.
                for _ in 0..200 {
                    step(&mut app, &mut dev, Event::Key(HwKey::Right));
                }
                while app.rename_text().is_some_and(|t| !t.is_empty()) {
                    match app.key_centre(&back) {
                        Some((x, y)) => step(&mut app, &mut dev, Event::Tap { x, y }),
                        None => return fail("no Back key".into()),
                    }
                }
            }
            // v1.5.2: `ink-box <i> x,y x,y;x,y …`: strokes in box units
            // into letter box i (20 ms between points).
            // v1.5.3: `ink-line` is the same with x in line heights too
            // (the Words line is wider than high).
            "ink-box" | "ink-line" => {
                let line = cmd == "ink-line";
                let (i, rest) = arg.split_once(' ').unwrap_or((arg, ""));
                let Some(b) = i.parse().ok().and_then(|i| {
                    // Draw has no boxes: index 0 is its whole area (v1.5.4).
                    app.letter_box(i).or_else(|| {
                        (i == 0)
                            .then(|| app.panel_layout().map(|l| l.area))
                            .flatten()
                    })
                }) else {
                    return fail(format!("no letter box {i}"));
                };
                for st in rest.split(';') {
                    let pts: Vec<(i32, i32)> = st
                        .split_whitespace()
                        .filter_map(|p| {
                            let (x, y) = p.split_once(',')?;
                            let (x, y): (f32, f32) = (x.parse().ok()?, y.parse().ok()?);
                            let bw = if line { b.h } else { b.w };
                            Some((b.x + (x * bw as f32) as i32, b.y + (y * b.h as f32) as i32))
                        })
                        .collect();
                    for (k, &(x, y)) in pts.iter().enumerate() {
                        clock += 20;
                        let phase = if k == 0 { Phase::Down } else { Phase::Move };
                        step(
                            &mut app,
                            &mut dev,
                            Event::Touch {
                                phase,
                                x,
                                y,
                                t: clock,
                            },
                        );
                        finger = (x, y);
                    }
                    clock += 20;
                    step(
                        &mut app,
                        &mut dev,
                        Event::Touch {
                            phase: Phase::Up,
                            x: finger.0,
                            y: finger.1,
                            t: clock,
                        },
                    );
                }
            }
            // Touch gestures (v0.2.2): press / move / release / wait, with
            // helpers that aim at words and selection handles.
            "press" | "move" | "press-on" | "move-on" | "press-word" | "move-word"
            | "move-handle-end" | "move-handle-start" | "grab-handle" => {
                let at = match cmd {
                    // v1.4.4: aim at a named control (e.g. a slider stop).
                    "press-on" | "move-on" => app.control_centre(arg),
                    "press" | "move" => {
                        let v: Vec<i32> = arg
                            .split_whitespace()
                            .filter_map(|x| x.parse().ok())
                            .collect();
                        (v.len() == 2).then(|| (v[0], v[1]))
                    }
                    "press-word" | "move-word" => app.word_point(arg),
                    "move-handle-end" => app.handle_point(arg, true),
                    "move-handle-start" => app.handle_point(arg, false),
                    _ => app.handle_centre(usize::from(arg == "end")),
                };
                let Some((x, y)) = at else {
                    return fail("nothing to aim at".into());
                };
                let phase = if cmd.starts_with("move") {
                    Phase::Move
                } else {
                    Phase::Down
                };
                if phase == Phase::Move {
                    // Glide in 6 steps over ~120 ms, like a finger.
                    for i in 1..=6 {
                        clock += 20;
                        let (fx, fy) = (
                            finger.0 + (x - finger.0) * i / 6,
                            finger.1 + (y - finger.1) * i / 6,
                        );
                        step(
                            &mut app,
                            &mut dev,
                            Event::Touch {
                                phase,
                                x: fx,
                                y: fy,
                                t: clock,
                            },
                        );
                    }
                } else {
                    step(
                        &mut app,
                        &mut dev,
                        Event::Touch {
                            phase,
                            x,
                            y,
                            t: clock,
                        },
                    );
                }
                finger = (x, y);
            }
            "wait" => {
                let ms: u64 = arg.parse().unwrap_or(0);
                let end = clock + ms;
                while clock < end {
                    clock += 50;
                    step(&mut app, &mut dev, Event::Tick { t: clock });
                }
            }
            "release" => step(
                &mut app,
                &mut dev,
                Event::Touch {
                    phase: Phase::Up,
                    x: finger.0,
                    y: finger.1,
                    t: clock,
                },
            ),
            "long-press-word" | "double-tap-word" => {
                let Some((x, y)) = app.word_point(arg) else {
                    return fail("word not on screen".into());
                };
                let reps = if cmd == "long-press-word" { 1 } else { 2 };
                for _ in 0..reps {
                    step(
                        &mut app,
                        &mut dev,
                        Event::Touch {
                            phase: Phase::Down,
                            x,
                            y,
                            t: clock,
                        },
                    );
                    let hold = if reps == 1 { 600 } else { 60 };
                    let end = clock + hold;
                    while clock < end {
                        clock += 50;
                        step(&mut app, &mut dev, Event::Tick { t: clock });
                    }
                    step(
                        &mut app,
                        &mut dev,
                        Event::Touch {
                            phase: Phase::Up,
                            x,
                            y,
                            t: clock,
                        },
                    );
                    clock += 80;
                }
                finger = (x, y);
            }
            "expect-bar" => {
                let got: Vec<&str> = app.bar_buttons().iter().map(|(l, _)| *l).collect();
                let want: Vec<&str> = if arg == "none" {
                    Vec::new()
                } else {
                    arg.split(',').collect()
                };
                if got != want {
                    return fail(format!("bar is {got:?}"));
                }
            }
            "expect-clipboard" => {
                if app.keyboard.clipboard != arg {
                    return fail(format!("clipboard is {:?}", app.keyboard.clipboard));
                }
            }
            "expect-caret-after" => {
                let want = app.buffer.text().rfind(arg).map(|i| i + arg.len());
                if want != Some(app.buffer.caret()) || app.buffer.selection().is_some() {
                    return fail(format!(
                        "caret {} sel {:?}",
                        app.buffer.caret(),
                        app.buffer.selection()
                    ));
                }
            }
            "esc" => events.push(Event::Key(HwKey::Escape)),
            "key" => match app.key_centre(arg) {
                Some((x, y)) => events.push(Event::Tap { x, y }),
                None => return fail(format!("no key labelled {arg:?}")),
            },
            "type" => {
                for ch in arg.chars() {
                    let label = if ch == ' ' {
                        "Space".to_owned()
                    } else {
                        ch.to_string()
                    };
                    let upper = ch.is_ascii_uppercase();
                    if upper {
                        let (x, y) = app.key_centre("Shift").unwrap_or_default();
                        step(&mut app, &mut dev, Event::Tap { x, y });
                    }
                    let want = if upper { ch.to_string() } else { label };
                    match app.key_centre(&want) {
                        Some((x, y)) => step(&mut app, &mut dev, Event::Tap { x, y }),
                        None => return fail(format!("no key for {ch:?} on this layer")),
                    }
                }
            }
            "perf-type" => {
                // Times each on-screen key tap (handle + render + present)
                // and prints p50 / p95 / max; `perf-type N` types N letters.
                let n: usize = arg.parse().unwrap_or(200);
                let mut perf = sigma_scribe::Perf::default();
                for i in 0..n {
                    let label = if i % 6 == 5 {
                        "Space".to_owned()
                    } else {
                        ((b'a' + (i % 26) as u8) as char).to_string()
                    };
                    let Some((x, y)) = app.key_centre(&label) else {
                        return fail(format!("no key {label}"));
                    };
                    let t0 = std::time::Instant::now();
                    step(&mut app, &mut dev, Event::Tap { x, y });
                    perf.samples
                        .push(t0.elapsed().as_micros().min(u128::from(u32::MAX)) as u32);
                }
                if let Some((n, p50, p95, max)) = perf.summary() {
                    println!(
                        "perf: {n} keystrokes, keystroke-to-refresh p50 {p50:.1} ms, p95 {p95:.1} ms, max {max:.1} ms"
                    );
                }
            }
            "pointer" => match arg {
                "plug" => events.push(Event::PointerPlug(true)),
                "unplug" => {
                    mouse = PointerDecoder::default();
                    events.push(Event::PointerPlug(false));
                }
                _ => return fail("pointer plug|unplug".into()),
            },
            "pointer-to" | "pointer-to-word" | "pointer-to-control" | "pointer-move"
            | "pointer-before" | "pointer-after" => {
                if cmd == "pointer-to-control" {
                    reach(&mut app, &mut dev, arg);
                }
                let v: Vec<i32> = arg
                    .split_whitespace()
                    .filter_map(|x| x.parse().ok())
                    .collect();
                let target = match cmd {
                    "pointer-to-word" => app.word_point(arg),
                    "pointer-before" => app.text_point(arg, false),
                    "pointer-after" => app.text_point(arg, true),
                    "pointer-to-control" => app.control_centre(arg),
                    _ => (v.len() == 2).then(|| (v[0], v[1])),
                };
                let Some((x, y)) = target else {
                    return fail("nothing to aim at".into());
                };
                let (dx, dy) = if cmd == "pointer-move" {
                    (x, y)
                } else {
                    (x - app.pointer.x, y - app.pointer.y)
                };
                for r in raw_move(dx, dy) {
                    events.extend(mouse.feed(r, clock));
                }
            }
            "click" | "double-click" | "right-click" | "mouse-down" | "mouse-up" => {
                let code = if cmd == "right-click" {
                    BTN_RIGHT
                } else {
                    BTN_LEFT
                };
                let presses = match cmd {
                    "double-click" => vec![true, false, true, false],
                    "mouse-down" => vec![true],
                    "mouse-up" => vec![false],
                    _ => vec![true, false],
                };
                for down in presses {
                    clock += 30;
                    for r in raw_button(code, down) {
                        for e in mouse.feed(r, clock) {
                            step(&mut app, &mut dev, e);
                        }
                    }
                }
            }
            "wheel" => {
                for r in raw_wheel(arg.parse().unwrap_or(0)) {
                    events.extend(mouse.feed(r, clock));
                }
            }
            "expect-pointer" => {
                let p = &app.pointer;
                if (p.visible && p.live) != (arg == "visible") {
                    return fail(format!("pointer {p:?}"));
                }
            }
            "usb" => match arg {
                "plug" => events.push(Event::Usb(true)),
                "unplug" => {
                    usb = KeyMapper::default();
                    events.push(Event::Usb(false));
                }
                _ => return fail("usb plug|unplug".into()),
            },
            // v1.3: typed on the layout chosen in Options (usb-key names
            // US key positions).
            "usb-type" => {
                let l = app.settings.kb_layout;
                usb.layout = Some(l);
                let arg = if l.hangul() {
                    sigma_core::hangul::spell(arg)
                } else {
                    arg.to_string()
                };
                events.extend(
                    arg.chars()
                        .flat_map(|c| raw_for_char_in(l, c))
                        .filter_map(|e| usb.feed(e)),
                );
            }
            "usb-key" => {
                let mut codes = Vec::new();
                for name in arg.split('+') {
                    match code_for_name(name) {
                        Some(c) => codes.push(c),
                        None => return fail(format!("unknown key {name:?}")),
                    }
                }
                let ev = |code, value| RawEvent {
                    kind: EV_KEY,
                    code,
                    value,
                };
                let raws: Vec<RawEvent> = codes
                    .iter()
                    .map(|&c| ev(c, 1))
                    .chain(codes.iter().rev().map(|&c| ev(c, 0)))
                    .collect();
                usb.layout = Some(app.settings.kb_layout);
                events.extend(raws.into_iter().filter_map(|e| usb.feed(e)));
            }
            "expect-usb" => {
                if app.usb_live != (arg == "on") {
                    return fail(format!("usb_live is {}", app.usb_live));
                }
            }
            "hw" => events.extend(arg.chars().map(|c| Event::Key(HwKey::Char(c)))),
            "page" => events.push(Event::PageTurn {
                forward: arg != "back",
            }),
            "shot" => {
                let view = app.view(&dev.panel);
                if let Err(e) =
                    sigma_device::desktop::write_png(&view, &shots.join(format!("{arg}.png")))
                {
                    return fail(e.to_string());
                }
                println!("shot {}", shots.join(format!("{arg}.png")).display());
            }
            "expect" => {
                if !app.buffer.text().contains(arg) {
                    return fail("text not in note".into());
                }
            }
            "expect-not" => {
                if app.buffer.text().contains(arg) {
                    return fail("text still in note".into());
                }
            }
            "expect-selection" => {
                if app.buffer.selected_text() != Some(arg) {
                    return fail(format!("selection is {:?}", app.buffer.selected_text()));
                }
            }
            "open-fixture" => {
                // Copies a fixture into the library root and opens it.
                let name = std::path::Path::new(arg)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let body = match std::fs::read_to_string(arg) {
                    Ok(b) => b,
                    Err(e) => return fail(e.to_string()),
                };
                if let Err(e) = std::fs::write(app.library.root().join(&name), body) {
                    return fail(e.to_string());
                }
                match sigma_core::NotePath::parse(&name) {
                    Ok(p) => {
                        app.open(&p);
                        app.buffer.set_caret(0);
                    }
                    Err(e) => return fail(e.to_string()),
                }
                step(&mut app, &mut dev, Event::Key(HwKey::Escape));
                let _ = app.render(&mut dev, sigma_device::Refresh::Full);
            }
            "expect-find" => {
                let got = app.find.as_ref().map(|_| app.find_label());
                if got.as_deref() != Some(arg) {
                    return fail(format!("find bar shows {got:?}"));
                }
            }
            // Graph map (v1.0, S9.1).
            "copy-notes" => {
                // Copies a folder of notes (the S10.1 test pack's _test/)
                // into the library as _test/, then opens
                // _test/test-00-start-here.md.
                fn copy(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
                    std::fs::create_dir_all(to)?;
                    for e in std::fs::read_dir(from)? {
                        let e = e?;
                        let dst = to.join(e.file_name());
                        if e.file_type()?.is_dir() {
                            copy(&e.path(), &dst)?;
                        } else {
                            std::fs::copy(e.path(), dst)?;
                        }
                    }
                    Ok(())
                }
                if let Err(e) = copy(std::path::Path::new(arg), &app.library.root().join("_test")) {
                    return fail(e.to_string());
                }
                match sigma_core::NotePath::parse("_test/test-00-start-here.md") {
                    Ok(p) => app.open(&p),
                    Err(e) => return fail(e.to_string()),
                }
                let _ = app.render(&mut dev, sigma_device::Refresh::Full);
            }
            "make-notes" => {
                // N synthetic notes in map/ with deterministic links: each
                // note links to the next and to one "hub"; every tenth is
                // an orphan. Opens map/n000.md.
                let n: usize = arg.parse().unwrap_or(0);
                let dir = app.library.root().join("map");
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    return fail(e.to_string());
                }
                for i in 0..n {
                    let mut body = format!("# Note {i}\n\n");
                    if i % 10 != 9 {
                        let next = (i + 1) % n;
                        if next % 10 != 9 {
                            body.push_str(&format!("Next: [[n{next:03}]]\n"));
                        }
                        body.push_str(&format!("Hub: [[n{:03}]]\n", (i * 7 / 10) % 20));
                    }
                    if let Err(e) = std::fs::write(dir.join(format!("n{i:03}.md")), body) {
                        return fail(e.to_string());
                    }
                }
                if let Ok(p) = sigma_core::NotePath::parse("map/n000.md") {
                    app.open(&p);
                }
                let _ = app.render(&mut dev, sigma_device::Refresh::Full);
            }
            "perf-graph" => {
                // Opens the Graph with an empty cache and reports the layout
                // time; fails above ARG ms (desktop).
                let limit: f64 = arg.parse().unwrap_or(f64::MAX);
                app.clear_map_cache();
                app.open_graph();
                let us = app.graph.as_ref().map_or(0, |g| g.layout_us);
                let n = app.graph.as_ref().map_or(0, |g| g.rows.len());
                let l = app.graph.as_ref().map_or(0, |g| g.links.len());
                let t0 = std::time::Instant::now();
                let _ = app.render(&mut dev, sigma_device::Refresh::Full);
                let draw = t0.elapsed().as_micros();
                println!(
                    "perf-graph notes={n} links={l} layout_ms={:.1} draw_ms={:.1}",
                    us as f64 / 1000.0,
                    draw as f64 / 1000.0
                );
                if us as f64 / 1000.0 > limit {
                    return fail(format!("layout took {} ms", us / 1000));
                }
                app.graph = None;
                let _ = app.render(&mut dev, sigma_device::Refresh::Full);
            }
            "expect-graph" => {
                let got = app
                    .graph
                    .as_ref()
                    .map(|g| if g.map { "map" } else { "list" });
                if got != Some(arg) {
                    return fail(format!("graph shows {got:?}"));
                }
            }
            "expect-selected" => {
                let got = app
                    .graph
                    .as_ref()
                    .and_then(|g| g.selected.map(|i| g.rows[i].name.clone()))
                    .unwrap_or_else(|| "none".into());
                if got != arg {
                    return fail(format!("selected is {got}"));
                }
            }
            "expect-zoom" => {
                let got = app.graph.as_ref().map(|g| g.zoom.to_string());
                if got.as_deref() != Some(arg) {
                    return fail(format!("zoom is {got:?}"));
                }
            }
            "expect-mode" => {
                let mode = if app.help.is_some() {
                    "help"
                } else if app.graph.is_some() {
                    "graph"
                } else if app.find.is_some() && app.preview.is_none() {
                    "find"
                } else if app.preview.is_some() {
                    "preview"
                } else {
                    "write"
                };
                if mode != arg {
                    return fail(format!("mode is {mode}"));
                }
            }
            "expect-help" => {
                let got = app.help.as_ref().map(|h| h.page.title());
                if got != Some(arg) {
                    return fail(format!("help shows {got:?}"));
                }
            }
            "expect-page" => {
                let page = app
                    .help
                    .as_ref()
                    .map(|h| h.at + 1)
                    .or(app.preview.as_ref().map(|p| p.page + 1));
                if page.map(|p| p.to_string()).as_deref() != Some(arg) {
                    return fail(format!("preview page is {page:?}"));
                }
            }
            "expect-pages-at-least" => {
                let n = app.preview.as_ref().map_or(0, |p| p.doc.pages.len());
                if n < arg.parse().unwrap_or(usize::MAX) {
                    return fail(format!("only {n} preview pages"));
                }
            }
            "expect-scratch" => {
                if !app.scratch.text().contains(arg) {
                    return fail(format!("scratch is {:?}", app.scratch.text()));
                }
            }
            "expect-file-contains" => {
                let (path, want) = arg.split_once('|').unwrap_or((arg, ""));
                let body =
                    std::fs::read_to_string(app.library.root().join(path)).unwrap_or_default();
                if !body.contains(want) {
                    return fail(format!("{path} is {body:?}"));
                }
            }
            "expect-line" => {
                if app.buffer.line_number().to_string() != arg {
                    return fail(format!("caret on line {}", app.buffer.line_number()));
                }
            }
            "expect-setting" => {
                let file = std::fs::read_to_string(app.library.root().join(".sigma-settings"))
                    .unwrap_or_default();
                if !file.lines().any(|l| l == arg) {
                    return fail(format!("settings file is {file:?}"));
                }
            }
            "expect-words" => {
                let n = sigma_scribe::words_per_row(&app);
                if n.to_string() != arg {
                    return fail(format!("{n} words per row"));
                }
            }
            "expect-screen" => {
                let got = format!("{}x{}", app.canvas.width, app.canvas.height);
                if got != arg {
                    return fail(format!("screen is {got}"));
                }
            }
            "sensor" => {
                // v1.2: pretend an accelerometer is (not) present.
                dev.gsensor = arg == "on";
                app.set_gsensor(dev.gsensor);
            }
            "experimental" => {
                // v1.12: Settings → Experimental features, without the taps.
                app.change(sigma_core::settings::Change::Experimental(arg == "on"));
            }
            "light" => {
                // v1.12: pretend a front light at this step (0 to 8), or none.
                dev.light = arg.parse::<u8>().ok();
                app.sync_light(&mut dev);
                if let Some(n) = dev.light {
                    app.change(sigma_core::settings::Change::Backlight(n));
                }
            }
            "warmth" => {
                // v1.17: pretend a warm-to-cool mixer at this step (0 to 10),
                // or none. `warmth 3 unreadable` cannot be read back, as the
                // stock reader never does.
                let mut it = arg.split_whitespace();
                dev.warmth = it.next().and_then(|n| n.parse::<u8>().ok());
                dev.warmth_readable = it.next() != Some("unreadable");
                app.sync_warmth(&mut dev);
            }
            "expect-warmth" => {
                app.sync_warmth(&mut dev);
                if dev.warmth.map_or(String::new(), |n| n.to_string()) != arg {
                    return fail(format!("warmth is {:?}", dev.warmth));
                }
            }
            "expect-light" => {
                app.sync_light(&mut dev);
                if dev.light.map_or(String::new(), |n| n.to_string()) != arg {
                    return fail(format!("light is {:?}", dev.light));
                }
            }
            "tilt" => {
                use sigma_device::gsensor::Tilt;
                let tilt = match arg {
                    "upright" => Tilt::Upright,
                    "cw" => Tilt::Clockwise,
                    "180" => Tilt::UpsideDown,
                    "ccw" => Tilt::CounterClockwise,
                    _ => return fail("tilt upright|cw|180|ccw".into()),
                };
                events.push(Event::Tilt { tilt, t: clock });
            }
            "expect-rotation" => {
                let got = app.rotation().name();
                if got != arg {
                    return fail(format!("rotation is {got}"));
                }
            }
            "expect-orientation-status" => {
                let got = app.orientation_status();
                if got != arg {
                    return fail(format!("status is {got:?}"));
                }
            }
            "locale-conf" => {
                // v1.4: read CurrentLocale from a Kobo config file (fixture)
                // with the same read-only parser as the device, then apply it.
                let loc = sigma_core::locale::read_locale(std::path::Path::new(arg));
                let l = loc.map_or(sigma_render::i18n::Lang::En, |l| {
                    sigma_render::i18n::Lang::from_locale(&l)
                });
                app.set_language(l);

                if let Err(e) = app.render(&mut dev, sigma_device::Refresh::Full) {
                    return fail(e.to_string());
                }
            }
            "language" => {
                // v1.4: en | fr | pt (what CurrentLocale would select).
                app.set_language(sigma_render::i18n::Lang::from_locale(arg));

                if let Err(e) = app.render(&mut dev, sigma_device::Refresh::Full) {
                    return fail(e.to_string());
                }
            }
            "expect-language" => {
                let got = format!("{:?}", sigma_render::i18n::lang()).to_lowercase();
                if got != arg {
                    return fail(format!("language is {got}"));
                }
            }
            "ui-scale" => {
                // v1.1: step Options -> UI scale to a value (100..=200).
                let Ok(want) = arg.trim_end_matches('%').parse::<u16>() else {
                    return fail("ui-scale needs a percentage".into());
                };
                if !sigma_core::settings::UI_SCALES.contains(&want) {
                    return fail(format!("no UI scale {want}"));
                }
                for _ in 0..sigma_core::settings::UI_SCALES.len() {
                    let now = app.settings.ui_scale;
                    if now == want {
                        break;
                    }
                    app.change(sigma_core::settings::Change::UiScale(if now < want {
                        1
                    } else {
                        -1
                    }));
                }
                if let Err(e) = app.render(&mut dev, sigma_device::Refresh::Full) {
                    return fail(e.to_string());
                }
            }
            "restart" => {
                // Power off and on: settings and the note must come back.
                app.handle(Event::Power);
                app = App::new(size.0, size.1, root.clone());
                app.set_builtin_graph(true);
                app.set_builtin_panes(true);
                app.set_gsensor(dev.gsensor);
                if let Err(e) = app.render(&mut dev, sigma_device::Refresh::Full) {
                    return fail(e.to_string());
                }
            }
            "sync-demo" => {
                // v1.16: what Settings → Sync shows for a state of Syncthing,
                // without running it. `expect-sync` checks the status word.
                match demo_sync(arg) {
                    Some(v) => app.set_sync_view(v),
                    None => return fail("unknown sync state".into()),
                }
                let _ = app.render(&mut dev, sigma_device::Refresh::Partial);
            }
            "expect-sync" => {
                let got = format!("{:?}", app.sync_view().status());
                if !got.starts_with(arg) {
                    return fail(format!("sync status is {got}"));
                }
            }
            "expect-sync-pending" => {
                let v = app.sync_view();
                let got = format!("{},{}", v.pending_devices.len(), v.pending_folders.len());
                if got != arg {
                    return fail(format!("pending devices,folders: {got}"));
                }
            }
            "expect-status" => {
                if app.autosave.state.label() != arg {
                    return fail(format!("status is {:?}", app.autosave.state.label()));
                }
            }
            _ => return fail("unknown step".into()),
        }
        for e in events {
            step(&mut app, &mut dev, e);
        }
    }
    let (w, h) = dev.size();
    println!(
        "sigma-emu: every step passed ({w}x{h}, {} refreshes)",
        dev.refreshes.len()
    );
    ExitCode::SUCCESS
}

/// v1.16: a state of Syncthing for the Sync page (see `docs/sync.md` §10).
fn demo_sync(name: &str) -> Option<sigma_core::sync::View> {
    use sigma_core::sync::{FolderView, Peer, PendingDevice, PendingFolder, Run, View};
    const ME: &str = "MFZWI3D-BONSGYC-YLTMRWG-C43ENR5-QXGZDMM-FZWI3DP-BONSGYY-LTMRWAD";
    const LAPTOP: &str = "P56IOI7-MZJNU2Y-IQGDREY-DM2MGTI-MGL3BXN-PQ6W5BM-TBBZ4TJ-XZWICQ2";
    let running = View {
        run: Run::Running,
        device_id: ME.into(),
        ..View::default()
    };
    let laptop = |connected| Peer {
        id: LAPTOP.into(),
        name: "Laptop".into(),
        connected,
    };
    let folder = |state: &str, need: u64| FolderView {
        id: "notes".into(),
        state: state.into(),
        need_files: need,
        need_bytes: need * 4096,
        ..FolderView::default()
    };
    Some(match name {
        "off" => View::default(),
        "missing" => View {
            run: Run::NotInstalled,
            ..View::default()
        },
        "starting" => View {
            run: Run::Starting,
            ..View::default()
        },
        "failed" => View {
            run: Run::Failed("exited at start (exit status: 1)".into()),
            ..View::default()
        },
        "alone" => running,
        "pending-device" => View {
            pending_devices: vec![PendingDevice {
                id: LAPTOP.into(),
                name: "Laptop".into(),
                address: "192.168.1.20:22000".into(),
            }],
            ..running
        },
        "pending-folder" => View {
            peers: vec![laptop(true)],
            pending_folders: vec![PendingFolder {
                id: "abcde-fghij".into(),
                label: "Notes".into(),
                device: LAPTOP.into(),
                device_name: "Laptop".into(),
            }],
            ..running
        },
        "syncing" => View {
            peers: vec![laptop(true)],
            folder: Some(folder("syncing", 3)),
            last_sync: Some("2026-10-28 13:58".into()),
            ..running
        },
        "idle" => View {
            peers: vec![laptop(true)],
            folder: Some(folder("idle", 0)),
            last_sync: Some("2026-10-28 14:05".into()),
            conflicts: 2,
            ..running
        },
        "offline" => View {
            peers: vec![laptop(false)],
            folder: Some(folder("idle", 0)),
            last_sync: Some("2026-10-27 21:40".into()),
            ..running
        },
        _ => return None,
    })
}

/// v1.4.2: taps through Settings pages until `name` is on screen.
fn reach(app: &mut App, dev: &mut DesktopDevice, name: &str) {
    for _ in 0..8 {
        if app.control_centre(name).is_some() {
            return;
        }
        let Some(hop) = app.settings_step(name) else {
            return;
        };
        let Some((x, y)) = app.control_centre(&hop) else {
            return;
        };
        step(app, dev, Event::Tap { x, y });
    }
}

fn step(app: &mut App, dev: &mut DesktopDevice, e: Event) {
    if let Some(r) = app.handle(e) {
        let _ = app.render(dev, r);
    }
    if app.settle() {
        let _ = app.render(dev, sigma_device::Refresh::Partial);
    }
}
