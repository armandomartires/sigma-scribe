//! Fuzz [`sigma_plugin_api::Frame::decode`] (P3-02).
#![no_main]

use libfuzzer_sys::fuzz_target;
use sigma_plugin_api::Frame;

fuzz_target!(|data: &[u8]| {
    let _ = Frame::decode(data);
});
