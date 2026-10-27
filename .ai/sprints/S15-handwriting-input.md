# S15 — v1.5: Handwriting input panel (Latin) (draft)

> **DRAFT — planning only (updated 2026-10-21 21:24 with the user's answers). No development until the user approves.**
> **2026-10-22:** superseded by the proposal in `.ai/plan.md` §12 (v1.5.0–v1.5.3, ink first, character boxes, gated word mode); pending approval.

**Scope change (user, 2026-10-21 21:24):** no sketching/drawing. Pen mode becomes a **handwriting-to-text input panel** like the Windows one (user's photo: a dark bar "Write here" with a baseline, an Enter key and a dropdown).

## Panel
- Replaces the on-screen keyboard area (toggle on the keyboard bar: Keys ⇄ Write). A dark bar (ink on black with white strokes is DU-friendly on e-ink) with **"Write here"** and a **baseline**; buttons: **Backspace**, **Space**, **Enter**, and a **dropdown** (language, Finger / Stylus / Auto, stroke width).
- Strokes are written on the line; after a pause (~700 ms) or when the writing moves past the right third, the ink is recognised and the **top result is inserted at the caret**; a **candidate row** (top 3–5) lets the user replace it with one tap; Backspace deletes the last inserted word/char. Scratch-out gesture deletes (later).
- Input from **finger or capacitive stylus** (both are the same capacitive contacts on the Forma): Finger / Stylus / **Auto by contact size** (`ABS_MT_TOUCH_MAJOR`/`PRESSURE`, ranges from a 0.5 d device probe; default Finger until Auto is proven). Palm rejection: first contact wins; contacts above the palm size threshold are ignored for 300 ms; two-finger contact is ignored on the panel; contacts outside the bar never ink. Ink width constant in mm (Stylus 0.6 mm, Finger 1.2 mm) — capacitive "pressure" is contact area, not force; stronger smoothing for fingers.
- Uses the v1.3 composer + candidate bar; the same panel later serves Arabic and CJK models.

## Feasibility study — offline, on-device online-HWR
**Device:** Kobo Forma, NXP i.MX6 SLL, single Cortex-A9 ~1 GHz with NEON, 512 MB RAM, ~6.7 GB user storage. No GPU/NPU usable.

| Option | Type / licence | Fit | Verdict |
|---|---|---|---|
| **Tesseract** | offline OCR of images (Apache-2.0) | ignores stroke order/timing; poor on handwriting | not suitable |
| **Google ML Kit Digital Ink** | excellent offline models (~20 MB/lang) but a proprietary SDK for Android/iOS under Google's terms | cannot be linked into a Linux/Kobo AGPL app | **not usable** |
| **MyScript iink** | best-in-class, commercial licence | cost, closed, not AGPL-compatible | not usable |
| **Lipi Toolkit** (HP Labs, MIT) | classic online shape recognisers, isolated chars | dated, weak on cursive | fallback reference only |
| **$P / $Q point-cloud** (published algorithms) | template matching, user-trainable | single letters/gestures, tiny and instant | **fallback** (character mode) |
| **Zinnia** (BSD) + Tomoe models (LGPL) | SVM online character recogniser | CJK single characters, ~1–5 MB, ms latency | **good for v1.7/v1.8 CJK** |
| **Own small neural model**: Bezier/delta features → 1D conv + 2–3 BiLSTM/GRU (128) → CTC, run with **tract** (pure Rust, ONNX LSTM/GRU, armv7 NEON) | architecture of Carbune et al. 2020 ("Fast multi-language LSTM-based online HWR", Google) | ~1–2 M params: 4–8 MB fp32, 1–2 MB int8 | **main path, must be trained by us** |

**Data for training (main risk):** IAM-OnDB (Latin, 221 writers) — registration, research/non-commercial terms; whether distributing a model trained on it in an AGPL app is allowed must be clarified with the IAM group. Others: BRUSH (2022), OnHW (Fraunhofer), Deepwriting (built on IAM-OnDB), UNIPEN (old) — mostly research licences too. Licence-clean supplement: synthetic ink from single-stroke (Hershey-style) fonts + augmentation, and **the user's own ink collected on the device** (opt-in, stored locally) for fine-tuning.
**Runtime estimate on the Forma:** ~2 M MAC per time step × ~150 steps per word ≈ 0.3 GMAC → **~0.3–0.8 s per word** with tract on a 1 GHz A9 (to be measured); CTC beam search with a word list (EN/FR/PT, ~50 k words each, ~0.5 MB) adds ~50–100 ms. RAM < 50 MB. onnxruntime also has armv7 builds but means a heavy C++ cross-build; tract keeps the static Rust build.
**Expected accuracy:** published small online models reach ~5–10 % character error on IAM-OnDB with a lexicon; finger input on a capacitive panel (coarse, jittery, no hover) will be worse — unknown until tested.
**Languages:** Latin (EN/FR/PT, accents) first; Arabic later needs its own data and model (Online-KHATT and similar are research-only) and RTL ink order; CJK via Zinnia-style per-character recognition with the v1.7/v1.8 dictionaries.

**Risk rating: HIGH** (training data licences, training effort, finger quality, A9 latency). **Fallbacks, in order:** (1) **word-at-a-time** recognition with lexicon and candidates (the plan above, write one word, pause); (2) **single-character boxes** (a row of 6–8 boxes, one letter each, small CNN or $P templates — reliable, slower to write); (3) letter gestures (Graffiti-like) with $P, user-trainable.
**Decisions (31):** opt-in, on-device, **local-only** collection of the user's own ink for training is approved (stored under `sigma-scribe/notes/.handwriting/`, never sent anywhere, deletable from Options); **character boxes are an accepted fallback** for v1.5 if word mode fails the gate.
**Plan:** spike first (1 wk): panel + ink logging on desktop/emulator, prototype model trained on licence-clear synthetic data + a small IAM-OnDB evaluation (if allowed), tract timing on armv7 (QEMU estimate, then device) → **go / fallback decision** before the full sprint. **Estimate:** spike 1 wk; full 3–5 wk (word mode) or ~2 wk (character mode).
