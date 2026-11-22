//! Fuzz [`sigma_plugin_api::Event::decode`] (P3-02).
#![no_main]

use libfuzzer_sys::fuzz_target;
use sigma_plugin_api::Event;

fuzz_target!(|data: &[u8]| {
    let _ = Event::decode(data);
});
