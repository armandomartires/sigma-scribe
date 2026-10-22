//! Tunes the user-sample blend: `cargo run -p hwr-train --release --example tune`.
use hwr_train::*;
use sigma_hwr::*;
use std::time::Instant;
fn main() {
    let m = Model::default_model();
    for font in HELD_OUT_FONTS {
        let test = synth_set(&[font], 10, 3003);
        for per in [1usize, 3] {
            let en = enrol_samples(font, per, 4004);
            for ep in [0usize, 10, 20, 40] {
                for w in [0.0f32, 1.5] {
                    let mut r = Recogniser::new(m.clone());
                    r.adapt_epochs = ep;
                    r.user_weight = w;
                    let t = Instant::now();
                    r.set_samples(&en);
                    println!(
                        "{font} per{per} ep{ep} w{w} {:?} {:?}",
                        score(&r, &test),
                        t.elapsed()
                    );
                }
            }
        }
    }
}
