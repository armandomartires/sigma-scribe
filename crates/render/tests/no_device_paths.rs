//! P2-03: `sigma-render` must not contain Kobo or device absolute paths.

use std::path::Path;

fn patterns() -> [String; 4] {
    [
        format!("/{}/{}", "mnt", "onboard"),
        format!("/{}/{}", "sys", "class"),
        format!("/{}/{}", "dev", "input"),
        "wpa_supplicant".to_string(),
    ]
}

fn walk(dir: &Path, hits: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            walk(&p, hits);
            continue;
        }
        if p.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            for pat in patterns() {
                if line.contains(&pat) {
                    hits.push(format!("{}:{}: {line}", p.display(), i + 1));
                }
            }
        }
    }
}

#[test]
fn src_has_no_device_path_literals() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut hits = Vec::new();
    walk(&src, &mut hits);
    assert!(
        hits.is_empty(),
        "device path literals in sigma-render:\n{}",
        hits.join("\n")
    );
}
