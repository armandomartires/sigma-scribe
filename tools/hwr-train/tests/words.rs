//! v1.5.3 word-mode gate figures (runs in CI with `--nocapture`): CER,
//! word accuracy and time per word on synthetic words, the gate of
//! `sprints/S15.3-word-mode.md` (G1 CER ≤ 15 % on unseen fonts after
//! enrolment; G2 ≤ 180 ms p95 here ≈ 1 s on the A9). Word mode failed
//! G1 until the piece model (`assets/hwr/pieces.hwr`); the floors below
//! keep that gain, with room for the small samples (40–60 words).

use hwr_train::{enrol_samples, word_score, HELD_OUT_FONTS, TRAIN_FONTS};
use sigma_hwr::{word::builtin_words, Model, Recogniser};

#[test]
fn word_mode_gate_figures() {
    let r = Recogniser::new(Model::default_model()).with_pieces(Model::pieces_model());
    let words = builtin_words();
    let seen = word_score(&r, TRAIN_FONTS, &words, 6, 11);
    println!("words, training fonts (new jitter): {seen:?}");
    let mut worst = 0.0f32;
    let mut p95 = 0.0f64;
    for font in HELD_OUT_FONTS {
        let plain = word_score(&r, &[font], &words, 40, 12);
        let mut ru = r.clone();
        ru.set_samples(&enrol_samples(font, 1, 4004));
        let enr = word_score(&ru, &[font], &words, 40, 12);
        println!("words, {font}: no enrolment {plain:?}");
        println!("words, {font}: 1 sample/char {enr:?}");
        worst = worst.max(enr.cer);
        p95 = p95.max(enr.p95_us);
    }
    let g1 = worst <= 0.15;
    let g2 = p95 <= 180_000.0;
    println!(
        "GATE G1 (CER <= 15 %): {} (worst {:.1} %); G2 (p95 <= 180 ms here): {} ({:.1} ms)",
        if g1 { "pass" } else { "FAIL" },
        worst * 100.0,
        if g2 { "pass" } else { "FAIL" },
        p95 / 1000.0
    );
    assert!(seen.cer < 0.18, "{seen:?}");
    assert!(worst < 0.20, "worst held-out CER {worst}");
    assert!(g2, "p95 {p95} us");
}
