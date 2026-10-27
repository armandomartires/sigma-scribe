# S0 — Foundations & Forma bring-up (Plan B)

**Release:** — · **Depends on:** plan approval

## Goal
Verified backup, working toolchain, and proof that a native FBInk app launched from NickelMenu draws, reads touch/buttons and returns to the stock reader on the Forma.

## Scope
- Backup ✅ · toolchain ✅ (Rust ARM musl, musl.cc gcc, FBInk static, NickelMenu v0.6.0) · Cobalt evaluation ✅ → dropped
- Device layer (FBInk backend, evdev, desktop window + headless) ✅ desktop
- Install NickelMenu + v0.1 on the Forma (`.ai/install-plan-forma.md`) — with Armando
- On-device checklist §5; touch transform confirmed in `touch.conf`

## Deliverables
- `.ai/s0-findings.md`, `.ai/install-plan-forma.md` (checklist results filled in)
- Draft Forma notes for FBInk/NickelMenu users if useful (post only with approval)

## Definition of done
- Install plan executed; checklist GO criteria met; results committed.

## Test checklist
- [ ] `cargo test --workspace` green · [ ] `scripts/package-kobo.sh` builds · [ ] device checklist §5 items 1–16

Tasks: `.ai/tasks/S0-tasks.md`
