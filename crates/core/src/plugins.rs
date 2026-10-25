//! Plugin data model: manifest, repository index, installed-state file.
//!
//! Plugins are data packs only (fonts, dictionaries, themes...). Nothing here
//! touches the disk or the network; the front ends do the I/O and feed text
//! in. Like `settings.rs`, every file is plain `key=value` lines and unknown
//! keys are ignored, so an older app never stops on a newer manifest.
//! The repository index is the same format, one block per plugin, blocks
//! separated by a blank line.

use std::collections::BTreeMap;

/// Plugin format version this app understands.
pub const API_VERSION: u32 = 1;

/// The one place plugins come from: our own `sigma-scribe-plugins` repo
/// (fonts, icons, dictionaries and the rest all live there). The app never
/// reads an index or package from anywhere else.
pub const INDEX_URL: &str = "https://armandomartires.github.io/sigma-scribe-plugins/index.json";

/// Every package URL in the index must start with one of these.
pub const SOURCE_PREFIXES: [&str; 2] = [
    "https://armandomartires.github.io/sigma-scribe-plugins/",
    "https://github.com/armandomartires/sigma-scribe-plugins/releases/download/",
];

/// True when `url` is inside the plugins repo.
#[must_use]
pub fn trusted_source(url: &str) -> bool {
    SOURCE_PREFIXES
        .iter()
        .any(|p| url.starts_with(p) && !url[p.len()..].contains(".."))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginKind {
    Font,
    Dictionary,
    HwrModel,
    WordList,
    Theme,
    Translation,
    Layout,
    Icons,
    /// A WASM screen plugin (`entry` names the module).
    Screen,
}

impl PluginKind {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "font" => Self::Font,
            "dictionary" => Self::Dictionary,
            "hwr-model" => Self::HwrModel,
            "word-list" => Self::WordList,
            "theme" => Self::Theme,
            "translation" => Self::Translation,
            "layout" => Self::Layout,
            "icons" => Self::Icons,
            "screen" => Self::Screen,
            _ => return None,
        })
    }

    /// The kind in English, for the Settings list.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Font => "Font",
            Self::Dictionary => "Dictionary",
            Self::HwrModel => "Handwriting model",
            Self::WordList => "Word list",
            Self::Theme => "Theme",
            Self::Translation => "Translation",
            Self::Layout => "Keyboard layout",
            Self::Icons => "Icons",
            Self::Screen => "Screen",
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Font => "font",
            Self::Dictionary => "dictionary",
            Self::HwrModel => "hwr-model",
            Self::WordList => "word-list",
            Self::Theme => "theme",
            Self::Translation => "translation",
            Self::Layout => "layout",
            Self::Icons => "icons",
            Self::Screen => "screen",
        }
    }
}

/// A `major.minor.patch` version. Missing parts count as 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Version(pub u32, pub u32, pub u32);

impl Version {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let mut it = s.trim().split('.');
        let mut part = || -> Option<u32> {
            match it.next() {
                Some(p) => p.parse().ok(),
                None => Some(0),
            }
        };
        let v = Self(part()?, part()?, part()?);
        it.next().is_none().then_some(v)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// An id is a lowercase slug: it becomes a folder name, so it must not be
/// able to point anywhere else.
#[must_use]
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 48
        && !id.starts_with('-')
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// True when `path` is a safe relative path inside a plugin folder: no
/// absolute paths, no `..`, no backslashes, no empty or NUL components.
#[must_use]
pub fn safe_rel_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains('\0')
        && path
            .split('/')
            .all(|c| !c.is_empty() && c != "." && c != "..")
}

/// What a plugin says about itself (`plugin.conf` inside the package, and
/// the same fields in an index block).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub kind: PluginKind,
    pub description: String,
    /// Oldest app version that can use it.
    pub min_app: Version,
    pub api: u32,
    /// The module of a [`PluginKind::Screen`] plugin, relative to its folder.
    pub entry: Option<String>,
}

fn fields(text: &str) -> BTreeMap<&str, &str> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.is_empty() || l.starts_with('#') {
                return None;
            }
            let (k, v) = l.split_once('=')?;
            Some((k.trim(), v.trim()))
        })
        .collect()
}

impl Manifest {
    /// Parse one manifest. `None` when a required field is missing or bad,
    /// the id is not a valid slug, or the plugin needs a newer API.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let f = fields(text);
        let id = (*f.get("id")?).to_string();
        if !valid_id(&id) {
            return None;
        }
        let api = f.get("api").map_or(Some(API_VERSION), |v| v.parse().ok())?;
        if api > API_VERSION {
            return None;
        }
        let kind = PluginKind::parse(f.get("kind")?)?;
        let entry = f.get("entry").map(|e| (*e).to_string());
        if entry.as_deref().is_some_and(|e| !safe_rel_path(e)) {
            return None;
        }
        // A screen plugin without a module is not a plugin.
        if kind == PluginKind::Screen && entry.is_none() {
            return None;
        }
        Some(Self {
            entry,
            kind,
            name: f
                .get("name")
                .map_or_else(|| id.clone(), |s| (*s).to_string()),
            id,
            version: Version::parse(f.get("version")?)?,
            description: f.get("description").unwrap_or(&"").to_string(),
            min_app: f
                .get("min_app")
                .map_or(Some(Version::default()), |v| Version::parse(v))?,
            api,
        })
    }

    #[must_use]
    pub fn compatible_with(&self, app: Version) -> bool {
        self.min_app <= app
    }
}

/// One downloadable plugin in the repository index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexEntry {
    pub manifest: Manifest,
    pub url: String,
    pub size: u64,
    /// Lowercase hex sha256 of the package file.
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Index {
    /// Rises with every published index; the app refuses to go backwards.
    pub serial: u64,
    pub entries: Vec<IndexEntry>,
}

fn valid_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Index {
    /// Parse an index. The header block holds `serial=`; bad entries are
    /// skipped so one broken plugin never hides the rest.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let text = text.replace("\r\n", "\n");
        let mut blocks = text.split("\n\n");
        let serial = blocks
            .next()
            .and_then(|h| fields(h).get("serial").and_then(|s| s.parse().ok()))
            .unwrap_or(0);
        let entries = blocks
            .filter_map(|b| {
                let f = fields(b);
                let url = (*f.get("url")?).to_string();
                let sha256 = f.get("sha256")?.to_ascii_lowercase();
                if !trusted_source(&url) || !valid_sha256(&sha256) {
                    return None;
                }
                Some(IndexEntry {
                    manifest: Manifest::parse(b)?,
                    size: f.get("size")?.parse().ok()?,
                    url,
                    sha256,
                })
            })
            .collect();
        Self { serial, entries }
    }

    #[must_use]
    pub fn find(&self, id: &str) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| e.manifest.id == id)
    }
}

/// An installed plugin as recorded in `plugins.state`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub id: String,
    pub version: Version,
    pub enabled: bool,
}

/// `plugins.state`: one `id=version,enabled|disabled` line per plugin.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct State {
    pub plugins: Vec<Installed>,
}

impl State {
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for (id, v) in fields(text) {
            let Some((ver, flag)) = v.split_once(',') else {
                continue;
            };
            let Some(version) = Version::parse(ver) else {
                continue;
            };
            if valid_id(id) && s.get(id).is_none() {
                s.plugins.push(Installed {
                    id: id.to_string(),
                    version,
                    enabled: flag.trim() != "disabled",
                });
            }
        }
        s
    }

    #[must_use]
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        for p in &self.plugins {
            let Version(a, b, c) = p.version;
            let flag = if p.enabled { "enabled" } else { "disabled" };
            out.push_str(&format!("{}={a}.{b}.{c},{flag}\n", p.id));
        }
        out
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Installed> {
        self.plugins.iter().find(|p| p.id == id)
    }

    /// Record an install or update. A reinstall keeps the enabled flag; a
    /// new plugin starts enabled.
    pub fn install(&mut self, id: &str, version: Version) {
        match self.plugins.iter_mut().find(|p| p.id == id) {
            Some(p) => p.version = version,
            None => self.plugins.push(Installed {
                id: id.to_string(),
                version,
                enabled: true,
            }),
        }
    }

    /// Returns false when the plugin is not installed.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        self.plugins
            .iter_mut()
            .find(|p| p.id == id)
            .map(|p| p.enabled = enabled)
            .is_some()
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let n = self.plugins.len();
        self.plugins.retain(|p| p.id != id);
        self.plugins.len() != n
    }

    pub fn enabled(&self) -> impl Iterator<Item = &Installed> {
        self.plugins.iter().filter(|p| p.enabled)
    }
}

/// Where plugins are installed: `SIGMA_PLUGINS`, else the folder beside the
/// app on the Kobo. Each plugin is `<root>/<id>/`, and `<root>/plugins.state`
/// records what is installed and enabled.
#[must_use]
pub fn plugins_root() -> std::path::PathBuf {
    std::env::var_os("SIGMA_PLUGINS").map_or_else(
        || std::path::PathBuf::from("/mnt/onboard/.adds/sigma-scribe/plugins"),
        std::path::PathBuf::from,
    )
}

/// The files named `rel` (a path inside a plugin, such as `fonts/cjk.ttf`)
/// that enabled plugins under `root` provide, in install order. Missing
/// state or files give an empty list, so callers fall back to their old
/// paths.
#[must_use]
pub fn enabled_files_in(root: &std::path::Path, rel: &str) -> Vec<std::path::PathBuf> {
    if !safe_rel_path(rel) {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(root.join("plugins.state")) else {
        return Vec::new();
    };
    State::parse(&text)
        .enabled()
        .map(|p| root.join(&p.id).join(rel))
        .filter(|p| p.is_file())
        .collect()
}

/// The module of the enabled screen plugin `id` under `root`: its folder,
/// manifest and the module bytes' path. `None` when it is not installed,
/// disabled, not a screen plugin, or its module file is missing.
#[must_use]
pub fn enabled_screen_in(
    root: &std::path::Path,
    id: &str,
) -> Option<(Manifest, std::path::PathBuf)> {
    if !valid_id(id) {
        return None;
    }
    let state = State::parse(&std::fs::read_to_string(root.join("plugins.state")).ok()?);
    if !state.get(id)?.enabled {
        return None;
    }
    let m = Manifest::parse(&std::fs::read_to_string(root.join(id).join("plugin.conf")).ok()?)?;
    if m.id != id || m.kind != PluginKind::Screen {
        return None;
    }
    let wasm = root.join(id).join(m.entry.as_deref()?);
    wasm.is_file().then_some((m, wasm))
}

/// A plugin found on disk, for the Settings list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub manifest: Manifest,
    /// On in `plugins.state`. A plugin the file does not mention is off.
    pub enabled: bool,
}

/// The plugins installed under `root`: every folder with a valid
/// `plugin.conf` whose id is the folder's name, sorted by name.
#[must_use]
pub fn list_in(root: &std::path::Path) -> Vec<Listed> {
    let state = std::fs::read_to_string(root.join("plugins.state"))
        .map(|t| State::parse(&t))
        .unwrap_or_default();
    let Ok(dir) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out: Vec<Listed> = dir
        .filter_map(Result::ok)
        .filter_map(|e| {
            let id = e.file_name().into_string().ok()?;
            let m = Manifest::parse(&std::fs::read_to_string(e.path().join("plugin.conf")).ok()?)?;
            (m.id == id).then(|| Listed {
                enabled: state.get(&id).is_some_and(|p| p.enabled),
                manifest: m,
            })
        })
        .collect();
    out.sort_by(|a, b| {
        a.manifest
            .name
            .to_lowercase()
            .cmp(&b.manifest.name.to_lowercase())
            .then_with(|| a.manifest.id.cmp(&b.manifest.id))
    });
    out
}

/// [`list_in`] for the installed plugins folder.
#[must_use]
pub fn list() -> Vec<Listed> {
    list_in(&plugins_root())
}

/// Turns plugin `id` on or off in `plugins.state`, atomically. A plugin that
/// is on disk but not yet in the file is added.
///
/// # Errors
/// `NotFound` when there is no valid plugin of that id under `root`, or any
/// filesystem error (the old file is left intact).
pub fn set_enabled_in(root: &std::path::Path, id: &str, enabled: bool) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    let manifest = valid_id(id)
        .then(|| std::fs::read_to_string(root.join(id).join("plugin.conf")).ok())
        .flatten()
        .and_then(|t| Manifest::parse(&t))
        .filter(|m| m.id == id)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "no such plugin"))?;
    let path = root.join("plugins.state");
    let mut state = std::fs::read_to_string(&path)
        .map(|t| State::parse(&t))
        .unwrap_or_default();
    if state.get(id).is_none() {
        state.install(id, manifest.version);
    }
    state.set_enabled(id, enabled);
    crate::store::write_atomic(&path, &state.serialize())
}

/// [`enabled_files_in`] for the installed plugins folder.
#[must_use]
pub fn enabled_files(rel: &str) -> Vec<std::path::PathBuf> {
    enabled_files_in(&plugins_root(), rel)
}

/// What the repository offers for one plugin, given what is installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    NotInstalled,
    UpToDate,
    UpdateAvailable,
    /// Needs a newer app.
    Incompatible,
}

#[must_use]
pub fn availability(entry: &IndexEntry, state: &State, app: Version) -> Availability {
    if !entry.manifest.compatible_with(app) {
        return Availability::Incompatible;
    }
    match state.get(&entry.manifest.id) {
        None => Availability::NotInstalled,
        Some(p) if p.version < entry.manifest.version => Availability::UpdateAvailable,
        Some(_) => Availability::UpToDate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn manifest(extra: &str) -> String {
        format!("id=cjk-font\nname=CJK font\nversion=1.2.0\nkind=font\n{extra}")
    }

    #[test]
    fn version_orders_numerically() {
        assert!(Version::parse("1.10.0") > Version::parse("1.9.9"));
        assert_eq!(Version::parse("2"), Some(Version(2, 0, 0)));
        assert_eq!(Version::parse("1.2.3.4"), None);
        assert_eq!(Version::parse("x"), None);
    }

    #[test]
    fn ids_cannot_escape_the_plugins_folder() {
        assert!(valid_id("cjk-font"));
        for bad in ["", "../x", "A", "a/b", "-a", "a b", "a.b"] {
            assert!(!valid_id(bad), "{bad}");
        }
    }

    #[test]
    fn rel_paths_reject_traversal() {
        assert!(safe_rel_path("fonts/cjk.ttf"));
        for bad in [
            "",
            "/etc/passwd",
            "../x",
            "a/../b",
            "a//b",
            "a\\b",
            "./a",
            "a/",
        ] {
            assert!(!safe_rel_path(bad), "{bad}");
        }
    }

    #[test]
    fn manifest_parses_with_defaults() {
        let m = Manifest::parse(&manifest("")).unwrap();
        assert_eq!(m.id, "cjk-font");
        assert_eq!(m.kind, PluginKind::Font);
        assert_eq!(m.min_app, Version::default());
        assert_eq!(m.api, API_VERSION);
    }

    #[test]
    fn manifest_rejects_bad_input() {
        assert!(Manifest::parse("name=x").is_none());
        assert!(Manifest::parse(&manifest("api=2\n")).is_none());
        assert!(Manifest::parse("id=../x\nversion=1\nkind=font").is_none());
        assert!(Manifest::parse("id=a\nversion=1\nkind=script").is_none());
        // Unknown keys are ignored.
        assert!(Manifest::parse(&manifest("future=1\n")).is_some());
    }

    #[test]
    fn index_only_accepts_our_repo() {
        let ok = format!("{}cjk-font-1.2.0.sgp", SOURCE_PREFIXES[0]);
        let ok2 = format!("{}v1/cjk-font-1.2.0.sgp", SOURCE_PREFIXES[1]);
        let text = format!(
            "serial=7\n\n{m}url={ok}\nsize=10\nsha256={SHA}\n\n\
             {m}url=https://example.org/a.sgp\nsize=10\nsha256={SHA}\n\n\
             {m}url=http://armandomartires.github.io/sigma-scribe-plugins/a.sgp\nsize=10\nsha256={SHA}\n\n\
             {m}url=https://armandomartires.github.io/sigma-scribe-plugins/../evil.sgp\nsize=10\nsha256={SHA}\n\n\
             {m}url=https://armandomartires.github.io/sigma-scribe-plugins-evil/a.sgp\nsize=10\nsha256={SHA}\n\n\
             {m}url={ok2}\nsize=10\nsha256=zz\n",
            m = manifest("")
        );
        let idx = Index::parse(&text);
        assert_eq!(idx.serial, 7);
        assert_eq!(idx.entries.len(), 1);
        assert_eq!(idx.find("cjk-font").unwrap().url, ok);
        assert!(trusted_source(&ok2));
        assert!(INDEX_URL.starts_with(SOURCE_PREFIXES[0]));
    }

    #[test]
    fn icons_is_a_kind() {
        assert_eq!(PluginKind::parse("icons"), Some(PluginKind::Icons));
        assert_eq!(PluginKind::Icons.as_str(), "icons");
    }

    #[test]
    fn state_round_trips_and_tracks_flags() {
        let mut s = State::default();
        s.install("cjk-font", Version(1, 2, 0));
        s.install("skk", Version(1, 0, 0));
        assert!(s.set_enabled("skk", false));
        assert!(!s.set_enabled("nope", false));
        s.install("skk", Version(1, 1, 0)); // update keeps disabled
        let back = State::parse(&s.serialize());
        assert_eq!(back, s);
        assert!(!back.get("skk").unwrap().enabled);
        assert_eq!(back.enabled().count(), 1);
        assert!(s.remove("skk"));
        assert!(s.get("skk").is_none());
    }

    #[test]
    fn enabled_files_skip_disabled_missing_and_unsafe() {
        let root = std::env::temp_dir().join(format!("sigma-plugins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for id in ["a", "b", "c"] {
            std::fs::create_dir_all(root.join(id).join("fonts")).unwrap();
        }
        std::fs::write(root.join("a/fonts/cjk.ttf"), b"a").unwrap();
        std::fs::write(root.join("b/fonts/cjk.ttf"), b"b").unwrap();
        // c is enabled but ships no font.
        std::fs::write(
            root.join("plugins.state"),
            "a=1.0.0,enabled\nb=1.0.0,disabled\nc=1.0.0,enabled\n",
        )
        .unwrap();
        let got = enabled_files_in(&root, "fonts/cjk.ttf");
        assert_eq!(got, vec![root.join("a/fonts/cjk.ttf")]);
        assert!(enabled_files_in(&root, "../plugins.state").is_empty());
        assert!(enabled_files_in(&root.join("nope"), "fonts/cjk.ttf").is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn screen_plugins_need_a_safe_entry() {
        let ok = Manifest::parse("id=graph-map\nversion=1\nkind=screen\nentry=graph.wasm").unwrap();
        assert_eq!(ok.kind, PluginKind::Screen);
        assert_eq!(ok.entry.as_deref(), Some("graph.wasm"));
        assert!(Manifest::parse("id=g\nversion=1\nkind=screen").is_none());
        assert!(Manifest::parse("id=g\nversion=1\nkind=screen\nentry=../x.wasm").is_none());
        assert!(Manifest::parse("id=g\nversion=1\nkind=font\nentry=/etc/x").is_none());
        // Data plugins need no entry.
        assert_eq!(Manifest::parse(&manifest("")).unwrap().entry, None);
    }

    #[test]
    fn enabled_screen_finds_only_enabled_screen_plugins_with_a_module() {
        let root = std::env::temp_dir().join(format!("sigma-screens-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for id in ["graph-map", "off", "font", "nomod", "other"] {
            std::fs::create_dir_all(root.join(id)).unwrap();
        }
        let conf = |id: &str, kind: &str| {
            std::fs::write(
                root.join(id).join("plugin.conf"),
                format!("id={id}\nversion=1.0.0\nkind={kind}\nentry=m.wasm\n"),
            )
            .unwrap();
        };
        conf("graph-map", "screen");
        conf("off", "screen");
        conf("font", "font");
        conf("nomod", "screen");
        // plugin.conf names a different id than its folder.
        std::fs::write(
            root.join("other/plugin.conf"),
            "id=graph-map\nversion=1\nkind=screen\nentry=m.wasm",
        )
        .unwrap();
        for id in ["graph-map", "off", "font", "other"] {
            std::fs::write(root.join(id).join("m.wasm"), b"\0asm").unwrap();
        }
        std::fs::write(
            root.join("plugins.state"),
            "graph-map=1.0.0,enabled\noff=1.0.0,disabled\nfont=1.0.0,enabled\nnomod=1.0.0,enabled\nother=1.0.0,enabled\n",
        )
        .unwrap();
        let (m, wasm) = enabled_screen_in(&root, "graph-map").unwrap();
        assert_eq!(m.id, "graph-map");
        assert_eq!(wasm, root.join("graph-map/m.wasm"));
        for id in ["off", "font", "nomod", "other", "missing", "../x"] {
            assert!(enabled_screen_in(&root, id).is_none(), "{id}");
        }
        assert!(enabled_screen_in(&root.join("nope"), "graph-map").is_none());
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn plugin_folder(root: &std::path::Path, id: &str, name: &str, kind: &str) {
        std::fs::create_dir_all(root.join(id)).unwrap();
        std::fs::write(
            root.join(id).join("plugin.conf"),
            format!("id={id}\nname={name}\nversion=1.2.0\nkind={kind}\nentry=m.wasm\n"),
        )
        .unwrap();
    }

    #[test]
    fn list_in_finds_valid_folders_sorted_with_their_state() {
        let root = std::env::temp_dir().join(format!("sigma-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        plugin_folder(&root, "zeta", "Zeta pack", "font");
        plugin_folder(&root, "graph-map", "Graph", "screen");
        plugin_folder(&root, "off", "alpha", "theme");
        // Not plugins: a folder without a manifest, one whose id differs from
        // its name, a stray file.
        std::fs::create_dir_all(root.join("empty")).unwrap();
        plugin_folder(&root, "liar", "Liar", "font");
        std::fs::write(
            root.join("liar/plugin.conf"),
            "id=other\nversion=1\nkind=font",
        )
        .unwrap();
        std::fs::write(root.join("notes.txt"), "x").unwrap();
        std::fs::write(
            root.join("plugins.state"),
            "graph-map=1.0.0,enabled\noff=1.0.0,disabled\n",
        )
        .unwrap();
        let got: Vec<(String, bool)> = list_in(&root)
            .into_iter()
            .map(|l| (l.manifest.id, l.enabled))
            .collect();
        assert_eq!(
            got,
            vec![
                ("off".to_string(), false), // "alpha"
                ("graph-map".to_string(), true),
                ("zeta".to_string(), false), // on disk, not in the file
            ]
        );
        assert!(list_in(&root.join("nope")).is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn set_enabled_in_writes_the_state_file() {
        let root = std::env::temp_dir().join(format!("sigma-toggle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        plugin_folder(&root, "graph-map", "Graph", "screen");
        plugin_folder(&root, "other", "Other", "font");
        // Not in the file yet: turning it on adds it.
        set_enabled_in(&root, "graph-map", true).unwrap();
        assert!(list_in(&root)
            .iter()
            .any(|l| l.manifest.id == "graph-map" && l.enabled));
        assert!(
            enabled_screen_in(&root, "graph-map").is_none(),
            "no module file yet"
        );
        std::fs::write(root.join("graph-map/m.wasm"), b"\0asm").unwrap();
        assert!(enabled_screen_in(&root, "graph-map").is_some());
        // Off and on again; other plugins and versions are left alone.
        set_enabled_in(&root, "other", true).unwrap();
        set_enabled_in(&root, "graph-map", false).unwrap();
        assert!(enabled_screen_in(&root, "graph-map").is_none());
        let state = State::parse(&std::fs::read_to_string(root.join("plugins.state")).unwrap());
        assert!(state.get("other").unwrap().enabled);
        assert!(!state.get("graph-map").unwrap().enabled);
        assert_eq!(state.get("graph-map").unwrap().version, Version(1, 2, 0));
        // Unknown, unsafe or manifest-less ids are errors and change nothing.
        let before = std::fs::read_to_string(root.join("plugins.state")).unwrap();
        for id in ["missing", "../x", "", "Graph-Map"] {
            assert!(set_enabled_in(&root, id, true).is_err(), "{id:?}");
        }
        assert_eq!(
            std::fs::read_to_string(root.join("plugins.state")).unwrap(),
            before
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn availability_follows_state_and_app_version() {
        let m = Manifest::parse(&manifest("min_app=1.5.0\n")).unwrap();
        let e = IndexEntry {
            manifest: m,
            url: format!("{}x.sgp", SOURCE_PREFIXES[0]),
            size: 1,
            sha256: SHA.into(),
        };
        let mut s = State::default();
        let app = Version(1, 11, 1);
        assert_eq!(availability(&e, &s, app), Availability::NotInstalled);
        s.install("cjk-font", Version(1, 1, 0));
        assert_eq!(availability(&e, &s, app), Availability::UpdateAvailable);
        s.install("cjk-font", Version(1, 2, 0));
        assert_eq!(availability(&e, &s, app), Availability::UpToDate);
        assert_eq!(
            availability(&e, &s, Version(1, 4, 0)),
            Availability::Incompatible
        );
    }
}
