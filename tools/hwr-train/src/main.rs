//! `cargo run -p hwr-train --release -- train` rebuilds
//! `assets/hwr/default.hwr`; `-- widths` rebuilds `assets/hwr/widths.txt`;
//! `-- eval` prints the held-out figures.

use hwr_train::{score, synth_set, Net, Rng, HELD_OUT_FONTS, TRAIN_FONTS};
use sigma_hwr::{Model, Recogniser, CHARSET};
use std::time::Instant;

const HIDDEN: usize = 192;
const PER: usize = 60;
const EPOCHS: usize = 12;

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    let out = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/hwr/default.hwr");
    match cmd.as_str() {
        "train" => {
            let t = Instant::now();
            let set = synth_set(TRAIN_FONTS, PER, 1);
            eprintln!("{} training samples in {:?}", set.len(), t.elapsed());
            let mut rng = Rng::new(7);
            let mut net = Net::new(HIDDEN, CHARSET.chars().count(), &mut rng);
            net.train(&set, EPOCHS, &mut rng);
            let bytes = net.model().to_bytes();
            std::fs::write(out, &bytes).expect("write model");
            eprintln!("wrote {} bytes in {:?}", bytes.len(), t.elapsed());
        }
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
            let m = Model::from_bytes(&std::fs::read(out).expect("model")).expect("parse");
            let r = Recogniser::new(m);
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
            let m = Model::from_bytes(&std::fs::read(out).expect("model")).expect("parse");
            let r = Recogniser::new(m);
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
            let m = Model::from_bytes(&std::fs::read(out).expect("model")).expect("parse");
            let r = Recogniser::new(m);
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
