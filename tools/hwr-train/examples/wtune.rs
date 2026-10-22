//! Tunes the word-mode letter bonus on the training fonts.
use hwr_train::*;
use sigma_hwr::*;
fn main() {
    let r = Recogniser::new(Model::default_model());
    let w = word::builtin_words();
    for b in [3.0f32, 8.0, 12.0, 20.0] {
        println!(
            "bonus {b}: {:?}",
            word_score_with(&r, TRAIN_FONTS, &w, 15, 77, b)
        );
    }
}
