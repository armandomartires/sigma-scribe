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

/// What a plugin provides, from the manifest's `kind=` line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginKind {
    /// A font (`font`).
    Font,
    /// A dictionary (`dictionary`).
    Dictionary,
    /// A handwriting recognition model (`hwr-model`).
    HwrModel,
    /// A word list (`word-list`).
    WordList,
    /// A theme (`theme`).
    Theme,
    /// A translation (`translation`).
    Translation,
    /// A keyboard layout (`layout`).
    Layout,
    /// An icon set (`icons`).
    Icons,
    /// A WASM screen plugin (`entry` names the module).
    Screen,
    /// A WASM plugin that draws one pane of the side drawer (`entry` names
    /// the module, `slot` the pane it replaces).
    Pane,
}

/// The drawer pane a [`PluginKind::Pane`] plugin provides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneSlot {
    /// The contents pane (`contents`).
    Contents,
    /// The to-dos pane (`todos`).
    Todos,
    /// The scratch pane (`scratch`).
    Scratch,
}

impl PaneSlot {
    /// The slot named by a manifest's `slot=` value; `None` for any other
    /// value.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "contents" => Self::Contents,
            "todos" => Self::Todos,
            "scratch" => Self::Scratch,
            _ => return None,
        })
    }
}

impl PluginKind {
    /// The kind named by a manifest's `kind=` value (`font`, `hwr-model`...);
    /// `None` for an unknown kind.
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
            "pane" => Self::Pane,
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
            Self::Pane => "Drawer pane",
        }
    }

    /// The kind as written in a manifest; the inverse of
    /// [`PluginKind::parse`].
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
            Self::Pane => "pane",
        }
    }
}

/// A `major.minor.patch` version. Missing parts count as 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Version(pub u32, pub u32, pub u32);

impl Version {
    /// Parses `1`, `1.2` or `1.2.3`. `None` for more than three parts or a
    /// part that is not a number.
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
    /// The plugin's id, a slug accepted by [`valid_id`].
    pub id: String,
    /// The display name; the id when the manifest gives none.
    pub name: String,
    /// The plugin's own version.
    pub version: Version,
    /// What the plugin provides.
    pub kind: PluginKind,
    /// A short description; empty when the manifest gives none.
    pub description: String,
    /// Oldest app version that can use it.
    pub min_app: Version,
    /// The plugin format version it was written for; never above
    /// [`API_VERSION`].
    pub api: u32,
    /// The module of a [`PluginKind::Screen`] plugin, relative to its folder.
    pub entry: Option<String>,
    /// The drawer pane of a [`PluginKind::Pane`] plugin.
    pub slot: Option<PaneSlot>,
    /// The plugin's own settings (`setting=` lines), shown on its page in
    /// Settings → Plugins while it is on.
    pub settings: Vec<SettingDecl>,
}

/// A setting a plugin declares in `plugin.conf`:
/// `setting=key|Label|Option A,Option B|Default`. A choice between two to
/// four options; the host draws it and keeps the value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingDecl {
    /// The key the value is stored under: lowercase letters, digits and `_`.
    pub key: String,
    /// The label shown in Settings.
    pub label: String,
    /// The two to four choices, as shown and as stored.
    pub options: Vec<String>,
    /// Index into `options`.
    pub default: usize,
}

/// Most settings a plugin may declare.
pub const MAX_PLUGIN_SETTINGS: usize = 8;

impl SettingDecl {
    /// Parses the value of one `setting=` line; `None` when it is not valid.
    #[must_use]
    pub fn parse(v: &str) -> Option<Self> {
        let mut it = v.split('|').map(str::trim);
        let (key, label, opts, default) = (it.next()?, it.next()?, it.next()?, it.next()?);
        if it.next().is_some()
            || key.is_empty()
            || key.len() > 24
            || !key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            || label.is_empty()
            || label.len() > 40
        {
            return None;
        }
        let options: Vec<String> = opts.split(',').map(|o| o.trim().to_owned()).collect();
        if !(2..=4).contains(&options.len()) || options.iter().any(|o| o.is_empty() || o.len() > 24)
        {
            return None;
        }
        let default = options.iter().position(|o| o == default).unwrap_or(0);
        Some(Self {
            key: key.to_owned(),
            label: label.to_owned(),
            options,
            default,
        })
    }
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
        // A screen or pane plugin without a module is not a plugin.
        if matches!(kind, PluginKind::Screen | PluginKind::Pane) && entry.is_none() {
            return None;
        }
        let slot = match (kind, f.get("slot")) {
            (PluginKind::Pane, Some(v)) => Some(PaneSlot::parse(v)?),
            (PluginKind::Pane, None) => return None,
            _ => None,
        };
        let mut settings: Vec<SettingDecl> = Vec::new();
        for d in text
            .lines()
            .filter_map(|l| l.trim().strip_prefix("setting="))
            .filter_map(SettingDecl::parse)
        {
            if settings.len() < MAX_PLUGIN_SETTINGS && settings.iter().all(|x| x.key != d.key) {
                settings.push(d);
            }
        }
        Some(Self {
            settings,
            entry,
            slot,
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

    /// Whether an app of version `app` can use this plugin.
    #[must_use]
    pub fn compatible_with(&self, app: Version) -> bool {
        self.min_app <= app
    }
}

/// One downloadable plugin in the repository index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexEntry {
    /// The plugin's manifest fields from its index block.
    pub manifest: Manifest,
    /// Where the package is downloaded from; always a [`trusted_source`].
    pub url: String,
    /// The package size in bytes, as the index states it.
    pub size: u64,
    /// Lowercase hex sha256 of the package file.
    pub sha256: String,
}

/// The repository index: what can be downloaded.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Index {
    /// Rises with every published index; the app refuses to go backwards.
    pub serial: u64,
    /// The valid entries, in index order.
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

    /// The first entry for plugin `id`.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| e.manifest.id == id)
    }
}

/// An installed plugin as recorded in `plugins.state`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    /// The plugin's id.
    pub id: String,
    /// The installed version.
    pub version: Version,
    /// Whether the plugin is on.
    pub enabled: bool,
}

/// `plugins.state`: one `id=version,enabled|disabled` line per plugin.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct State {
    /// The installed plugins, at most one per id.
    pub plugins: Vec<Installed>,
}

impl State {
    /// Parses `plugins.state`. Lines without a valid id, version and flag
    /// are skipped; any flag other than `disabled` counts as enabled. When
    /// an id has several valid lines the first wins. The result is sorted
    /// by id.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for l in text.lines().map(str::trim) {
            if l.is_empty() || l.starts_with('#') {
                continue;
            }
            let Some((id, v)) = l.split_once('=') else {
                continue;
            };
            let id = id.trim();
            let Some((ver, flag)) = v.trim().split_once(',') else {
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
        s.plugins.sort_by(|a, b| a.id.cmp(&b.id));
        s
    }

    /// The state as `plugins.state` text, one line per plugin.
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

    /// The record of plugin `id`, if it is installed.
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

    /// Turns plugin `id` on or off. Returns false when the plugin is not
    /// installed.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        self.plugins
            .iter_mut()
            .find(|p| p.id == id)
            .map(|p| p.enabled = enabled)
            .is_some()
    }

    /// Forgets plugin `id`. Returns false when it was not installed.
    pub fn remove(&mut self, id: &str) -> bool {
        let n = self.plugins.len();
        self.plugins.retain(|p| p.id != id);
        self.plugins.len() != n
    }

    /// The plugins that are on, in record order.
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
/// that enabled plugins under `root` provide, sorted by plugin id. Missing
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

/// The enabled pane plugin for `slot` under `root`: its manifest and module
/// path. `None` when none is installed and enabled with its module present.
/// If several qualify, the first by id wins.
#[must_use]
pub fn enabled_pane_in(
    root: &std::path::Path,
    slot: PaneSlot,
) -> Option<(Manifest, std::path::PathBuf)> {
    let state = State::parse(&std::fs::read_to_string(root.join("plugins.state")).ok()?);
    let mut ids: Vec<&str> = state.enabled().map(|p| p.id.as_str()).collect();
    ids.sort_unstable();
    ids.into_iter().find_map(|id| {
        if !valid_id(id) {
            return None;
        }
        let m = Manifest::parse(&std::fs::read_to_string(root.join(id).join("plugin.conf")).ok()?)?;
        if m.id != id || m.kind != PluginKind::Pane || m.slot != Some(slot) {
            return None;
        }
        let wasm = root.join(id).join(m.entry.as_deref()?);
        wasm.is_file().then_some((m, wasm))
    })
}

/// The file holding a plugin's setting values.
fn settings_path(root: &std::path::Path, id: &str) -> std::path::PathBuf {
    root.join(id).join("settings")
}

/// The chosen option of each of `m`'s declared settings, as an index into
/// its options: what `<root>/<id>/settings` says, else the default. Values
/// that are not one of the options (an older or hand-edited file) count as
/// the default.
#[must_use]
pub fn settings_in(root: &std::path::Path, m: &Manifest) -> Vec<usize> {
    let text = valid_id(&m.id)
        .then(|| std::fs::read_to_string(settings_path(root, &m.id)).ok())
        .flatten()
        .unwrap_or_default();
    let saved = fields(&text);
    m.settings
        .iter()
        .map(|d| {
            saved
                .get(d.key.as_str())
                .and_then(|v| d.options.iter().position(|o| o == v))
                .unwrap_or(d.default)
        })
        .collect()
}

/// Sets one declared setting of plugin `m` to option number `option`, keeping
/// the file's other values. Written atomically.
///
/// # Errors
/// `NotFound` for a setting the plugin did not declare, `InvalidInput` for an
/// option out of range, or any filesystem error.
pub fn set_setting_in(
    root: &std::path::Path,
    m: &Manifest,
    key: &str,
    option: usize,
) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    let d = m
        .settings
        .iter()
        .find(|d| d.key == key)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "no such setting"))?;
    let value = d
        .options
        .get(option)
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "no such option"))?;
    if !valid_id(&m.id) {
        return Err(Error::new(ErrorKind::NotFound, "no such plugin"));
    }
    let path = settings_path(root, &m.id);
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = old
        .lines()
        .filter(|l| l.split_once('=').is_none_or(|(k, _)| k.trim() != key))
        .map(str::to_owned)
        .collect();
    lines.push(format!("{key}={value}"));
    crate::store::write_atomic(&path, &(lines.join("\n") + "\n"))
}

/// The manifest of plugin `id` under `root`, when there is a valid one.
#[must_use]
pub fn manifest_in(root: &std::path::Path, id: &str) -> Option<Manifest> {
    valid_id(id)
        .then(|| std::fs::read_to_string(root.join(id).join("plugin.conf")).ok())
        .flatten()
        .and_then(|t| Manifest::parse(&t))
        .filter(|m| m.id == id)
}

/// A plugin found on disk, for the Settings list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// The plugin's `plugin.conf`.
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
    /// Not installed.
    NotInstalled,
    /// Installed, and the index has no newer version.
    UpToDate,
    /// Installed, and the index has a newer version.
    UpdateAvailable,
    /// Needs a newer app.
    Incompatible,
}

/// What the index offers for `entry`, given the installed `state` and the
/// app version `app`. An incompatible entry is [`Availability::Incompatible`]
/// whether or not it is installed.
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
    fn state_keeps_the_first_valid_line_per_id() {
        let s = State::parse(
            "skk=1.0.0,enabled\n\
             cjk-font=bad\n\
             cjk-font=1.2.0,disabled\n\
             cjk-font=garbage\n\
             cjk-font=1.3.0,enabled\n\
             skk=2.0.0,disabled\n",
        );
        let ids: Vec<&str> = s.plugins.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["cjk-font", "skk"]);
        let cjk = s.get("cjk-font").unwrap();
        assert_eq!((cjk.version, cjk.enabled), (Version(1, 2, 0), false));
        let skk = s.get("skk").unwrap();
        assert_eq!((skk.version, skk.enabled), (Version(1, 0, 0), true));
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
    fn pane_plugins_need_a_slot_and_a_safe_entry() {
        let ok = Manifest::parse("id=t\nversion=1\nkind=pane\nentry=t.wasm\nslot=todos").unwrap();
        assert_eq!(ok.kind, PluginKind::Pane);
        assert_eq!(ok.slot, Some(PaneSlot::Todos));
        assert_eq!(PluginKind::Pane.as_str(), "pane");
        assert!(Manifest::parse("id=t\nversion=1\nkind=pane\nentry=t.wasm").is_none());
        assert!(Manifest::parse("id=t\nversion=1\nkind=pane\nentry=t.wasm\nslot=x").is_none());
        assert!(Manifest::parse("id=t\nversion=1\nkind=pane\nslot=todos").is_none());
        assert!(Manifest::parse("id=t\nversion=1\nkind=pane\nentry=../t\nslot=todos").is_none());
        // Other kinds ignore a stray slot.
        assert_eq!(
            Manifest::parse("id=t\nversion=1\nkind=font\nslot=todos")
                .unwrap()
                .slot,
            None
        );
    }

    #[test]
    fn enabled_pane_finds_the_enabled_plugin_for_the_slot() {
        let root = std::env::temp_dir().join(format!("sigma-panes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let conf = |id: &str, slot: &str, wasm: bool| {
            std::fs::create_dir_all(root.join(id)).unwrap();
            std::fs::write(
                root.join(id).join("plugin.conf"),
                format!("id={id}\nversion=1\nkind=pane\nentry=m.wasm\nslot={slot}"),
            )
            .unwrap();
            if wasm {
                std::fs::write(root.join(id).join("m.wasm"), b"\0asm").unwrap();
            }
        };
        conf("todo-pane", "todos", true);
        conf("scratch-pane", "scratch", true);
        conf("off-todos", "todos", true);
        conf("nomod", "todos", false);
        std::fs::write(
            root.join("plugins.state"),
            "todo-pane=1.0.0,enabled\nscratch-pane=1.0.0,enabled\noff-todos=1.0.0,disabled\nnomod=1.0.0,enabled\n",
        )
        .unwrap();
        let (m, wasm) = enabled_pane_in(&root, PaneSlot::Todos).unwrap();
        assert_eq!(
            (m.id.as_str(), wasm),
            ("todo-pane", root.join("todo-pane/m.wasm"))
        );
        assert_eq!(
            enabled_pane_in(&root, PaneSlot::Scratch).unwrap().0.id,
            "scratch-pane"
        );
        std::fs::write(root.join("plugins.state"), "todo-pane=1.0.0,disabled\n").unwrap();
        assert!(enabled_pane_in(&root, PaneSlot::Todos).is_none());
        assert!(enabled_pane_in(&root.join("nope"), PaneSlot::Todos).is_none());
        std::fs::remove_dir_all(&root).unwrap();
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

    fn declared(extra: &str) -> Manifest {
        Manifest::parse(&format!(
            "id=graph-map\nversion=1\nkind=screen\nentry=g.wasm\n{extra}"
        ))
        .unwrap()
    }

    #[test]
    fn settings_are_declared_in_plugin_conf() {
        let m =
            declared("setting=view|Start on|Map,List|Map\nsetting=labels|Labels|Auto, All|All\n");
        assert_eq!(m.settings.len(), 2);
        assert_eq!(m.settings[0].key, "view");
        assert_eq!(m.settings[0].label, "Start on");
        assert_eq!(m.settings[0].options, ["Map", "List"]);
        assert_eq!(m.settings[0].default, 0);
        assert_eq!(
            m.settings[1].options,
            ["Auto", "All"],
            "options are trimmed"
        );
        assert_eq!(m.settings[1].default, 1, "the default names an option");
        assert!(declared("").settings.is_empty());
    }

    #[test]
    fn bad_setting_lines_are_skipped_not_fatal() {
        let m = declared(
            "setting=Bad Key|L|a,b|a\n\
             setting=one|L|only|only\n\
             setting=five|L|a,b,c,d,e|a\n\
             setting=empty|L|a,|a\n\
             setting=nolabel||a,b|a\n\
             setting=extra|L|a,b|a|x\n\
             setting=ok|Fine|a,b|zzz\n\
             setting=ok|Again|c,d|c\n",
        );
        assert_eq!(m.settings.len(), 1, "{:?}", m.settings);
        assert_eq!(
            m.settings[0].label, "Fine",
            "the first of a repeated key wins"
        );
        assert_eq!(
            m.settings[0].default, 0,
            "an unknown default is the first option"
        );
        // At most eight.
        let many: String = (0..12).map(|i| format!("setting=k{i}|L|a,b|a\n")).collect();
        assert_eq!(declared(&many).settings.len(), MAX_PLUGIN_SETTINGS);
    }

    #[test]
    fn setting_values_default_save_and_ignore_junk() {
        let root = std::env::temp_dir().join(format!("sigma-setvals-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let m =
            declared("setting=view|Start on|Map,List|Map\nsetting=labels|Labels|Auto,All|Auto\n");
        // Nothing saved: the defaults.
        assert_eq!(settings_in(&root, &m), [0, 0]);
        std::fs::create_dir_all(root.join("graph-map")).unwrap();
        set_setting_in(&root, &m, "view", 1).unwrap();
        assert_eq!(settings_in(&root, &m), [1, 0]);
        set_setting_in(&root, &m, "labels", 1).unwrap();
        set_setting_in(&root, &m, "view", 0).unwrap();
        assert_eq!(settings_in(&root, &m), [0, 1], "the other value is kept");
        let text = std::fs::read_to_string(root.join("graph-map/settings")).unwrap();
        assert_eq!(text.lines().count(), 2, "one line per key: {text:?}");
        // A value that is not an option, and a key that is not declared, are
        // ignored when reading.
        std::fs::write(
            root.join("graph-map/settings"),
            "view=Sideways\nlabels=All\nold=1\n",
        )
        .unwrap();
        assert_eq!(settings_in(&root, &m), [0, 1]);
        // Writing refuses what was not declared or is out of range.
        assert!(set_setting_in(&root, &m, "nope", 0).is_err());
        assert!(set_setting_in(&root, &m, "view", 2).is_err());
        assert_eq!(
            settings_in(&root, &m),
            [0, 1],
            "failed writes change nothing"
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
