//! Settings → Plugins → Repository: fetch the plugins index, install and
//! update plugins from it.
//!
//! The index and every package are fetched in the app (ureq with rustls and
//! the bundled webpki roots: the Kobo has no `curl`), only from
//! [`plugins::trusted_source`] URLs and over https, redirects included. `SIGMA_PLUGIN_REPO=<folder>` reads `index.json` and the
//! packages (by file name) from a folder instead: an offline mirror, and
//! what the tests use. Every package is checked against the size and sha256
//! in the index before anything is unpacked ([`sigma_core::pluginpkg`]).
//! A fetch blocks the screen until it ends.

use crate::App;
use sigma_core::pluginpkg;
use sigma_core::plugins::{self, Availability, Index};
use sigma_device::Refresh;
use sigma_render::options::{RepoRow, RepoState};
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use std::time::Duration;

/// Largest index the app reads.
const MAX_INDEX: u64 = 256 * 1024;

fn mirror() -> Option<PathBuf> {
    std::env::var_os("SIGMA_PLUGIN_REPO").map(PathBuf::from)
}

/// The bytes at `url`, at most `max` of them.
fn fetch(url: &str, max: u64) -> Result<Vec<u8>> {
    if !plugins::trusted_source(url) && url != plugins::INDEX_URL {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            "not the plugins repository",
        ));
    }
    let body = if let Some(dir) = mirror() {
        let name = url.rsplit('/').next().unwrap_or_default();
        if name.is_empty() || name.starts_with('.') {
            return Err(Error::new(ErrorKind::InvalidInput, "bad file name"));
        }
        std::fs::read(dir.join(name))?
    } else {
        download(url, max)?
    };
    if body.len() as u64 > max {
        return Err(Error::new(ErrorKind::InvalidData, "download too large"));
    }
    Ok(body)
}

/// `url` over https, following redirects only to https, at most `max`
/// bytes, two minutes at most.
fn download(url: &str, max: u64) -> Result<Vec<u8>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .into();
    let failed = |e: ureq::Error| match e {
        ureq::Error::StatusCode(c) => {
            Error::new(ErrorKind::NotFound, format!("download failed (HTTP {c})"))
        }
        ureq::Error::Io(e) => Error::new(ErrorKind::NotConnected, format!("no network: {e}")),
        ureq::Error::HostNotFound => {
            Error::new(ErrorKind::NotConnected, "no network (is Wi-Fi on?)")
        }
        e => Error::other(format!("download failed: {e}")),
    };
    let mut response = agent.get(url).call().map_err(failed)?;
    response
        .body_mut()
        .with_config()
        // One byte over: ureq refuses a body that reaches its limit, and
        // `fetch` refuses one over `max`.
        .limit(max + 1)
        .read_to_vec()
        .map_err(failed)
}

fn app_version() -> plugins::Version {
    plugins::Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_default()
}

impl App {
    fn repo_note(&mut self, text: impl Into<String>) {
        self.repo_note = text.into();
    }

    /// Rebuilds the Repository rows from the index and what is installed.
    fn rebuild_repo_rows(&mut self) {
        let state = std::fs::read_to_string(self.plugins_root.join("plugins.state"))
            .map(|t| plugins::State::parse(&t))
            .unwrap_or_default();
        let app = app_version();
        self.repo_rows = self
            .repo_index
            .iter()
            .flat_map(|ix| &ix.entries)
            .map(|e| RepoRow {
                name: e.manifest.name.clone(),
                note: format!(
                    "{} \u{b7} {} \u{b7} {} KB",
                    e.manifest.kind.label(),
                    e.manifest.version,
                    e.size.div_ceil(1024)
                ),
                state: match plugins::availability(e, &state, app) {
                    Availability::NotInstalled => RepoState::Install,
                    Availability::UpdateAvailable => RepoState::Update,
                    Availability::UpToDate => RepoState::Current,
                    Availability::Incompatible => RepoState::Incompatible,
                },
            })
            .collect();
    }

    /// Fetches the index. It must not be older than the last one seen.
    pub(crate) fn plugin_check(&mut self) -> Refresh {
        let serial_path = self.plugins_root.join("index.serial");
        let seen: u64 = std::fs::read_to_string(&serial_path)
            .ok()
            .and_then(|t| t.trim().parse().ok())
            .unwrap_or(0);
        let result = fetch(plugins::INDEX_URL, MAX_INDEX).and_then(|b| {
            let ix = Index::parse(&String::from_utf8_lossy(&b));
            if ix.serial < seen {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "the index is older than the last one",
                ));
            }
            Ok(ix)
        });
        match result {
            Ok(ix) => {
                let _ = sigma_core::store::write_atomic(&serial_path, &format!("{}\n", ix.serial));
                sigma_core::info!(
                    "plugin",
                    "repository: {} plugins, serial {}",
                    ix.entries.len(),
                    ix.serial
                );
                let n = ix.entries.len();
                self.repo_index = Some(ix);
                self.repo_note(format!("{n} available"));
            }
            Err(e) => {
                sigma_core::warn!("plugin", "repository: {e}");
                self.repo_note(format!("Could not check: {e}"));
            }
        }
        self.rebuild_repo_rows();
        Refresh::Full
    }

    /// Installs or updates the repository's plugin number `i`.
    pub(crate) fn plugin_get(&mut self, i: usize) -> Refresh {
        let Some(entry) = self
            .repo_index
            .as_ref()
            .and_then(|ix| ix.entries.get(i))
            .cloned()
        else {
            return Refresh::Partial;
        };
        let id = entry.manifest.id.clone();
        let result = fetch(&entry.url, entry.size.min(pluginpkg::MAX_PACKAGE))
            .and_then(|b| pluginpkg::install_in(&self.plugins_root, &entry, &b, app_version()));
        match result {
            Ok(v) => {
                sigma_core::info!("plugin", "{id}: installed {v}");
                self.repo_note(format!("{} {v} installed", entry.manifest.name));
                self.plugin_failed = false;
                self.pane_failed = [false; 3];
            }
            Err(e) => {
                sigma_core::warn!("plugin", "{id}: install failed: {e}");
                self.repo_note(format!("{} not installed: {e}", entry.manifest.name));
            }
        }
        self.refresh_plugin_list();
        self.sync_plugin_icons();
        self.rebuild_repo_rows();
        Refresh::Full
    }

    /// Removes installed plugin number `i` of the list the page showed. A
    /// plugin that is on is left alone.
    pub(crate) fn plugin_remove(&mut self, i: usize) -> Refresh {
        let Some(row) = self.plugin_list.get(i).filter(|p| !p.enabled) else {
            return Refresh::Partial;
        };
        let (id, name) = (row.id.clone(), row.name.clone());
        match pluginpkg::uninstall_in(&self.plugins_root, &id) {
            Ok(()) => {
                sigma_core::info!("plugin", "{id}: removed");
                self.repo_note(format!("{name} removed"));
                self.plugin_idle = None;
            }
            Err(e) => {
                sigma_core::warn!("plugin", "{id}: remove failed: {e}");
                self.repo_note(format!("{name} not removed: {e}"));
            }
        }
        self.refresh_plugin_list();
        self.sync_plugin_icons();
        self.rebuild_repo_rows();
        Refresh::Full
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fetches the live index and its first package over the network:
    /// `cargo test -p sigma-scribe --lib live_repository -- --ignored`.
    #[test]
    #[ignore = "needs the network"]
    fn live_repository() {
        let ix = Index::parse(&String::from_utf8_lossy(
            &fetch(plugins::INDEX_URL, MAX_INDEX).unwrap(),
        ));
        let e = ix.entries.first().expect("an entry");
        let bytes = fetch(&e.url, e.size).unwrap();
        pluginpkg::verify(e, &bytes).unwrap();
        let err = download("http://armandomartires.github.io/", 1024).unwrap_err();
        assert!(err.to_string().contains("download failed"), "{err}");
    }
}
