//! v1.0 release prep: version, changelog, docs and package layout agree.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn read(p: &str) -> String {
    std::fs::read_to_string(root().join(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[test]
fn changelog_and_notes_cover_this_version() {
    assert!(read("CHANGELOG.md").contains(&format!("## {VERSION} ")));
    let notes = read(&format!("docs/release/v{VERSION}-notes.md"));
    assert!(notes.contains(VERSION) && notes.contains("DRAFT"));
    assert!(read("docs/release/mobileread-draft.md").contains("NOT posted"));
}

#[test]
fn readme_images_exist() {
    let readme = read("README.md");
    let mut n = 0;
    for part in readme.split("](").skip(1) {
        let path = part.split(')').next().unwrap();
        if path.ends_with(".png") {
            assert!(root().join(path).exists(), "missing {path}");
            n += 1;
        }
    }
    assert!(n >= 5);
}

#[test]
fn package_tree_is_adds_only_with_notices() {
    let adds = root().join("packaging/kobo/.adds");
    for f in [
        "nm/sigma-scribe",
        "sigma-scribe/launch.sh",
        "sigma-scribe/touch.conf",
        "sigma-scribe/THIRD-PARTY.txt",
    ] {
        assert!(adds.join(f).exists(), "{f}");
    }
    let tp = read("packaging/kobo/.adds/sigma-scribe/THIRD-PARTY.txt");
    assert!(tp.contains("Affero") && tp.contains("FBInk") && tp.contains("886f25f1"));
    // The FBInk pin is the same in CI and the release workflow.
    for wf in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        assert!(read(wf).contains("git checkout 886f25f1"), "{wf}");
    }
    let launch = read("packaging/kobo/.adds/sigma-scribe/launch.sh");
    assert!(!launch.contains("rm "), "the launcher never deletes");
}

#[test]
fn about_shows_this_version() {
    let d = std::env::temp_dir().join(format!("sigma-rel-{}", std::process::id()));
    let app = sigma_scribe::App::new(1440, 1920, d);
    assert!(app.about()[0].1.contains(&format!("Version {VERSION}")));
}
