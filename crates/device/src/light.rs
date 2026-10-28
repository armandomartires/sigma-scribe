//! The front light (v1.12, Display → Backlight), through sysfs.
//!
//! The Kobo exposes its light as one or more `/sys/class/backlight/*`
//! devices, each with `brightness` and `max_brightness`. Which of them is
//! the white light differs by model, so [`Light::find`] takes the first
//! device from a short preference list (on the Forma it ends up with
//! `mxc_msp430.0`, confirmed on the device),
//! else any device that has both files, and logs every device it sees.
//! `SIGMA_LIGHT_DEVICE=<name>` picks one by hand. The light keeps its
//! level when the app leaves, as it does for the stock reader.
//!
//! The warm-to-cool mixer ([`Warmth`], v1.17) is a separate file, `color`
//! in `tlc5947_bl` on the Forma. KOReader writes it as `10 - warmth` and
//! never reads it back; see [`Warmth`].

use std::io;
use std::path::{Path, PathBuf};

/// The sysfs directory of the backlight class.
pub const ROOT: &str = "/sys/class/backlight";

/// Preferred devices, best first (KOReader's names for the Mark 7 and
/// older Kobos).
const PREFERRED: [&str; 5] = [
    "lm3630a_ledb",
    "mxc_msp430.0",
    "tlc5947_bl",
    "lm3630a_led1b",
    "lm3630a_led",
];

/// Steps above off (nine stops, 0 to 8).
pub const MAX_STEP: u8 = 8;

/// One sysfs backlight device, driven in steps of 0 to [`MAX_STEP`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Light {
    dir: PathBuf,
    /// The device's own maximum (`max_brightness`).
    max: u32,
}

/// One line per device for the log: name, current, maximum.
#[must_use]
pub fn describe(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(root) else {
        return out;
    };
    let mut names: Vec<_> = rd.filter_map(Result::ok).map(|e| e.path()).collect();
    names.sort();
    for p in names {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned());
        let now = read_u32(&p.join("brightness"));
        let max = read_u32(&p.join("max_brightness"));
        out.push(format!(
            "{} brightness {} of {}",
            name.unwrap_or_default(),
            now.map_or("?".into(), |v| v.to_string()),
            max.map_or("?".into(), |v| v.to_string())
        ));
    }
    out
}

fn read_u32(p: &Path) -> Option<u32> {
    std::fs::read_to_string(p).ok()?.trim().parse().ok()
}

/// The device value for a step: 0 is off, [`MAX_STEP`] the device maximum.
#[must_use]
pub fn step_to_value(step: u8, max: u32) -> u32 {
    let step = u32::from(step.min(MAX_STEP));
    (step * max + u32::from(MAX_STEP) / 2) / u32::from(MAX_STEP)
}

/// The nearest step for a device value.
#[must_use]
pub fn value_to_step(value: u32, max: u32) -> u8 {
    if max == 0 {
        return 0;
    }
    let v = value.min(max);
    ((v * u32::from(MAX_STEP) + max / 2) / max) as u8
}

impl Light {
    /// The front light under `root`, if there is one.
    #[must_use]
    pub fn find(root: &Path) -> Option<Self> {
        let usable = |name: &str| {
            let dir = root.join(name);
            let max = read_u32(&dir.join("max_brightness"))?;
            (max > 0 && dir.join("brightness").exists()).then_some(Self { dir, max })
        };
        if let Ok(want) = std::env::var("SIGMA_LIGHT_DEVICE") {
            if let Some(l) = usable(&want) {
                return Some(l);
            }
        }
        for name in PREFERRED {
            if let Some(l) = usable(name) {
                return Some(l);
            }
        }
        let mut names: Vec<String> = std::fs::read_dir(root)
            .ok()?
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names.iter().find_map(|n| usable(n))
    }

    /// The device's name, for the log.
    #[must_use]
    pub fn name(&self) -> String {
        self.dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// The current step, from the device.
    #[must_use]
    pub fn step(&self) -> Option<u8> {
        read_u32(&self.dir.join("brightness")).map(|v| value_to_step(v, self.max))
    }

    /// Sets the light to `step`.
    ///
    /// # Errors
    /// The `brightness` file could not be written.
    pub fn set_step(&self, step: u8) -> io::Result<()> {
        std::fs::write(
            self.dir.join("brightness"),
            step_to_value(step, self.max).to_string(),
        )
    }
}

/// The mixer file of the Forma ("Frost"), under [`ROOT`].
const MIXER: &str = "tlc5947_bl/color";

/// Warmth steps above the coolest light (eleven stops, 0 to 10).
pub const WARMTH_MAX: u8 = 10;

/// The natural-light mixer: cool white (0) to warm orange ([`WARMTH_MAX`]).
///
/// The Forma's `tlc5947_bl/color` takes 0 to 10 and runs the wrong way
/// round: 10 is cold and 0 is warm. KOReader (`frontend/device/kobo/device.lua`,
/// the `KoboFrost` entry: `nl_min = 0`, `nl_max = 10`, `nl_inverted = true`;
/// `frontend/device/sysfs_light.lua`, `setNaturalBrightness`) writes
/// `nl_max - warmth`, with no dependence on the white light being on or on
/// the order of the two writes. This does the same; the stock reader never
/// reads the file back, so [`Warmth::step`] may find nothing readable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warmth {
    file: PathBuf,
}

impl Warmth {
    /// The mixer under `root`, if there is one.
    #[must_use]
    pub fn find(root: &Path) -> Option<Self> {
        let file = root.join(MIXER);
        file.exists().then_some(Self { file })
    }

    /// The mixer's path, for the log.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.file
    }

    /// The file's value for a warmth step: the scale is inverted.
    #[must_use]
    pub fn step_to_value(step: u8) -> u8 {
        WARMTH_MAX - step.min(WARMTH_MAX)
    }

    /// The current step, if the file can be read and holds a number.
    #[must_use]
    pub fn step(&self) -> Option<u8> {
        let v = read_u32(&self.file)?;
        let v = u8::try_from(v.min(u32::from(WARMTH_MAX))).ok()?;
        Some(WARMTH_MAX - v)
    }

    /// Sets the mixer to `step`.
    ///
    /// # Errors
    /// The `color` file could not be written.
    pub fn set_step(&self, step: u8) -> io::Result<()> {
        std::fs::write(&self.file, Self::step_to_value(step).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake(tag: &str, devices: &[(&str, u32, u32)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("sigma-light-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (name, now, max) in devices {
            let d = root.join(name);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("brightness"), now.to_string()).unwrap();
            std::fs::write(d.join("max_brightness"), max.to_string()).unwrap();
        }
        root
    }

    #[test]
    fn nine_steps_map_to_the_device_range_and_back() {
        for max in [10, 100, 255] {
            assert_eq!(step_to_value(0, max), 0, "off is off");
            assert_eq!(step_to_value(MAX_STEP, max), max, "full is the maximum");
            for s in 0..=MAX_STEP {
                assert_eq!(value_to_step(step_to_value(s, max), max), s, "max {max}");
            }
        }
        let mut last = 0;
        for s in 0..=MAX_STEP {
            let v = step_to_value(s, 100);
            assert!(v >= last);
            last = v;
        }
        assert_eq!(value_to_step(5, 0), 0);
    }

    #[test]
    fn it_prefers_the_known_device_and_falls_back_to_any() {
        let root = fake("pref", &[("other", 3, 9), ("lm3630a_ledb", 40, 100)]);
        let l = Light::find(&root).unwrap();
        assert_eq!(l.name(), "lm3630a_ledb");
        assert_eq!(l.step(), Some(3), "40 of 100 is step 3");
        let root2 = fake("any", &[("zzz", 0, 255), ("aaa", 0, 255)]);
        assert_eq!(Light::find(&root2).unwrap().name(), "aaa");
        let root3 = fake("none", &[]);
        assert!(Light::find(&root3).is_none());
        assert!(describe(&root)
            .iter()
            .any(|l| l.starts_with("other brightness 3 of 9")));
    }

    #[test]
    fn setting_a_step_writes_the_device_value() {
        let root = fake("set", &[("lm3630a_ledb", 0, 100)]);
        let l = Light::find(&root).unwrap();
        l.set_step(4).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("lm3630a_ledb/brightness")).unwrap(),
            "50"
        );
        assert_eq!(l.step(), Some(4));
        l.set_step(MAX_STEP).unwrap();
        assert_eq!(l.step(), Some(MAX_STEP));
        l.set_step(0).unwrap();
        assert_eq!(l.step(), Some(0));
    }

    #[test]
    fn the_mixer_is_inverted_like_koreaders() {
        let root = fake("mixer", &[]);
        assert!(Warmth::find(&root).is_none(), "no file, no mixer");
        let d = root.join("tlc5947_bl");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("color"), "10").unwrap();
        let w = Warmth::find(&root).unwrap();
        assert_eq!(w.step(), Some(0), "10 in the file is the coolest");
        w.set_step(WARMTH_MAX).unwrap();
        assert_eq!(std::fs::read_to_string(d.join("color")).unwrap(), "0");
        assert_eq!(w.step(), Some(WARMTH_MAX));
        w.set_step(3).unwrap();
        assert_eq!(std::fs::read_to_string(d.join("color")).unwrap(), "7");
        assert_eq!(w.step(), Some(3));
        w.set_step(200).unwrap();
        assert_eq!(
            std::fs::read_to_string(d.join("color")).unwrap(),
            "0",
            "clamped"
        );
        for s in 0..=WARMTH_MAX {
            w.set_step(s).unwrap();
            assert_eq!(w.step(), Some(s));
        }
    }

    #[test]
    fn an_unreadable_mixer_has_no_step() {
        let root = fake("mixer-bad", &[]);
        let d = root.join("tlc5947_bl");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("color"), "warm").unwrap();
        let w = Warmth::find(&root).unwrap();
        assert_eq!(w.step(), None);
        std::fs::write(d.join("color"), "999").unwrap();
        assert_eq!(w.step(), Some(0), "an out-of-range value is the coolest");
    }
}
