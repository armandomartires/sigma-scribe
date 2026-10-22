//! Prints synthetic free writing as an emulator `ink-line` argument (area
//! heights) and what the free reader makes of it:
//! `cargo run -p hwr-train --release --example emitfree -- <font> "<words>" <seed>`.
use hwr_train::*;
use sigma_hwr::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = load_font(&a[1]);
    let ws: Vec<&str> = a[2].split(' ').collect();
    let mut rng = Rng::new(a[3].parse().unwrap());
    let segs = synth_phrase(&f, &ws, (0.08, 0.16), &mut rng).unwrap();
    let r = Recogniser::new(Model::default_model());
    let per: Vec<Vec<Seg>> = segs.iter().map(|&s| vec![s]).collect();
    eprintln!("{:?}", free::read(&r, &per, None));
    let x0 = segs
        .iter()
        .map(|s| s.0 .0.min(s.1 .0))
        .fold(f32::MAX, f32::min);
    let y0 = segs
        .iter()
        .map(|s| s.0 .1.min(s.1 .1))
        .fold(f32::MAX, f32::min);
    let y1 = segs
        .iter()
        .map(|s| s.0 .1.max(s.1 .1))
        .fold(f32::MIN, f32::max);
    let k = 0.4 / (y1 - y0);
    let m = |p: (f32, f32)| ((p.0 - x0) * k + 0.08, (p.1 - y0) * k + 0.25);
    let s: Vec<String> = segs
        .iter()
        .map(|&(p, q)| {
            let (p, q) = (m(p), m(q));
            format!("{:.3},{:.3} {:.3},{:.3}", p.0, p.1, q.0, q.1)
        })
        .collect();
    println!("{}", s.join(";"));
}
