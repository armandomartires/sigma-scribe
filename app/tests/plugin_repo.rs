//! Settings → Plugins → Repository: check the index, install and update a
//! plugin from a package, refuse a bad one. The repository is a local folder
//! (`SIGMA_PLUGIN_REPO`), so nothing here uses the network.

use sigma_core::pluginpkg::sha256_hex;
use sigma_core::plugins;
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Refresh};
use sigma_render::options::PLUGINS_CATEGORY;
use sigma_render::SettingsPage;
use sigma_scribe::App;
use std::path::{Path, PathBuf};

const BASE: &str = "https://armandomartires.github.io/sigma-scribe-plugins/";

fn tar(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, body) in files {
        let mut h = [0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        h[124..135].copy_from_slice(format!("{:011o}", body.len()).as_bytes());
        h[156] = b'0';
        h[257..262].copy_from_slice(b"ustar");
        out.extend_from_slice(&h);
        out.extend_from_slice(body);
        out.resize(out.len().div_ceil(512) * 512, 0);
    }
    out.extend_from_slice(&[0u8; 1024]);
    out
}

/// Writes `demo-<version>.tar` into `repo` and returns its index block.
fn package(repo: &Path, version: &str, min_app: &str) -> String {
    let conf =
        format!("id=demo\nname=Demo font\nversion={version}\nkind=font\nmin_app={min_app}\n");
    let bytes = tar(&[("plugin.conf", conf.as_bytes()), ("fonts/demo.ttf", b"ttf")]);
    let name = format!("demo-{version}.tar");
    std::fs::write(repo.join(&name), &bytes).unwrap();
    format!(
        "{conf}url={BASE}{name}\nsize={}\nsha256={}\n",
        bytes.len(),
        sha256_hex(&bytes)
    )
}

fn index(repo: &Path, serial: u64, blocks: &[String]) {
    std::fs::write(
        repo.join("index.json"),
        format!("serial={serial}\n\n{}", blocks.join("\n")),
    )
    .unwrap();
}

struct Rig {
    app: App,
    dev: DesktopDevice,
    plugins: PathBuf,
}

impl Rig {
    fn tap_on(&mut self, label: &str) {
        let (x, y) = self
            .app
            .control_centre(label)
            .unwrap_or_else(|| panic!("no control {label}"));
        if let Some(r) = self.app.handle(Event::Tap { x, y }) {
            self.app.render(&mut self.dev, r).unwrap();
        }
    }
}

#[test]
fn install_update_and_refuse() {
    let d = std::env::temp_dir().join(format!("sigma-repo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let (notes, plugins, repo) = (d.join("notes"), d.join("plugins"), d.join("repo"));
    for p in [&notes, &plugins, &repo] {
        std::fs::create_dir_all(p).unwrap();
    }
    std::fs::write(notes.join("a.md"), "hi").unwrap();
    std::env::set_var("SIGMA_PLUGIN_REPO", &repo);

    let v1 = package(&repo, "1.0.0", "1.0.0");
    index(&repo, 5, &[v1]);

    let mut app = App::new(1440, 1920, notes);
    app.set_plugins_root(plugins.clone());
    app.open(&sigma_core::NotePath::parse("a.md").unwrap());
    let mut dev = DesktopDevice::new(1440, 1920);
    app.render(&mut dev, Refresh::Full).unwrap();
    let mut r = Rig { app, dev, plugins };
    r.tap_on("Options");
    r.tap_on("Plugins");
    assert_eq!(
        r.app.settings_page,
        SettingsPage::Category(PLUGINS_CATEGORY)
    );

    // Nothing is listed until the index is checked.
    assert!(r.app.control_centre("Demo font/Install").is_none());
    r.tap_on("Check for plugins/Check");
    r.tap_on("Demo font/Install");
    assert_eq!(
        std::fs::read(r.plugins.join("demo/fonts/demo.ttf")).unwrap(),
        b"ttf"
    );
    assert!(std::fs::read_to_string(r.plugins.join("plugins.state"))
        .unwrap()
        .contains("demo=1.0.0,enabled"));
    assert_eq!(plugins::list_in(&r.plugins).len(), 1);
    assert!(r.app.control_centre("Demo font/Install").is_none());

    // A plugin that is on cannot be removed; switched off, it can, and the
    // repository offers it again.
    assert!(r
        .app
        .control_centre("Demo font \u{b7} Remove/Remove")
        .is_none());
    r.tap_on("Demo font/Off");
    r.tap_on("Demo font \u{b7} Remove/Remove");
    assert!(!r.plugins.join("demo").exists());
    assert!(plugins::list_in(&r.plugins).is_empty());
    assert!(r.app.control_centre("Demo font/Install").is_some());
    r.tap_on("Demo font/Install");
    assert_eq!(plugins::list_in(&r.plugins).len(), 1);

    // A newer version in the index offers Update, and keeps the switch.
    plugins::set_enabled_in(&r.plugins, "demo", false).unwrap();
    let v2 = package(&repo, "1.1.0", "1.0.0");
    index(&repo, 6, std::slice::from_ref(&v2));
    r.tap_on("Check for plugins/Check");
    r.tap_on("Demo font/Update");
    let l = plugins::list_in(&r.plugins);
    assert_eq!(l[0].manifest.version, plugins::Version(1, 1, 0));
    assert!(!l[0].enabled);

    // An index older than the last one is refused: the list stays as it was.
    index(&repo, 3, &[package(&repo, "9.0.0", "1.0.0")]);
    r.tap_on("Check for plugins/Check");
    assert!(r.app.control_centre("Demo font/Update").is_none());
    assert_eq!(
        std::fs::read_to_string(r.plugins.join("index.serial"))
            .unwrap()
            .trim(),
        "6"
    );

    // A package that does not match its index entry is not installed.
    let bad = v2.replace("1.1.0", "1.2.0");
    index(&repo, 7, &[bad]);
    r.tap_on("Check for plugins/Check");
    r.tap_on("Demo font/Update");
    assert_eq!(
        plugins::list_in(&r.plugins)[0].manifest.version,
        plugins::Version(1, 1, 0)
    );
    let _ = std::fs::remove_dir_all(&d);
}
