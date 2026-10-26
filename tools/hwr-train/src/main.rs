//! `cargo run -p hwr-train --release -- train` rebuilds
//! `assets/hwr/default.hwr` (letters); `-- train-pieces` rebuilds
//! `assets/hwr/pieces.hwr` (the word decoder's model); `-- widths` rebuilds
//! `assets/hwr/widths.txt`; `-- eval`, `-- words` and `-- free` print the
//! held-out figures. `HWR_MODEL` / `HWR_PIECES_MODEL` point at other model
//! files (experiments).

use hwr_train::{score, synth_set, Net, Rng, HELD_OUT_FONTS, TRAIN_FONTS};
use sigma_hwr::{Model, Recogniser, CHARSET};
use std::time::Instant;

const HIDDEN: usize = 192;
const PER: usize = 60;
const EPOCHS: usize = 12;
/// Hidden units of the piece model: more than the letter model, which
/// keeps its letter accuracy while it also learns "not a letter".
const PIECES_HIDDEN: usize = 256;
/// Random letter strings per training font cut into pieces (see
/// `piece_set`).
const PIECES_PER: usize = 400;
/// "Not a letter" pieces kept per training font.
const MAX_NONE: usize = 3000;

fn path(var: &str, file: &str) -> String {
    std::env::var(var)
        .unwrap_or_else(|_| format!("{}/../../assets/hwr/{file}", env!("CARGO_MANIFEST_DIR")))
}

/// Trains a network on the isolated letters of the training fonts, and
/// with `pieces` also on pieces of synthetic words with a "not a letter"
/// class, and writes it to `out`.
fn train(out: &str, hidden: usize, pieces: bool) {
    let t = Instant::now();
    let mut set = synth_set(TRAIN_FONTS, PER, 1);
    let letters = CHARSET.chars().count();
    let classes = if pieces {
        let p = hwr_train::piece_set(TRAIN_FONTS, PIECES_PER, MAX_NONE, 2, letters);
        let none = p.iter().filter(|(_, c)| *c == letters).count();
        eprintln!("pieces: {} letters, {none} not a letter", p.len() - none);
        set.extend(p);
        letters + 1
    } else {
        letters
    };
    eprintln!("{} training samples in {:?}", set.len(), t.elapsed());
    let mut rng = Rng::new(7);
    let mut net = Net::new(hidden, classes, &mut rng);
    net.train(&set, EPOCHS, &mut rng);
    let bytes = net.model().to_bytes();
    std::fs::write(out, &bytes).expect("write model");
    eprintln!("wrote {} bytes in {:?}", bytes.len(), t.elapsed());
}

fn load(path: &str) -> Model {
    Model::from_bytes(&std::fs::read(path).expect("model")).expect("parse")
}

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    let out = path("HWR_MODEL", "default.hwr");
    let out = out.as_str();
    let pieces = path("HWR_PIECES_MODEL", "pieces.hwr");
    match cmd.as_str() {
        "train" => train(out, HIDDEN, false),
        "train-pieces" => train(&pieces, PIECES_HIDDEN, true),
        "widths" => {
            let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/hwr/widths.txt");
            let mut text = String::from(
                "# Letter widths for the word decoder: character, mean of ln(ink width in\n\
                 # line units), spread, over the training fonts. Made by `hwr-train widths`.\n",
            );
            for (c, m, sd) in hwr_train::letter_widths() {
                text.push_str(&format!("{c}\t{m:.3}\t{sd:.3}\n"));
            }
            std::fs::write(path, text).expect("write widths");
        }
        "eval" => {
            let r = Recogniser::new(load(out));
            let seen = synth_set(TRAIN_FONTS, 10, 1001);
            let unseen = synth_set(HELD_OUT_FONTS, 20, 2002);
            println!("held-out jitter, training fonts: {:?}", score(&r, &seen));
            println!(
                "held-out fonts {HELD_OUT_FONTS:?}: {:?}",
                score(&r, &unseen)
            );
            for font in HELD_OUT_FONTS {
                let test = synth_set(&[font], 20, 3003);
                let mut ru = r.clone();
                let before = score(&ru, &test);
                ru.set_samples(&hwr_train::enrol_samples(font, 1, 4004));
                let after = score(&ru, &test);
                println!("{font}: before {before:?}\n  after 1 sample/char {after:?}");
            }
            let raw = hwr_train::enrol_samples(HELD_OUT_FONTS[0], 1, 5005);
            let t = Instant::now();
            for s in &raw {
                let _ = r.recognise(&s.strokes, 5);
            }
            println!(
                "end to end (features + net): {:.1} us/char",
                t.elapsed().as_secs_f64() * 1e6 / raw.len() as f64
            );
            let t = Instant::now();
            for (f, _) in &unseen {
                let _ = r.rank(f, 5);
            }
            println!(
                "classify: {:.1} us/char",
                t.elapsed().as_secs_f64() * 1e6 / unseen.len() as f64
            );
        }
        "words" => {
            let r = Recogniser::new(load(out)).with_pieces(load(&pieces));
            let words = sigma_hwr::word::builtin_words();
            let seen = hwr_train::word_score(&r, TRAIN_FONTS, &words, 30, 11);
            println!("words, training fonts (new jitter): {seen:?}");
            for font in HELD_OUT_FONTS {
                let plain = hwr_train::word_score(&r, &[font], &words, 150, 12);
                let mut ru = r.clone();
                ru.set_samples(&hwr_train::enrol_samples(font, 1, 4004));
                let enr = hwr_train::word_score(&ru, &[font], &words, 150, 12);
                println!("words, {font}: no enrolment {plain:?}\n  1 sample/char {enr:?}");
            }
        }
        "free" => {
            let r = Recogniser::new(load(out)).with_pieces(load(&pieces));
            let printed = (0.06, 0.16);
            let touching = (-0.02, 0.05);
            println!(
                "free, training fonts, printed: {:?}",
                hwr_train::free_score(&r, TRAIN_FONTS, 30, printed, 21)
            );
            for font in HELD_OUT_FONTS {
                let plain = hwr_train::free_score(&r, &[font], 150, printed, 22);
                let mut ru = r.clone();
                ru.set_samples(&hwr_train::enrol_samples(font, 1, 4004));
                let enr = hwr_train::free_score(&ru, &[font], 150, printed, 22);
                let tch = hwr_train::free_score(&ru, &[font], 150, touching, 23);
                println!("free, {font}, printed: no enrolment {plain:?}\n  1 sample/char {enr:?}\n  touching letters, enrolled {tch:?}");
            }
        }
        _ => eprintln!("usage: hwr-train train|eval|words|free"),
    }
}
