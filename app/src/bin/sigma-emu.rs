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
//!   expect-not TEXT | expect-selection TEXT | clear-field | expect-file PATH | expect-no-file PATH | expect-open PATH

use sigma_device::desktop::DesktopDevice;
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
    let _ = std::fs::create_dir_all(&shots);
    let mut dev = DesktopDevice::new(size.0, size.1);
    let mut app = App::new(size.0, size.1, notes);
    if let Err(e) = app.render(&mut dev, sigma_device::Refresh::Full) {
        eprintln!("sigma-emu: {e}");
        return ExitCode::FAILURE;
    }
    // Virtual clock for touch gestures (ms); every step advances it.
    let mut clock: u64 = 0;
    let mut finger: (i32, i32) = (0, 0);
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
            "tap-on" => match app.control_centre(arg) {
                Some((x, y)) => events.push(Event::Tap { x, y }),
                None => return fail(format!("no control {arg:?} on this screen")),
            },
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
            // Touch gestures (v0.2.2): press / move / release / wait, with
            // helpers that aim at words and selection handles.
            "press" | "move" | "press-word" | "move-word" | "move-handle-end"
            | "move-handle-start" | "grab-handle" => {
                let at = match cmd {
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
            "hw" => events.extend(arg.chars().map(|c| Event::Key(HwKey::Char(c)))),
            "page" => events.push(Event::PageTurn {
                forward: arg != "back",
            }),
            "shot" => {
                if let Err(e) = dev.shot(&shots.join(format!("{arg}.png"))) {
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
            "expect-mode" => {
                let mode = if app.preview.is_some() {
                    "preview"
                } else {
                    "write"
                };
                if mode != arg {
                    return fail(format!("mode is {mode}"));
                }
            }
            "expect-page" => {
                let page = app.preview.as_ref().map(|p| p.page + 1);
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

fn step(app: &mut App, dev: &mut DesktopDevice, e: Event) {
    if let Some(r) = app.handle(e) {
        let _ = app.render(dev, r);
    }
}
