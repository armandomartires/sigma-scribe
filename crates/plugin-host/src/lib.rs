//! Runs a Sigma Scribe screen plugin: a WASM module speaking the
//! `sigma-plugin-api` wire format.
//!
//! The plugin is untrusted. It is validated on load, runs in a sandbox with
//! no imports except the five below, and every call has a fuel budget and a
//! memory ceiling, so a broken plugin fails with an [`Error`] instead of
//! hanging the e-ink UI. After any failure the plugin is poisoned: the host
//! drops it and falls back, it never reuses a half-run instance.
//!
//! Imports (module `sigma`):
//! - `text_width(face: i32, size: f32, ptr: i32, len: i32) -> f32`
//! - `ascent(face: i32, size: f32) -> f32`
//! - `tr(ptr, len, out, cap) -> i32`: translated text into `out`; bytes
//!   written, or -1 when `cap` is too small.
//! - `cache_get(key: i64, out: i32, cap: i32) -> i32`: bytes written, -1
//!   for a miss, -2 when `cap` is too small.
//! - `cache_put(key: i64, ptr: i32, len: i32) -> i32`: 0, or -1 on failure.

mod cache;

pub use cache::DirCache;

use sigma_plugin_api::{Event, Face, Frame, ABI, MAX_STR};
use wasmi::{
    Caller, Engine, Extern, Instance, Linker, Memory, Module, Store, StoreLimits, TypedFunc,
};

/// What the host provides to a plugin: text metrics and translation.
pub trait Env {
    fn text_width(&self, face: Face, size: f32, s: &str) -> f32;
    fn ascent(&self, face: Face, size: f32) -> f32;
    fn tr(&self, s: &str) -> String;
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Fuel per call (about one unit per executed instruction).
    pub fuel: u64,
    /// Linear memory ceiling in bytes.
    pub memory: usize,
    /// Largest encoded event or frame, in bytes.
    pub message: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            fuel: 20_000_000_000,
            memory: 64 << 20,
            message: 16 << 20,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// Not valid WASM, wrong imports, or missing exports.
    Load(String),
    /// The plugin speaks another wire format version.
    Abi(u32),
    /// A trap, fuel exhaustion or memory limit.
    Trap(String),
    /// The plugin returned bytes that are not a valid frame.
    Decode,
    /// A message or pointer was out of bounds.
    Bounds,
    /// A previous call failed; the instance is not used again.
    Poisoned,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Load(s) => write!(f, "plugin failed to load: {s}"),
            Self::Abi(v) => write!(f, "plugin ABI {v}, host ABI {ABI}"),
            Self::Trap(s) => write!(f, "plugin trapped: {s}"),
            Self::Decode => f.write_str("plugin returned an invalid frame"),
            Self::Bounds => f.write_str("plugin message out of bounds"),
            Self::Poisoned => f.write_str("plugin stopped after an earlier failure"),
        }
    }
}

impl std::error::Error for Error {}

struct State {
    env: Box<dyn Env>,
    cache: DirCache,
    limits: StoreLimits,
}

pub struct Plugin {
    store: Store<State>,
    memory: Memory,
    alloc: TypedFunc<u32, u32>,
    call: TypedFunc<(u32, u32), u64>,
    limits: Limits,
    poisoned: bool,
}

fn guest_bytes<'a>(
    caller: &'a Caller<'_, State>,
    mem: Memory,
    ptr: i32,
    len: i32,
) -> Option<&'a [u8]> {
    let (ptr, len) = (usize::try_from(ptr).ok()?, usize::try_from(len).ok()?);
    mem.data(caller).get(ptr..ptr.checked_add(len)?)
}

fn memory_of(caller: &Caller<'_, State>) -> Result<Memory, wasmi::Error> {
    caller
        .get_export("memory")
        .and_then(Extern::into_memory)
        .ok_or_else(|| wasmi::Error::new("no memory export"))
}

fn guest_str(caller: &Caller<'_, State>, ptr: i32, len: i32) -> Result<String, wasmi::Error> {
    let mem = memory_of(caller)?;
    if usize::try_from(len).map_or(true, |n| n > MAX_STR) {
        return Err(wasmi::Error::new("string too long"));
    }
    let b = guest_bytes(caller, mem, ptr, len).ok_or_else(|| wasmi::Error::new("bad pointer"))?;
    String::from_utf8(b.to_vec()).map_err(|_| wasmi::Error::new("not utf-8"))
}

fn face_of(v: i32) -> Result<Face, wasmi::Error> {
    u8::try_from(v)
        .ok()
        .and_then(Face::from_u8)
        .ok_or_else(|| wasmi::Error::new("bad face"))
}

fn write_guest(
    caller: &mut Caller<'_, State>,
    out: i32,
    cap: i32,
    data: &[u8],
) -> Result<bool, wasmi::Error> {
    let (Ok(out), Ok(cap)) = (usize::try_from(out), usize::try_from(cap)) else {
        return Err(wasmi::Error::new("bad pointer"));
    };
    if data.len() > cap {
        return Ok(false);
    }
    memory_of(caller)?
        .write(&mut *caller, out, data)
        .map_err(|_| wasmi::Error::new("bad pointer"))?;
    Ok(true)
}

fn link(engine: &Engine, max_message: usize) -> Result<Linker<State>, wasmi::Error> {
    let mut l = <Linker<State>>::new(engine);
    l.func_wrap(
        "sigma",
        "text_width",
        |c: Caller<'_, State>,
         face: i32,
         size: f32,
         ptr: i32,
         len: i32|
         -> Result<f32, wasmi::Error> {
            let s = guest_str(&c, ptr, len)?;
            Ok(c.data().env.text_width(face_of(face)?, size, &s))
        },
    )?;
    l.func_wrap(
        "sigma",
        "ascent",
        |c: Caller<'_, State>, face: i32, size: f32| -> Result<f32, wasmi::Error> {
            Ok(c.data().env.ascent(face_of(face)?, size))
        },
    )?;
    l.func_wrap(
        "sigma",
        "tr",
        |mut c: Caller<'_, State>,
         ptr: i32,
         len: i32,
         out: i32,
         cap: i32|
         -> Result<i32, wasmi::Error> {
            let s = guest_str(&c, ptr, len)?;
            let t = c.data().env.tr(&s);
            Ok(if write_guest(&mut c, out, cap, t.as_bytes())? {
                t.len() as i32
            } else {
                -1
            })
        },
    )?;
    l.func_wrap(
        "sigma",
        "cache_get",
        move |mut c: Caller<'_, State>,
              key: i64,
              out: i32,
              cap: i32|
              -> Result<i32, wasmi::Error> {
            let Some(v) = c.data().cache.get(key as u64) else {
                return Ok(-1);
            };
            if v.len() > max_message {
                return Ok(-1);
            }
            Ok(if write_guest(&mut c, out, cap, &v)? {
                v.len() as i32
            } else {
                -2
            })
        },
    )?;
    l.func_wrap(
        "sigma",
        "cache_put",
        move |c: Caller<'_, State>, key: i64, ptr: i32, len: i32| -> Result<i32, wasmi::Error> {
            if usize::try_from(len).map_or(true, |n| n > max_message) {
                return Ok(-1);
            }
            let mem = memory_of(&c)?;
            let b =
                guest_bytes(&c, mem, ptr, len).ok_or_else(|| wasmi::Error::new("bad pointer"))?;
            Ok(if c.data().cache.put(key as u64, b).is_ok() {
                0
            } else {
                -1
            })
        },
    )?;
    Ok(l)
}

impl Plugin {
    /// Validates and instantiates `wasm`.
    ///
    /// # Errors
    /// [`Error::Load`] for invalid modules, missing exports or imports the
    /// host does not provide; [`Error::Abi`] for another wire version.
    pub fn load(
        wasm: &[u8],
        env: Box<dyn Env>,
        cache: DirCache,
        limits: Limits,
    ) -> Result<Self, Error> {
        let load = |e: &dyn std::fmt::Display| Error::Load(e.to_string());
        let mut config = wasmi::Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, wasm).map_err(|e| load(&e))?;
        let linker = link(&engine, limits.message).map_err(|e| load(&e))?;
        let state = State {
            env,
            cache,
            limits: wasmi::StoreLimitsBuilder::new()
                .memory_size(limits.memory)
                .memories(1)
                .tables(4)
                .instances(1)
                .trap_on_grow_failure(true)
                .build(),
        };
        let mut store = Store::new(&engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_fuel(limits.fuel).map_err(|e| load(&e))?;
        let inst: Instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(|e| load(&e))?;
        let memory = inst
            .get_memory(&store, "memory")
            .ok_or_else(|| Error::Load("no memory export".into()))?;
        let abi = inst
            .get_typed_func::<(), u32>(&store, "sigma_abi")
            .map_err(|e| load(&e))?
            .call(&mut store, ())
            .map_err(|e| Error::Trap(e.to_string()))?;
        if abi != ABI {
            return Err(Error::Abi(abi));
        }
        let alloc = inst
            .get_typed_func(&store, "sigma_alloc")
            .map_err(|e| load(&e))?;
        let call = inst
            .get_typed_func(&store, "sigma_call")
            .map_err(|e| load(&e))?;
        Ok(Self {
            store,
            memory,
            alloc,
            call,
            limits,
            poisoned: false,
        })
    }

    /// Sends `event` and returns the plugin's frame.
    ///
    /// # Errors
    /// Any [`Error`]; after one, every later call fails with
    /// [`Error::Poisoned`].
    pub fn call(&mut self, event: &Event) -> Result<Frame, Error> {
        let out = self.call_raw(&event.encode())?;
        Frame::decode(&out).map_err(|_| {
            self.poisoned = true;
            Error::Decode
        })
    }

    /// Sends raw bytes and returns the raw answer (for tests and tools).
    ///
    /// # Errors
    /// As [`Plugin::call`].
    pub fn call_raw(&mut self, input: &[u8]) -> Result<Vec<u8>, Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        let r = self.call_inner(input);
        if r.is_err() {
            self.poisoned = true;
        }
        r
    }

    fn call_inner(&mut self, input: &[u8]) -> Result<Vec<u8>, Error> {
        let trap = |e: wasmi::Error| Error::Trap(e.to_string());
        if input.len() > self.limits.message {
            return Err(Error::Bounds);
        }
        self.store.set_fuel(self.limits.fuel).map_err(trap)?;
        let len = u32::try_from(input.len()).map_err(|_| Error::Bounds)?;
        let ptr = self.alloc.call(&mut self.store, len).map_err(trap)?;
        self.memory
            .write(&mut self.store, ptr as usize, input)
            .map_err(|_| Error::Bounds)?;
        let packed = self.call.call(&mut self.store, (ptr, len)).map_err(trap)?;
        let (out_ptr, out_len) = ((packed >> 32) as usize, (packed & 0xffff_ffff) as usize);
        if out_len > self.limits.message {
            return Err(Error::Bounds);
        }
        let mut out = vec![0u8; out_len];
        self.memory
            .read(&self.store, out_ptr, &mut out)
            .map_err(|_| Error::Bounds)?;
        Ok(out)
    }

    /// Whether an earlier call failed, so the instance must not be reused.
    #[must_use]
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// Fuel left after the last call (for tuning the budget).
    #[must_use]
    pub fn fuel_left(&self) -> u64 {
        self.store.get_fuel().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests;
