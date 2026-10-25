//! The side-drawer panes as WASM plugins (`contents-pane`, `todo-pane`,
//! `scratch-pane`) draw the same pixels, send the same refresh classes and
//! reach the same states as the app's built-in panes. Two apps share one
//! library; one has the plugins installed, the other has the built-in panes.
//! The same steps run on both, and after each the whole screen, the refresh
//! regions and the app state must be identical.
//!
//! The modules under test are app/tests/fixtures/<id>/pane.wasm, built from
//! the sigma-scribe-plugins repo (scripts/update-plugin-fixtures.sh).

use sigma_core::settings::{DrawerWidth, Paper};
use sigma_core::{Change, NotePath};
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Refresh};
use sigma_scribe::App;
use std::path::{Path, PathBuf};

const PANES: [&str; 3] = ["contents-pane", "todo-pane", "scratch-pane"];

fn fixture(id: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(id)
}

/// What the library holds.
#[derive(Clone, Copy)]
enum Lib {
    /// Headings and to-dos, done and open, in several notes, with long text.
    Rich,
    /// More to-dos and headings than the drawer has room for.
    Crowded,
    /// A note with no headings and no to-dos.
    Bare,
}

fn write_library(dir: &Path, lib: Lib) {
    let w = |name: &str, body: &str| std::fs::write(dir.join(name), body).unwrap();
    match lib {
        Lib::Rich => {
            w(
                "a.md",
                "# Plan for the week\n\nintro\n\n## Monday\n- [ ] call Ana about the contract\n- [x] send the invoice\n\n### A very long heading that cannot possibly fit in the drawer's width\n- [ ] a very long to-do text that cannot possibly fit on one row of the drawer\n\n## Tuesday\n#### Deep one\n##### Deeper still\n- [x] done early\n",
            );
            w(
                "b.md",
                "# Other\n- [ ] buy milk\n- [x] pay rent\n- [ ] 日本語のタスク\n",
            );
            w("c.md", "- [ ] in a note with no heading\n");
        }
        Lib::Crowded => {
            let mut a = String::from("# Top\n");
            for i in 0..40 {
                a += &format!(
                    "## Section {i}\n- [{}] task number {i}\n",
                    if i % 3 == 0 { 'x' } else { ' ' }
                );
            }
            w("a.md", &a);
            w("b.md", "- [ ] elsewhere\n- [x] elsewhere done\n");
        }
        Lib::Bare => {
            w("a.md", "just words, no structure at all\n");
        }
    }
}

struct Rig {
    app: App,
    dev: DesktopDevice,
}

impl Rig {
    fn new(
        tag: &str,
        plugin: bool,
        w: u32,
        h: u32,
        lib: Lib,
        night: bool,
        width: DrawerWidth,
    ) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-pq-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (notes, plugins) = (d.join("notes"), d.join("plugins"));
        std::fs::create_dir_all(&notes).unwrap();
        write_library(&notes, lib);
        std::fs::create_dir_all(&plugins).unwrap();
        let mut state = String::new();
        if plugin {
            for id in PANES {
                std::fs::create_dir_all(plugins.join(id)).unwrap();
                for f in ["plugin.conf", "pane.wasm"] {
                    std::fs::copy(fixture(id).join(f), plugins.join(id).join(f)).unwrap();
                }
                state += &format!("{id}=1.0.0,enabled\n");
            }
        }
        std::fs::write(plugins.join("plugins.state"), state).unwrap();
        let mut app = App::new(w, h, notes);
        app.set_plugins_root(plugins);
        if !plugin {
            // The reference: the app's own panes, with no plugin installed.
            app.set_builtin_panes(true);
        }
        app.open(&NotePath::parse("a.md").unwrap());
        if night {
            app.change(Change::Paper(Paper::Night));
        }
        app.change(Change::DrawerWidth(width));
        let mut dev = DesktopDevice::new(w, h);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev }
    }

    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(r) = r {
            self.app.render(&mut self.dev, r).unwrap();
        }
        r
    }
}

#[derive(Clone, Debug)]
enum Step {
    /// Tap the control with this label (title icons, `Heading:`, `Todo:` ...).
    Tap(&'static str),
    /// Type text on the hardware keyboard.
    Type(&'static str),
    Key(HwKey),
}

fn run(r: &mut Rig, step: &Step) -> Option<Refresh> {
    match step {
        Step::Tap(label) => {
            let (x, y) = r.app.control_centre(label).unwrap_or_else(|| {
                panic!(
                    "no control {label}; scratch {:?}, to-dos {:?}, panes {:?}",
                    r.app.scratch.text(),
                    r.app
                        .todo_list()
                        .iter()
                        .map(|t| t.todo.text.clone())
                        .collect::<Vec<_>>(),
                    r.app.drawer
                )
            });
            r.send(Event::Tap { x, y })
        }
        Step::Type(s) => {
            let mut last = None;
            for c in s.chars() {
                last = r.send(Event::Key(HwKey::Char(c))).or(last);
            }
            last
        }
        Step::Key(k) => r.send(Event::Key(*k)),
    }
}

fn equivalent(tag: &str, w: u32, h: u32, lib: Lib, night: bool, steps: &[Step]) {
    equivalent_wide(tag, w, h, lib, night, DrawerWidth::Wide, steps);
}

fn equivalent_wide(
    tag: &str,
    w: u32,
    h: u32,
    lib: Lib,
    night: bool,
    width: DrawerWidth,
    steps: &[Step],
) {
    let mut builtin = Rig::new(&format!("{tag}-b"), false, w, h, lib, night, width);
    let mut plugin = Rig::new(&format!("{tag}-p"), true, w, h, lib, night, width);
    for (n, step) in steps.iter().enumerate() {
        let (rb, rp) = (run(&mut builtin, step), run(&mut plugin, step));
        // The to-do index is built in the background; wait for it in both.
        for r in [&mut builtin, &mut plugin] {
            r.app.settle();
            r.app.render(&mut r.dev, Refresh::Partial).unwrap();
        }
        let ctx = format!("{tag}: step {n} {step:?}");
        assert_eq!(rb, rp, "{ctx}: refresh class");
        assert_eq!(
            builtin.app.current, plugin.app.current,
            "{ctx}: current note"
        );
        assert_eq!(builtin.app.drawer, plugin.app.drawer, "{ctx}: open panes");
        assert_eq!(
            builtin.app.scratch.text(),
            plugin.app.scratch.text(),
            "{ctx}: scratch text"
        );
        assert_eq!(
            builtin.app.buffer.text(),
            plugin.app.buffer.text(),
            "{ctx}: note text"
        );
        assert_eq!(
            builtin.app.preview.is_some(),
            plugin.app.preview.is_some(),
            "{ctx}: preview"
        );
        let (cb, cp) = (&builtin.app.canvas, &plugin.app.canvas);
        if cb.pixels != cp.pixels {
            let differing = cb
                .pixels
                .iter()
                .zip(&cp.pixels)
                .filter(|(a, b)| a != b)
                .count();
            panic!("{ctx}: {differing} pixels differ, within {:?}", cb.diff(cp));
        }
    }
}

#[test]
fn contents_draws_and_jumps_like_the_built_in() {
    equivalent(
        "contents",
        1440,
        1920,
        Lib::Rich,
        false,
        &[
            Step::Tap("Contents"),
            Step::Tap("Heading:Monday"),
            Step::Tap("Heading:Deep one"),
            Step::Tap("Contents"),
        ],
    );
}

#[test]
fn todos_draw_filter_toggle_and_open_like_the_built_in() {
    equivalent(
        "todos",
        1440,
        1920,
        Lib::Rich,
        false,
        &[
            Step::Tap("To-dos"),
            Step::Tap("Filter"),
            Step::Tap("Filter"),
            Step::Tap("Filter"),
            Step::Tap("TodoBox:call Ana about the contract"),
            Step::Tap("TodoBox:buy milk"),
            Step::Tap("Todo:buy milk"),
            Step::Tap("To-dos"),
        ],
    );
}

#[test]
fn scratch_types_and_lists_its_todos_like_the_built_in() {
    equivalent(
        "scratch",
        1440,
        1920,
        Lib::Bare,
        false,
        &[
            Step::Tap("Scratch"),
            Step::Type("hello [[a]]"),
            Step::Key(HwKey::Return),
            Step::Type("- [ ] from the scratch pad"),
            Step::Tap("ScratchPane"),
            Step::Key(HwKey::Escape),
            Step::Tap("To-dos"),
            Step::Tap("Todo:from the scratch pad"),
            Step::Tap("TodoBox:from the scratch pad"),
        ],
    );
}

#[test]
fn all_three_panes_share_the_drawer_like_the_built_in() {
    equivalent(
        "all",
        1440,
        1920,
        Lib::Rich,
        true,
        &[
            Step::Tap("Contents"),
            Step::Tap("To-dos"),
            Step::Tap("Scratch"),
            Step::Type("note to self"),
            Step::Tap("Heading:Monday"),
            Step::Tap("TodoBox:send the invoice"),
            Step::Tap("Contents"),
            Step::Tap("Scratch"),
            Step::Tap("To-dos"),
        ],
    );
}

#[test]
fn rows_that_do_not_fit_say_how_many_more_like_the_built_in() {
    equivalent(
        "crowded",
        1440,
        1920,
        Lib::Crowded,
        false,
        &[
            Step::Tap("Contents"),
            Step::Tap("To-dos"),
            Step::Tap("Filter"),
            Step::Tap("Filter"),
        ],
    );
}

#[test]
fn empty_panes_show_their_hints_like_the_built_in() {
    equivalent(
        "bare",
        1440,
        1920,
        Lib::Bare,
        false,
        &[
            Step::Tap("Contents"),
            Step::Tap("To-dos"),
            Step::Tap("Scratch"),
        ],
    );
}

#[test]
fn panes_in_preview_and_a_smaller_screen_match_the_built_in() {
    equivalent(
        "preview",
        1080,
        1440,
        Lib::Rich,
        false,
        &[
            Step::Tap("Contents"),
            Step::Tap("To-dos"),
            Step::Tap("Preview"),
            Step::Tap("Heading:Monday"),
            Step::Tap("TodoBox:buy milk"),
            Step::Tap("Pencil"),
        ],
    );
}

#[test]
fn every_drawer_width_matches_the_built_in() {
    for (tag, width) in [
        ("narrow", DrawerWidth::Narrow),
        ("medium", DrawerWidth::Medium),
    ] {
        equivalent_wide(
            tag,
            1440,
            1920,
            Lib::Rich,
            false,
            width,
            &[
                Step::Tap("Contents"),
                Step::Tap("To-dos"),
                Step::Tap("Scratch"),
                Step::Type("a side note that is long enough to wrap in a narrow drawer"),
                Step::Tap("TodoBox:call Ana about the contract"),
                Step::Tap("Preview"),
                Step::Tap("Heading:Monday"),
            ],
        );
    }
}
