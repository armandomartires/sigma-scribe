use super::*;
use sigma_plugin_api::{Action, Op, Rect, Redraw, Tone};

struct TestEnv;
impl Env for TestEnv {
    fn text_width(&self, _: Face, size: f32, s: &str) -> f32 {
        s.chars().count() as f32 * size / 2.0
    }
    fn ascent(&self, _: Face, size: f32) -> f32 {
        size * 0.8
    }
    fn tr(&self, s: &str) -> String {
        s.to_uppercase()
    }
}

fn dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("sigma-host-{name}-{}", std::process::id()))
}

fn plugin_with(wat: &str, d: &std::path::Path, limits: Limits) -> Result<Plugin, Error> {
    let wasm = wat::parse_str(wat).unwrap();
    Plugin::load(
        &wasm,
        Box::new(TestEnv),
        DirCache::new(d.to_path_buf(), 8, 1 << 20),
        limits,
    )
}

fn plugin(wat: &str, name: &str) -> Plugin {
    plugin_with(wat, &dir(name), Limits::default()).unwrap()
}

/// Memory and the two entry points every module has; the ABI is the API's.
fn head() -> String {
    format!(
        r#"
  (memory (export "memory") 2)
  (func (export "sigma_abi") (result i32) i32.const {})
  (func (export "sigma_alloc") (param i32) (result i32) i32.const 1024)
"#,
        sigma_plugin_api::ABI
    )
}

fn module(body: &str) -> String {
    module_with("", body)
}

/// Imports must precede every function in WAT, so they come first.
fn module_with(imports: &str, body: &str) -> String {
    format!("(module {imports} {} {body})", head())
}

fn pack(ptr: &str, len: &str) -> String {
    format!("(i64.or (i64.shl (i64.extend_i32_u {ptr}) (i64.const 32)) (i64.extend_i32_u {len}))")
}

fn echo() -> String {
    module(&format!(
        r#"(func (export "sigma_call") (param $p i32) (param $l i32) (result i64)
             {})"#,
        pack("(local.get $p)", "(local.get $l)")
    ))
}

#[test]
fn echo_round_trips_bytes() {
    let mut p = plugin(&echo(), "echo");
    assert_eq!(p.call_raw(b"hello").unwrap(), b"hello");
    assert_eq!(p.call_raw(b"").unwrap(), b"");
}

#[test]
fn a_frame_is_decoded() {
    let frame = Frame {
        ops: vec![Op::Fill {
            r: Rect::new(0, 0, 4, 4),
            c: Tone::Ink,
        }],
        hits: vec![],
        actions: vec![Action::OpenNote(2)],
        redraw: Redraw::Partial,
        pending: true,
        field: None,
    };
    let bytes = frame.encode();
    let esc: String = bytes.iter().map(|b| format!("\\{b:02x}")).collect();
    let wat = module(&format!(
        r#"(data (i32.const 8192) "{esc}")
           (func (export "sigma_call") (param i32 i32) (result i64)
             {})"#,
        pack("(i32.const 8192)", &format!("(i32.const {})", bytes.len()))
    ));
    let mut p = plugin(&wat, "frame");
    assert_eq!(p.call(&Event::Tap { x: 1, y: 2 }).unwrap(), frame);
}

#[test]
fn garbage_is_a_decode_error_and_poisons() {
    let mut p = plugin(&echo(), "garbage");
    assert!(
        matches!(p.call(&Event::Back), Err(Error::Decode)),
        "an event is not a frame"
    );
    assert!(matches!(p.call_raw(b"x"), Err(Error::Poisoned)));
}

fn cache_guest() -> String {
    module_with(
        r#"(import "sigma" "cache_put" (func $put (param i64 i32 i32) (result i32)))
           (import "sigma" "cache_get" (func $get (param i64 i32 i32) (result i32)))"#,
        &format!(
            r#"(func (export "sigma_call") (param $p i32) (param $l i32) (result i64)
             (local $n i32)
             (if (i32.eq (i32.load8_u (local.get $p)) (i32.const 112))
               (then
                 (drop (call $put (i64.const 42) (i32.add (local.get $p) (i32.const 1))
                                  (i32.sub (local.get $l) (i32.const 1))))
                 (return (i64.const 0))))
             (local.set $n (call $get (i64.const 42) (i32.const 8192) (i32.const 4096)))
             (if (i32.lt_s (local.get $n) (i32.const 0)) (then (local.set $n (i32.const 0))))
             {})"#,
            pack("(i32.const 8192)", "(local.get $n)")
        ),
    )
}

#[test]
fn cache_persists_across_plugin_instances() {
    let d = dir("cache");
    let _ = std::fs::remove_dir_all(&d);
    let mut a = plugin_with(&cache_guest(), &d, Limits::default()).unwrap();
    assert_eq!(a.call_raw(b"g").unwrap(), b"", "a miss reads as empty");
    a.call_raw(b"pXYZ").unwrap();
    assert_eq!(a.call_raw(b"g").unwrap(), b"XYZ");
    drop(a);
    // A new instance (the next app start) sees the saved value.
    let mut b = plugin_with(&cache_guest(), &d, Limits::default()).unwrap();
    assert_eq!(b.call_raw(b"g").unwrap(), b"XYZ");
    std::fs::remove_dir_all(d).unwrap();
}

fn env_guest() -> String {
    module_with(
        r#"(import "sigma" "text_width" (func $tw (param i32 f32 i32 i32) (result f32)))
           (import "sigma" "ascent" (func $asc (param i32 f32) (result f32)))
           (import "sigma" "tr" (func $tr (param i32 i32 i32 i32) (result i32)))"#,
        &format!(
            r#"(func (export "sigma_call") (param $p i32) (param $l i32) (result i64)
             (local $n i32)
             (local.set $n (i32.const 4))
             (if (i32.eq (i32.load8_u (local.get $p)) (i32.const 119))
               (then
                 (f32.store (i32.const 8192)
                   (call $tw (i32.const 0) (f32.const 20) (i32.add (local.get $p) (i32.const 1))
                             (i32.sub (local.get $l) (i32.const 1))))
                 (return {w})))
             (if (i32.eq (i32.load8_u (local.get $p)) (i32.const 97))
               (then
                 (f32.store (i32.const 8192) (call $asc (i32.const 1) (f32.const 10)))
                 (return {w})))
             (if (i32.eq (i32.load8_u (local.get $p)) (i32.const 98))
               (then (drop (call $tw (i32.const 0) (f32.const 1) (i32.const 1000000) (i32.const 4)))))
             (local.set $n (call $tr (i32.add (local.get $p) (i32.const 1))
                                     (i32.sub (local.get $l) (i32.const 1))
                                     (i32.const 8192) (i32.const 3)))
             (if (i32.lt_s (local.get $n) (i32.const 0)) (then (local.set $n (i32.const 0))))
             {t})"#,
            w = pack("(i32.const 8192)", "(i32.const 4)"),
            t = pack("(i32.const 8192)", "(local.get $n)"),
        ),
    )
}

#[test]
fn host_imports_measure_and_translate() {
    let mut p = plugin(&env_guest(), "env");
    let f = |b: Vec<u8>| f32::from_le_bytes(b.try_into().unwrap());
    assert_eq!(f(p.call_raw(b"wabcd").unwrap()), 40.0);
    assert_eq!(f(p.call_raw(b"a").unwrap()), 8.0);
    assert_eq!(p.call_raw(b"tabc").unwrap(), b"ABC");
    // The output buffer is 3 bytes: a longer answer is refused, not truncated.
    assert_eq!(p.call_raw(b"tabcd").unwrap(), b"");
}

#[test]
fn a_bad_guest_pointer_is_a_trap() {
    let mut p = plugin(&env_guest(), "badptr");
    assert!(matches!(p.call_raw(b"b"), Err(Error::Trap(_))));
    assert!(matches!(p.call_raw(b"a"), Err(Error::Poisoned)));
}

#[test]
fn an_endless_loop_runs_out_of_fuel() {
    let wat = module(
        r#"(func (export "sigma_call") (param i32 i32) (result i64)
             (loop $l (br $l)) (i64.const 0))"#,
    );
    let limits = Limits {
        fuel: 100_000,
        ..Limits::default()
    };
    let mut p = plugin_with(&wat, &dir("loop"), limits).unwrap();
    assert!(matches!(p.call_raw(b""), Err(Error::Trap(_))));
    assert!(matches!(p.call_raw(b""), Err(Error::Poisoned)));
}

#[test]
fn fuel_is_refilled_for_every_call() {
    let limits = Limits {
        fuel: 100_000,
        ..Limits::default()
    };
    let mut p = plugin_with(&echo(), &dir("refuel"), limits).unwrap();
    for _ in 0..1000 {
        p.call_raw(b"x").unwrap();
    }
    assert!(p.fuel_left() > 0);
}

#[test]
fn memory_growth_past_the_ceiling_traps() {
    let wat = module(
        r#"(func (export "sigma_call") (param i32 i32) (result i64)
             (drop (memory.grow (i32.const 100))) (i64.const 0))"#,
    );
    let limits = Limits {
        memory: 2 * 65536,
        ..Limits::default()
    };
    let mut p = plugin_with(&wat, &dir("mem"), limits).unwrap();
    assert!(matches!(p.call_raw(b""), Err(Error::Trap(_))));
}

#[test]
fn an_answer_outside_memory_is_refused() {
    let wat = module(&format!(
        r#"(func (export "sigma_call") (param i32 i32) (result i64) {})"#,
        pack("(i32.const 2147483647)", "(i32.const 10)")
    ));
    let mut p = plugin(&wat, "oob");
    assert!(matches!(p.call_raw(b""), Err(Error::Bounds)));
}

#[test]
fn oversized_input_is_refused() {
    let limits = Limits {
        message: 8,
        ..Limits::default()
    };
    let mut p = plugin_with(&echo(), &dir("big"), limits).unwrap();
    assert!(matches!(p.call_raw(&[0; 9]), Err(Error::Bounds)));
}

#[test]
fn bad_modules_do_not_load() {
    let d = dir("load");
    let cache = || DirCache::new(d.clone(), 1, 10);
    let load = |wasm: &[u8]| Plugin::load(wasm, Box::new(TestEnv), cache(), Limits::default());
    assert!(matches!(load(b"not wasm"), Err(Error::Load(_))));
    // Wrong ABI.
    let wat = wat::parse_str(
        r#"(module (memory (export "memory") 1)
             (func (export "sigma_abi") (result i32) i32.const 99)
             (func (export "sigma_alloc") (param i32) (result i32) i32.const 0)
             (func (export "sigma_call") (param i32 i32) (result i64) i64.const 0))"#,
    )
    .unwrap();
    assert!(matches!(load(&wat), Err(Error::Abi(99))));
    // Missing exports.
    let wat = wat::parse_str(r#"(module (memory (export "memory") 1))"#).unwrap();
    assert!(matches!(load(&wat), Err(Error::Load(_))));
    // An import the host does not provide (no file or network access).
    let wat = wat::parse_str(module_with(
        r#"(import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32) (result i32)))"#,
        r#"(func (export "sigma_call") (param i32 i32) (result i64) i64.const 0)"#,
    ))
    .unwrap();
    assert!(matches!(load(&wat), Err(Error::Load(_))));
}

/// An environment that records the plugin's log lines and has a log floor.
struct RecEnv {
    lines: std::rc::Rc<std::cell::RefCell<Vec<(i32, String)>>>,
    floor: i32,
}
impl Env for RecEnv {
    fn text_width(&self, _: Face, _: f32, _: &str) -> f32 {
        0.0
    }
    fn ascent(&self, _: Face, _: f32) -> f32 {
        0.0
    }
    fn tr(&self, s: &str) -> String {
        s.to_owned()
    }
    fn log(&self, level: i32, msg: &str) {
        self.lines.borrow_mut().push((level, msg.to_owned()));
    }
    fn log_floor(&self) -> i32 {
        self.floor
    }
}

fn log_guest() -> String {
    module_with(
        r#"(import "sigma" "log" (func $log (param i32 i32 i32)))
           (import "sigma" "log_level" (func $lvl (result i32)))"#,
        &format!(
            r#"(func (export "sigma_call") (param $p i32) (param $l i32) (result i64)
                 (if (i32.eq (i32.load8_u (local.get $p)) (i32.const 108)) ;; 'l'
                   (then
                     (call $log (i32.const 30) (i32.add (local.get $p) (i32.const 1))
                                (i32.sub (local.get $l) (i32.const 1)))
                     (return (i64.const 0))))
                 (if (i32.eq (i32.load8_u (local.get $p)) (i32.const 98)) ;; 'b'
                   (then (call $log (i32.const 20) (i32.const 1000000) (i32.const 4))))
                 (i32.store (i32.const 8192) (call $lvl))
                 {})"#,
            pack("(i32.const 8192)", "(i32.const 4)")
        ),
    )
}

#[test]
fn a_plugin_can_log_and_ask_the_floor() {
    let lines = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let env = RecEnv {
        lines: lines.clone(),
        floor: 30,
    };
    let wasm = wat::parse_str(log_guest()).unwrap();
    let cache = DirCache::new(dir("log"), 1, 10);
    let mut p = Plugin::load(&wasm, Box::new(env), cache, Limits::default()).unwrap();
    assert_eq!(p.call_raw(b"lhello there").unwrap(), b"");
    assert_eq!(*lines.borrow(), [(30, "hello there".to_owned())]);
    // The floor is what the host says: here WARNING.
    assert_eq!(p.call_raw(b"f").unwrap(), 30i32.to_le_bytes());
    // A line from a bad pointer is a trap, not a crash.
    assert!(matches!(p.call_raw(b"b"), Err(Error::Trap(_))));
}

#[test]
fn an_environment_without_logging_ignores_lines_and_reports_no_floor() {
    // TestEnv uses the defaults: lines are dropped and the floor is 100.
    let wasm = wat::parse_str(log_guest()).unwrap();
    let cache = DirCache::new(dir("nolog"), 1, 10);
    let mut p = Plugin::load(&wasm, Box::new(TestEnv), cache, Limits::default()).unwrap();
    assert_eq!(p.call_raw(b"lignored").unwrap(), b"");
    assert_eq!(p.call_raw(b"f").unwrap(), 100i32.to_le_bytes());
}
