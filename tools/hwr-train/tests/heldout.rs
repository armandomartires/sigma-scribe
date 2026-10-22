//! v1.5.2 held-out check (runs in CI): the bundled model on synthetic
//! ink it never saw (new jitter seeds of the training fonts, and two
//! fonts never trained on), plus enrolment and latency. Prints the
//! figures; fails if they drop below the floors.

use hwr_train::{enrol_samples, score, synth_set, HELD_OUT_FONTS, TRAIN_FONTS};
use sigma_hwr::{Model, Recogniser};
use std::time::Instant;

#[test]
fn held_out_accuracy_latency_and_enrolment() {
    let r = Recogniser::new(Model::default_model());
    let seen = score(&r, &synth_set(TRAIN_FONTS, 3, 1001));
    let unseen_set = synth_set(HELD_OUT_FONTS, 6, 2002);
    let unseen = score(&r, &unseen_set);
    println!("held-out jitter, training fonts: {seen:?}");
    println!("held-out fonts: {unseen:?}");
    assert!(seen.top1 >= 0.95 && seen.top5 >= 0.99, "{seen:?}");
    assert!(unseen.top1 >= 0.78 && unseen.top5 >= 0.97, "{unseen:?}");
    assert!(unseen.fold1 >= 0.88, "{unseen:?}");

    let font = HELD_OUT_FONTS[0];
    let test = synth_set(&[font], 6, 3003);
    let before = score(&r, &test);
    let mut ru = r.clone();
    let t = Instant::now();
    ru.set_samples(&enrol_samples(font, 1, 4004));
    let tune = t.elapsed();
    let after = score(&ru, &test);
    println!("{font}: before {before:?} after 1 sample/char {after:?}, fine-tune {tune:?}");
    assert!(after.top1 > before.top1, "enrolment did not help");

    let raw = enrol_samples(font, 1, 5005);
    let t = Instant::now();
    for s in &raw {
        let _ = r.recognise(&s.strokes, 5);
    }
    let us = t.elapsed().as_secs_f64() * 1e6 / raw.len() as f64;
    println!("latency: {us:.0} us/char (features + net + ranking)");
    assert!(us < 20_000.0, "{us} us/char");
}
