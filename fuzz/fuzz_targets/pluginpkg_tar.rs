//! Fuzz [`sigma_core::pluginpkg::read_tar`] (P3-02).
#![no_main]

use libfuzzer_sys::fuzz_target;
use sigma_core::pluginpkg::read_tar;

fuzz_target!(|data: &[u8]| {
    let _ = read_tar(data);
});
