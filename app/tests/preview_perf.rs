//! How long entering preview takes on a large note (parse and layout on the
//! UI thread). Prints the times:
//!
//!     cargo test --release -p sigma-scribe --test preview_perf -- --ignored --nocapture
//!
//! Ignored by default: timings depend on the machine. On the Forma the same
//! number is logged at Debug level (Settings -> Logging) as
//! `preview layout of <note> (<bytes> bytes): <n> ms`; expect roughly 8 to
//! 10 times the desktop figure (docs/perf.md).

use sigma_core::NotePath;
use sigma_scribe::App;
use std::time::Instant;

/// A note of about `kb` kilobytes: headings, lists, quotes, code, emphasis
/// and wiki links, repeated.
fn note(kb: usize) -> String {
    let block = "## Section\n\nSome **strong** and *emphasised* text with a [[field-notes]] link, \
        `inline code` and ~~strike~~, long enough to wrap over a few lines of the page.\n\n\
        - first item\n  - nested item with more words\n- [ ] a task\n\n\
        > a quote that also wraps over more than one line of the page, to be sure\n\n\
        ```\nfn code() {}\n```\n\n";
    block.repeat(kb * 1024 / block.len() + 1)
}

#[test]
#[ignore = "prints timings; run with --ignored --nocapture"]
fn enter_preview_on_large_notes() {
    let d = std::env::temp_dir().join(format!("sigma-preview-perf-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for kb in [10, 50, 100, 250, 500] {
        let name = format!("big-{kb}.md");
        std::fs::write(d.join(&name), note(kb)).unwrap();
        let mut app = App::new(1440, 1920, d.clone());
        app.open(&NotePath::parse(&name).unwrap());
        let t = Instant::now();
        app.enter_preview();
        let ms = t.elapsed().as_secs_f64() * 1e3;
        let pages = app.preview.as_ref().map_or(0, |p| p.doc.pages.len());
        println!("{kb:>4} KB: preview in {ms:>8.1} ms, {pages} pages");
        assert!(app.preview.is_some());
    }
    let _ = std::fs::remove_dir_all(&d);
}
