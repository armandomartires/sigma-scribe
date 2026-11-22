//! Fuzz [`sigma_core::settings::Settings::parse`] (P3-02).
#![no_main]

use libfuzzer_sys::fuzz_target;
use sigma_core::settings::Settings;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = Settings::parse(&text);
});
