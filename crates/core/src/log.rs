//! Leveled logging for the app, the device layer and plugins.
//!
//! Levels are Python's: DEBUG, INFO, WARNING, ERROR, CRITICAL. One global
//! setting (Settings → Logging) turns the log file on and picks the lowest
//! level written to it.
//!
//! - **Logging on:** every line at or above the chosen level goes to the log
//!   file (timestamped, tagged with where it came from); warnings and worse
//!   also still go to stderr, so the launcher's `sigma-scribe.log` keeps the
//!   problems.
//! - **Logging off:** nothing is written to the log file, and stderr gets
//!   INFO and above, which is what the app printed before there were levels.
//!
//! The file rotates: at [`MAX_FILE`] bytes it becomes `<name>.1` (one backup
//! is kept), so it can never fill the Kobo's storage.
//!
//! Hot paths ask [`would_log`] first (one relaxed atomic load), and the macros
//! only build the message when it is wanted.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Severity, with Python's numbers (`logging.DEBUG` is 10, and so on).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug = 10,
    #[default]
    Info = 20,
    Warning = 30,
    Error = 40,
    Critical = 50,
}

impl Level {
    pub const ALL: [Level; 5] = [
        Level::Debug,
        Level::Info,
        Level::Warning,
        Level::Error,
        Level::Critical,
    ];

    /// The name written in the log: `WARNING`.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warning => "WARNING",
            Level::Error => "ERROR",
            Level::Critical => "CRITICAL",
        }
    }

    /// The name in the settings file: `warning`.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warning => "warning",
            Level::Error => "error",
            Level::Critical => "critical",
        }
    }

    /// A level from its name, in any case (`warn` counts as `warning`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|l| l.key() == s)
            .or_else(|| (s == "warn").then_some(Level::Warning))
    }

    /// The level for a Python-style number: anything up to 10 is DEBUG, up to
    /// 20 INFO, and so on; above 40 is CRITICAL. (Plugins send numbers.)
    #[must_use]
    pub fn from_number(n: i32) -> Self {
        match n {
            i32::MIN..=10 => Level::Debug,
            11..=20 => Level::Info,
            21..=30 => Level::Warning,
            31..=40 => Level::Error,
            _ => Level::Critical,
        }
    }
}

/// The log file's name inside the log folder.
pub const FILE_NAME: &str = "sigma-scribe.log";

/// The log file is rotated when a line would take it past this size.
pub const MAX_FILE: u64 = 512 * 1024;

/// Where one line goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Route {
    pub file: bool,
    pub stderr: bool,
}

struct Sink {
    file: File,
    path: PathBuf,
    size: u64,
}

/// A logger. The program has one ([`configure`], [`log`]); tests build their
/// own.
pub struct Logger {
    enabled: bool,
    level: Level,
    dir: Option<PathBuf>,
    sink: Option<Sink>,
    /// The file could not be opened or written: log as if off.
    failed: bool,
    max: u64,
    start: Instant,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    /// A logger that is off.
    #[must_use]
    pub fn new() -> Self {
        Self::with_max(MAX_FILE)
    }

    #[must_use]
    pub fn with_max(max: u64) -> Self {
        Self {
            enabled: false,
            level: Level::Info,
            dir: None,
            sink: None,
            failed: false,
            max,
            start: Instant::now(),
        }
    }

    /// Turns the file on or off and sets its lowest level. `dir` is the log
    /// folder. A change closes the file; it is reopened (appending) by the
    /// next line that needs it.
    pub fn configure(&mut self, enabled: bool, level: Level, dir: Option<PathBuf>) {
        let changed = enabled != self.enabled || dir != self.dir;
        self.enabled = enabled;
        self.level = level;
        if changed {
            self.sink = None;
            self.failed = false;
            self.dir = dir;
        }
    }

    fn on(&self) -> bool {
        self.enabled && !self.failed
    }

    /// Where a line of this level goes.
    #[must_use]
    pub fn route(&self, level: Level) -> Route {
        if self.on() {
            Route {
                file: level >= self.level,
                stderr: level >= Level::Warning,
            }
        } else {
            Route {
                file: false,
                stderr: level >= Level::Info,
            }
        }
    }

    /// The lowest level that goes anywhere.
    #[must_use]
    pub fn floor(&self) -> Level {
        if self.on() {
            self.level.min(Level::Warning)
        } else {
            Level::Info
        }
    }

    /// A log file line (without the newline).
    #[must_use]
    pub fn format(&self, level: Level, target: &str, msg: &str, now: SystemTime) -> String {
        let secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let up = self.start.elapsed();
        // Continuation lines of a multi-line message are indented.
        let msg = msg.replace('\n', "\n    ");
        format!(
            "{} +{}.{:03} {:<8} {target}: {msg}",
            timestamp(secs),
            up.as_secs(),
            up.subsec_millis(),
            level.label()
        )
    }

    /// Writes one line where the route says. Returns what was done.
    pub fn emit(&mut self, level: Level, target: &str, msg: &str) -> Route {
        let mut route = self.route(level);
        if route.file {
            let line = self.format(level, target, msg, SystemTime::now());
            if let Err(e) = self.write_line(&line) {
                eprintln!("log: cannot write the log file: {e}");
                self.failed = true;
                self.sink = None;
                route.file = false;
                route.stderr = level >= Level::Info;
            }
        }
        if route.stderr {
            eprintln!("{target}: {msg}");
        }
        route
    }

    fn open(&mut self) -> io::Result<()> {
        let dir = self
            .dir
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no log folder"))?;
        fs::create_dir_all(&dir)?;
        let path = dir.join(FILE_NAME);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let size = file.metadata()?.len();
        self.sink = Some(Sink { file, path, size });
        // A marker, so each session starts visibly in the file.
        let head = self.format(
            Level::Info,
            "log",
            &format!(
                "logging on (level {}), sigma-scribe {}",
                self.level.label(),
                env!("CARGO_PKG_VERSION")
            ),
            SystemTime::now(),
        );
        self.put(&head)
    }

    fn write_line(&mut self, line: &str) -> io::Result<()> {
        if self.sink.is_none() {
            self.open()?;
        }
        self.put(line)
    }

    /// Appends a line, rotating first if it would pass the size limit.
    fn put(&mut self, line: &str) -> io::Result<()> {
        let add = line.len() as u64 + 1;
        let max = self.max;
        let need_rotate = self
            .sink
            .as_ref()
            .is_some_and(|s| s.size > 0 && s.size + add > max);
        if need_rotate {
            self.rotate()?;
        }
        let s = self
            .sink
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "log closed"))?;
        writeln!(s.file, "{line}")?;
        s.size += add;
        Ok(())
    }

    /// `name` becomes `name.1` (replacing the old backup) and a new file starts.
    fn rotate(&mut self) -> io::Result<()> {
        let Some(old) = self.sink.take() else {
            return Ok(());
        };
        let path = old.path.clone();
        drop(old);
        let backup = backup_path(&path);
        let _ = fs::remove_file(&backup);
        fs::rename(&path, &backup)?;
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        self.sink = Some(Sink {
            file,
            path,
            size: 0,
        });
        Ok(())
    }
}

fn backup_path(path: &Path) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(".1");
    PathBuf::from(p)
}

/// `YYYY-MM-DD HH:MM:SSZ` (UTC) for Unix seconds. Before 2020 the clock is
/// taken to be unset (a Kobo without a battery clock starts in 1970), and
/// the date is shown as question marks; the uptime next to it still orders
/// the lines.
#[must_use]
pub fn timestamp(secs: u64) -> String {
    if secs < 1_577_836_800 {
        return "????-??-?? ??:??:??Z".to_owned();
    }
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

// --- the program's logger ---

fn global() -> &'static Mutex<Logger> {
    static LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();
    LOGGER.get_or_init(|| Mutex::new(Logger::new()))
}

/// The lowest level that goes anywhere, for [`would_log`]. INFO to start.
static FLOOR: AtomicU8 = AtomicU8::new(Level::Info as u8);

fn lock() -> MutexGuard<'static, Logger> {
    global().lock().unwrap_or_else(|e| e.into_inner())
}

/// Turns the log file on or off, sets its lowest level and folder. Takes
/// effect for the next line.
pub fn configure(enabled: bool, level: Level, dir: Option<PathBuf>) {
    let mut g = lock();
    g.configure(enabled, level, dir);
    FLOOR.store(g.floor() as u8, Ordering::Relaxed);
}

/// Whether a line of this level would go anywhere: one atomic load.
#[must_use]
pub fn would_log(level: Level) -> bool {
    level as u8 >= FLOOR.load(Ordering::Relaxed)
}

/// The lowest level that goes anywhere (a plugin asks, to skip work).
#[must_use]
pub fn floor() -> Level {
    Level::from_number(i32::from(FLOOR.load(Ordering::Relaxed)))
}

/// Logs one line. Use the macros: they skip building the message when the
/// level is off.
pub fn log(level: Level, target: &str, args: fmt::Arguments<'_>) {
    if !would_log(level) {
        return;
    }
    let msg = args.to_string();
    let mut g = lock();
    g.emit(level, target, &msg);
    FLOOR.store(g.floor() as u8, Ordering::Relaxed);
}

/// Writes a line to the log file only (when the file is on and wants this
/// level), never to stderr. For a message stderr already has by other means,
/// such as a panic (the default hook prints it).
pub fn file_only(level: Level, target: &str, args: fmt::Arguments<'_>) {
    let mut g = lock();
    if !g.route(level).file {
        return;
    }
    let msg = args.to_string();
    let line = g.format(level, target, &msg, SystemTime::now());
    if g.write_line(&line).is_err() {
        g.failed = true;
        g.sink = None;
    }
    FLOOR.store(g.floor() as u8, Ordering::Relaxed);
}

/// Logs at DEBUG: `debug!("usb", "event {n}")`.
#[macro_export]
macro_rules! debug {
    ($target:expr, $($arg:tt)*) => {
        if $crate::log::would_log($crate::log::Level::Debug) {
            $crate::log::log($crate::log::Level::Debug, $target, format_args!($($arg)*));
        }
    };
}

/// Logs at INFO.
#[macro_export]
macro_rules! info {
    ($target:expr, $($arg:tt)*) => {
        if $crate::log::would_log($crate::log::Level::Info) {
            $crate::log::log($crate::log::Level::Info, $target, format_args!($($arg)*));
        }
    };
}

/// Logs at WARNING.
#[macro_export]
macro_rules! warn {
    ($target:expr, $($arg:tt)*) => {
        if $crate::log::would_log($crate::log::Level::Warning) {
            $crate::log::log($crate::log::Level::Warning, $target, format_args!($($arg)*));
        }
    };
}

/// Logs at ERROR.
#[macro_export]
macro_rules! error {
    ($target:expr, $($arg:tt)*) => {
        if $crate::log::would_log($crate::log::Level::Error) {
            $crate::log::log($crate::log::Level::Error, $target, format_args!($($arg)*));
        }
    };
}

/// Logs at CRITICAL.
#[macro_export]
macro_rules! critical {
    ($target:expr, $($arg:tt)*) => {
        if $crate::log::would_log($crate::log::Level::Critical) {
            $crate::log::log($crate::log::Level::Critical, $target, format_args!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sigma-log-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn levels_are_pythons() {
        assert_eq!(
            Level::ALL.map(|l| l as i32),
            [10, 20, 30, 40, 50],
            "the numbers are logging.DEBUG ... logging.CRITICAL"
        );
        assert!(Level::Debug < Level::Info && Level::Error < Level::Critical);
        assert_eq!(Level::Warning.label(), "WARNING");
        assert_eq!(Level::Warning.key(), "warning");
        for l in Level::ALL {
            assert_eq!(Level::parse(l.key()), Some(l));
            assert_eq!(Level::parse(&l.label().to_lowercase()), Some(l));
            assert_eq!(
                Level::parse(&format!(" {} ", l.label())),
                Some(l),
                "case and space"
            );
        }
        assert_eq!(Level::parse("warn"), Some(Level::Warning));
        assert_eq!(Level::parse("fatal"), None);
        assert_eq!(Level::parse(""), None);
        for (n, l) in [
            (i32::MIN, Level::Debug),
            (0, Level::Debug),
            (10, Level::Debug),
            (11, Level::Info),
            (20, Level::Info),
            (30, Level::Warning),
            (40, Level::Error),
            (41, Level::Critical),
            (999, Level::Critical),
        ] {
            assert_eq!(Level::from_number(n), l, "{n}");
        }
        assert_eq!(Level::default(), Level::Info);
    }

    #[test]
    fn timestamps_are_utc_and_an_unset_clock_is_marked() {
        assert_eq!(timestamp(1_577_836_800), "2020-01-01 00:00:00Z");
        assert_eq!(timestamp(1_700_000_000), "2023-11-14 22:13:20Z");
        assert_eq!(timestamp(1_761_389_527), "2025-10-25 10:52:07Z");
        // A leap day and the end of a year.
        assert_eq!(timestamp(1_709_164_800), "2024-02-29 00:00:00Z");
        assert_eq!(timestamp(1_735_689_599), "2024-12-31 23:59:59Z");
        // 1970 (a Kobo whose clock is not set) and anything before 2020.
        assert_eq!(timestamp(0), "????-??-?? ??:??:??Z");
        assert_eq!(timestamp(1_577_836_799), "????-??-?? ??:??:??Z");
    }

    #[test]
    fn a_line_has_time_uptime_level_source_and_message() {
        let l = Logger::new();
        let t = UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        let line = l.format(Level::Warning, "usb", "host mode off", t);
        assert!(line.starts_with("2023-11-14 22:13:20Z +0."), "{line}");
        assert!(line.ends_with(" WARNING  usb: host mode off"), "{line}");
        assert!(l
            .format(Level::Critical, "x", "y", t)
            .contains(" CRITICAL x: y"));
        // A multi-line message stays one entry, with its continuation indented.
        let two = l.format(Level::Error, "panic", "first\nsecond", t);
        assert!(two.ends_with("panic: first\n    second"), "{two}");
    }

    #[test]
    fn where_lines_go_depends_on_the_switch_and_the_level() {
        let mut l = Logger::new();
        let r = |l: &Logger| Level::ALL.map(|x| l.route(x));
        // Off: nothing in the file; stderr gets INFO and up, as before levels.
        let off = r(&l);
        assert!(off.iter().all(|x| !x.file));
        assert_eq!(off.map(|x| x.stderr), [false, true, true, true, true]);
        assert_eq!(l.floor(), Level::Info);
        // On at WARNING: the file gets WARNING and up, stderr WARNING and up.
        l.configure(true, Level::Warning, Some(PathBuf::from("/nowhere")));
        let on = r(&l);
        assert_eq!(on.map(|x| x.file), [false, false, true, true, true]);
        assert_eq!(on.map(|x| x.stderr), [false, false, true, true, true]);
        assert_eq!(l.floor(), Level::Warning);
        // On at DEBUG: the file gets everything; stderr only the problems.
        l.configure(true, Level::Debug, Some(PathBuf::from("/nowhere")));
        assert_eq!(r(&l).map(|x| x.file), [true; 5]);
        assert_eq!(r(&l).map(|x| x.stderr), [false, false, true, true, true]);
        assert_eq!(l.floor(), Level::Debug);
        // On at CRITICAL: the floor stays at WARNING so stderr still hears problems.
        l.configure(true, Level::Critical, Some(PathBuf::from("/nowhere")));
        assert_eq!(l.floor(), Level::Warning);
    }

    #[test]
    fn the_file_gets_lines_at_or_above_the_level_in_order() {
        let d = dir("file");
        let mut l = Logger::new();
        l.configure(true, Level::Info, Some(d.clone()));
        l.emit(Level::Debug, "app", "too quiet");
        l.emit(Level::Info, "app", "starting");
        l.emit(Level::Error, "plugin/graph-map", "it failed");
        let text = fs::read_to_string(d.join(FILE_NAME)).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "{text}");
        assert!(lines[0].contains("INFO") && lines[0].contains("logging on (level INFO)"));
        assert!(lines[1].ends_with("INFO     app: starting"), "{}", lines[1]);
        assert!(
            lines[2].ends_with("ERROR    plugin/graph-map: it failed"),
            "{}",
            lines[2]
        );
        assert!(!text.contains("too quiet"));
        // Nothing is created until a line needs the file.
        let d2 = dir("lazy");
        let mut l2 = Logger::new();
        l2.configure(true, Level::Error, Some(d2.clone()));
        l2.emit(Level::Info, "app", "not for the file");
        assert!(!d2.exists());
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn switching_off_stops_the_file_and_switching_on_appends() {
        let d = dir("toggle");
        let mut l = Logger::new();
        l.configure(true, Level::Debug, Some(d.clone()));
        l.emit(Level::Info, "app", "one");
        l.configure(false, Level::Debug, Some(d.clone()));
        l.emit(Level::Info, "app", "two (off)");
        l.configure(true, Level::Debug, Some(d.clone()));
        l.emit(Level::Info, "app", "three");
        let text = fs::read_to_string(d.join(FILE_NAME)).unwrap();
        assert!(text.contains("one") && text.contains("three") && !text.contains("two"));
        assert_eq!(
            text.matches("logging on").count(),
            2,
            "each session is marked"
        );
        // A new logger (the next start of the app) appends too.
        let mut again = Logger::new();
        again.configure(true, Level::Debug, Some(d.clone()));
        again.emit(Level::Info, "app", "four");
        let text = fs::read_to_string(d.join(FILE_NAME)).unwrap();
        assert!(text.find("three").unwrap() < text.find("four").unwrap());
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn the_file_rotates_and_keeps_one_backup() {
        let d = dir("rotate");
        let mut l = Logger::with_max(600);
        l.configure(true, Level::Debug, Some(d.clone()));
        for i in 0..60 {
            l.emit(
                Level::Info,
                "app",
                &format!("line number {i:03} with some padding text"),
            );
        }
        let cur = fs::read_to_string(d.join(FILE_NAME)).unwrap();
        let old = fs::read_to_string(d.join(format!("{FILE_NAME}.1"))).unwrap();
        assert!(
            cur.len() <= 600 && old.len() <= 600,
            "{} {}",
            cur.len(),
            old.len()
        );
        assert!(
            cur.contains("line number 059"),
            "the newest line is in the file"
        );
        assert!(
            !d.join(format!("{FILE_NAME}.2")).exists(),
            "one backup only"
        );
        // The backup holds the lines just before the file's, in order.
        let first_cur = cur.lines().find(|l| l.contains("line number")).unwrap();
        let n_cur: usize = first_cur.split("number ").nth(1).unwrap()[..3]
            .parse()
            .unwrap();
        let last_old = old
            .lines()
            .rev()
            .find(|l| l.contains("line number"))
            .unwrap();
        let n_old: usize = last_old.split("number ").nth(1).unwrap()[..3]
            .parse()
            .unwrap();
        assert_eq!(n_old + 1, n_cur);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn a_log_folder_that_cannot_be_used_falls_back_to_stderr_only() {
        let d = dir("bad");
        fs::create_dir_all(&d).unwrap();
        // The "folder" is a file: it cannot be created.
        let blocker = d.join("blocker");
        fs::write(&blocker, b"x").unwrap();
        let mut l = Logger::new();
        l.configure(true, Level::Debug, Some(blocker.join("log")));
        let r = l.emit(Level::Info, "app", "hello");
        assert!(!r.file, "no file");
        assert_eq!(l.floor(), Level::Info, "now logging as if off");
        assert!(!l.route(Level::Debug).file);
        assert!(l.route(Level::Info).stderr);
        // No folder at all behaves the same.
        let mut none = Logger::new();
        none.configure(true, Level::Debug, None);
        assert!(!none.emit(Level::Error, "app", "x").file);
        // Turning it off and on again gives it another try.
        l.configure(true, Level::Debug, Some(d.join("good")));
        assert!(l.emit(Level::Info, "app", "now it works").file);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn the_program_logger_filters_by_level_and_the_macros_skip_work_when_off() {
        // The program's logger is global: this is the only test that uses it.
        let d = dir("global");
        configure(false, Level::Info, None);
        assert!(!would_log(Level::Debug) && would_log(Level::Info));
        let mut built = false;
        crate::debug!("t", "{}", {
            built = true;
            "never"
        });
        assert!(
            !built,
            "a debug line is not even formatted while logging is off"
        );
        configure(true, Level::Debug, Some(d.clone()));
        assert!(would_log(Level::Debug) && floor() == Level::Debug);
        crate::debug!("app", "dbg {}", 1);
        crate::info!("app", "inf");
        crate::warn!("app", "wrn");
        crate::error!("app", "err");
        crate::critical!("app", "crt");
        let text = fs::read_to_string(d.join(FILE_NAME)).unwrap();
        for want in [
            "DEBUG    app: dbg 1",
            "INFO     app: inf",
            "WARNING  app: wrn",
            "ERROR    app: err",
            "CRITICAL app: crt",
        ] {
            assert!(text.contains(want), "{want} in {text}");
        }
        file_only(Level::Critical, "panic", format_args!("only in the file"));
        let text = fs::read_to_string(d.join(FILE_NAME)).unwrap();
        assert!(text.contains("CRITICAL panic: only in the file"));
        configure(true, Level::Error, Some(d.clone()));
        crate::info!("app", "filtered out");
        crate::error!("app", "kept");
        let text = fs::read_to_string(d.join(FILE_NAME)).unwrap();
        assert!(!text.contains("filtered out") && text.contains("kept"));
        configure(false, Level::Info, None);
        assert!(!would_log(Level::Debug));
        fs::remove_dir_all(d).unwrap();
    }
}
