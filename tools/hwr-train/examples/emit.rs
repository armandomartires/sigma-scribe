//! Prints a synthetic word as an emulator `ink-line` argument and what
//! word mode reads: `cargo run -p hwr-train --release --example emit -- <font> <word> <seed>`.
use hwr_train::*;
use sigma_hwr::{word::*, *};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = load_font(&a[1]);
    let mut rng = Rng::new(a[3].parse().unwrap());
    let segs = synth_word(&f, &a[2], &mut rng).unwrap();
    let x0 = segs
        .iter()
        .map(|s| s.0 .0.min(s.1 .0))
        .fold(f32::MAX, f32::min)
        - 0.1;
    let r = Recogniser::new(Model::default_model());
    let segs: Vec<Seg> = segs
        .iter()
        .map(|&(p, q)| ((p.0 - x0, p.1), (q.0 - x0, q.1)))
        .collect();
    let top = recognise(&r, &segs, &Lexicon::new(&r, &builtin_words()), 5);
    eprintln!("{top:?}");
    let s: Vec<String> = segs
        .iter()
        .map(|(p, q)| format!("{:.3},{:.3} {:.3},{:.3}", p.0, p.1, q.0, q.1))
        .collect();
    println!("{}", s.join(";"));
}
