//! Plugin packages: the file the repository index points at, and how it is
//! checked and installed.
//!
//! A package is an uncompressed ustar archive of regular files, with
//! `plugin.conf` at its root. The index states its size and sha256; nothing
//! is unpacked before both match. Installing is atomic per plugin: the files
//! are written to a scratch folder beside the others and renamed into place,
//! so a failure leaves the old version (or nothing) as it was. No network
//! here; the front ends fetch the bytes.

use crate::plugins::{self, safe_rel_path, valid_id, IndexEntry, Manifest, State, Version};
use std::collections::BTreeMap;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;

/// Largest package the app will unpack.
pub const MAX_PACKAGE: u64 = 32 * 1024 * 1024;

/// Most files a package may hold.
pub const MAX_FILES: usize = 256;

const K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// The lowercase hex sha256 of `data`.
#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in msg.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, c) in block.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

fn bad(msg: &str) -> Error {
    Error::new(ErrorKind::InvalidData, msg.to_string())
}

fn cstr(field: &[u8]) -> &[u8] {
    let n = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    &field[..n]
}

/// The files of a ustar archive, by relative path. Only regular files and
/// directories are accepted; a leading `./` is dropped.
///
/// # Errors
/// `InvalidData` for anything else: links, devices, extended headers, unsafe
/// or repeated paths, a truncated archive, too many files.
pub fn read_tar(data: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    let mut at = 0;
    loop {
        let Some(head) = data.get(at..at + 512) else {
            return Err(bad("truncated archive"));
        };
        if head.iter().all(|&b| b == 0) {
            return Ok(files);
        }
        if &head[257..262] != b"ustar" {
            return Err(bad("not a ustar archive"));
        }
        let size = std::str::from_utf8(cstr(&head[124..136]))
            .ok()
            .and_then(|s| usize::from_str_radix(s.trim(), 8).ok())
            .ok_or_else(|| bad("bad size"))?;
        let name = std::str::from_utf8(cstr(&head[0..100])).map_err(|_| bad("bad name"))?;
        let prefix = std::str::from_utf8(cstr(&head[345..500])).map_err(|_| bad("bad name"))?;
        let full = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}/{name}")
        };
        let full = full
            .strip_prefix("./")
            .unwrap_or(&full)
            .trim_end_matches('/');
        at += 512;
        match head[156] {
            b'0' | 0 => {
                if !safe_rel_path(full) {
                    return Err(bad("unsafe path"));
                }
                let body = data
                    .get(at..at + size)
                    .ok_or_else(|| bad("truncated file"))?;
                if files.insert(full.to_string(), body.to_vec()).is_some() {
                    return Err(bad("repeated path"));
                }
                if files.len() > MAX_FILES {
                    return Err(bad("too many files"));
                }
                at += size.div_ceil(512) * 512;
            }
            b'5' => {
                if !full.is_empty() && full != "." && !safe_rel_path(full) {
                    return Err(bad("unsafe path"));
                }
            }
            _ => return Err(bad("unsupported entry")),
        }
    }
}

/// Checks `bytes` against the index `entry` and unpacks it in memory: size
/// and sha256 must match, `plugin.conf` must exist and agree with the index
/// on id and version, and every path must be safe.
///
/// # Errors
/// `InvalidData` naming what failed.
pub fn verify(entry: &IndexEntry, bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    if bytes.len() as u64 != entry.size || entry.size > MAX_PACKAGE {
        return Err(bad("size does not match the index"));
    }
    if sha256_hex(bytes) != entry.sha256 {
        return Err(bad("checksum does not match the index"));
    }
    let files = read_tar(bytes)?;
    let conf = files
        .get("plugin.conf")
        .and_then(|b| std::str::from_utf8(b).ok())
        .and_then(Manifest::parse)
        .ok_or_else(|| bad("no valid plugin.conf"))?;
    if conf.id != entry.manifest.id || conf.version != entry.manifest.version {
        return Err(bad("plugin.conf does not match the index"));
    }
    Ok(files)
}

/// Installs the package `bytes` for `entry` under `root` (the plugins
/// folder) and records it in `plugins.state`. A new plugin starts on; an
/// update keeps its on/off choice and its settings file. Returns the version
/// installed.
///
/// # Errors
/// `InvalidData` when [`verify`] fails or the plugin needs a newer app than
/// `app`; `Unsupported` when it would replace a newer installed version;
/// any filesystem error, after which the old version is as it was.
pub fn install_in(root: &Path, entry: &IndexEntry, bytes: &[u8], app: Version) -> Result<Version> {
    let id = entry.manifest.id.as_str();
    if !valid_id(id) {
        return Err(bad("bad plugin id"));
    }
    if !entry.manifest.compatible_with(app) {
        return Err(bad("needs a newer app"));
    }
    let files = verify(entry, bytes)?;
    let state_path = root.join("plugins.state");
    let mut state = std::fs::read_to_string(&state_path)
        .map(|t| State::parse(&t))
        .unwrap_or_default();
    if state
        .get(id)
        .is_some_and(|p| p.version > entry.manifest.version)
    {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "a newer version is installed",
        ));
    }
    let fresh = root.join(format!(".install-{id}"));
    let old = root.join(format!(".old-{id}"));
    let dest = root.join(id);
    let _ = std::fs::remove_dir_all(&fresh);
    let _ = std::fs::remove_dir_all(&old);
    let written = (|| {
        for (rel, body) in &files {
            let p = fresh.join(rel);
            if let Some(dir) = p.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&p, body)?;
        }
        if let Ok(s) = std::fs::read(dest.join("settings")) {
            std::fs::write(fresh.join("settings"), s)?;
        }
        Ok::<(), Error>(())
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_dir_all(&fresh);
        return Err(e);
    }
    let had = dest.exists();
    if had {
        std::fs::rename(&dest, &old)?;
    }
    if let Err(e) = std::fs::rename(&fresh, &dest) {
        if had {
            let _ = std::fs::rename(&old, &dest);
        }
        let _ = std::fs::remove_dir_all(&fresh);
        return Err(e);
    }
    let _ = std::fs::remove_dir_all(&old);
    state.install(id, entry.manifest.version);
    crate::store::write_atomic(&state_path, &state.serialize())?;
    Ok(entry.manifest.version)
}

/// Removes plugin `id` from `root` and from `plugins.state`.
///
/// # Errors
/// `NotFound` when it is not installed, or any filesystem error.
pub fn uninstall_in(root: &Path, id: &str) -> Result<()> {
    if !valid_id(id) || plugins::manifest_in(root, id).is_none() {
        return Err(Error::new(ErrorKind::NotFound, "no such plugin"));
    }
    let state_path = root.join("plugins.state");
    let mut state = std::fs::read_to_string(&state_path)
        .map(|t| State::parse(&t))
        .unwrap_or_default();
    state.remove(id);
    crate::store::write_atomic(&state_path, &state.serialize())?;
    std::fs::remove_dir_all(root.join(id))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn entry(bytes: &[u8], version: &str, min_app: &str) -> IndexEntry {
        IndexEntry {
            manifest: Manifest::parse(&format!(
                "id=demo\nname=Demo\nversion={version}\nkind=font\nmin_app={min_app}\n"
            ))
            .unwrap(),
            url: "https://armandomartires.github.io/sigma-scribe-plugins/demo.tar".into(),
            size: bytes.len() as u64,
            sha256: sha256_hex(bytes),
        }
    }

    fn pkg(version: &str) -> Vec<u8> {
        let conf = format!("id=demo\nname=Demo\nversion={version}\nkind=font\n");
        tar(&[
            ("plugin.conf", conf.as_bytes()),
            ("fonts/demo.ttf", b"font"),
        ])
    }

    fn root(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("sigma-pkg-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    const APP: Version = Version(1, 15, 0);

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(&[b'a'; 1000]),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    #[test]
    fn installs_and_records_state() {
        let r = root("install");
        let b = pkg("1.0.0");
        let e = entry(&b, "1.0.0", "1.0.0");
        assert_eq!(install_in(&r, &e, &b, APP).unwrap(), Version(1, 0, 0));
        assert_eq!(
            std::fs::read(r.join("demo/fonts/demo.ttf")).unwrap(),
            b"font"
        );
        let l = plugins::list_in(&r);
        assert_eq!(l.len(), 1);
        assert!(l[0].enabled);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn update_keeps_off_switch_and_settings() {
        let r = root("update");
        let b = pkg("1.0.0");
        install_in(&r, &entry(&b, "1.0.0", "1.0.0"), &b, APP).unwrap();
        plugins::set_enabled_in(&r, "demo", false).unwrap();
        std::fs::write(r.join("demo/settings"), "view=List\n").unwrap();
        let b2 = pkg("1.1.0");
        install_in(&r, &entry(&b2, "1.1.0", "1.0.0"), &b2, APP).unwrap();
        assert_eq!(
            std::fs::read_to_string(r.join("demo/settings")).unwrap(),
            "view=List\n"
        );
        let l = plugins::list_in(&r);
        assert_eq!(l[0].manifest.version, Version(1, 1, 0));
        assert!(!l[0].enabled);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn refuses_tampered_wrong_size_downgrade_and_new_app() {
        let r = root("refuse");
        let b = pkg("1.1.0");
        let mut e = entry(&b, "1.1.0", "1.0.0");
        let mut t = b.clone();
        t[600] ^= 1;
        assert!(install_in(&r, &e, &t, APP).is_err());
        e.size += 1;
        assert!(install_in(&r, &e, &b, APP).is_err());
        let e = entry(&b, "1.1.0", "9.0.0");
        assert!(install_in(&r, &e, &b, APP).is_err());
        let e = entry(&b, "1.1.0", "1.0.0");
        install_in(&r, &e, &b, APP).unwrap();
        let old = pkg("1.0.0");
        let err = install_in(&r, &entry(&old, "1.0.0", "1.0.0"), &old, APP).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Unsupported);
        assert_eq!(plugins::list_in(&r)[0].manifest.version, Version(1, 1, 0));
        assert!(!r.join(".install-demo").exists());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn rejects_unsafe_archives_and_mismatched_manifest() {
        let conf: &[u8] = b"id=demo\nname=Demo\nversion=1.0.0\nkind=font\n";
        for files in [
            vec![("plugin.conf", conf), ("../evil", b"x" as &[u8])],
            vec![("plugin.conf", conf), ("/abs", b"x")],
            vec![("plugin.conf", conf), ("a", b"1"), ("a", b"2")],
            vec![("fonts/only.ttf", b"x")],
        ] {
            let b = tar(&files);
            assert!(verify(&entry(&b, "1.0.0", "1.0.0"), &b).is_err());
        }
        let b = pkg("1.0.0");
        assert!(verify(&entry(&b, "2.0.0", "1.0.0"), &b).is_err());
        let mut link = tar(&[("plugin.conf", conf)]);
        link[156] = b'2';
        assert!(read_tar(&link).is_err());
    }

    #[test]
    fn uninstall_removes_files_and_state() {
        let r = root("uninstall");
        let b = pkg("1.0.0");
        install_in(&r, &entry(&b, "1.0.0", "1.0.0"), &b, APP).unwrap();
        uninstall_in(&r, "demo").unwrap();
        assert!(!r.join("demo").exists());
        assert!(plugins::list_in(&r).is_empty());
        assert!(!std::fs::read_to_string(r.join("plugins.state"))
            .unwrap()
            .contains("demo"));
        assert_eq!(
            uninstall_in(&r, "demo").unwrap_err().kind(),
            ErrorKind::NotFound
        );
        let _ = std::fs::remove_dir_all(&r);
    }
}
