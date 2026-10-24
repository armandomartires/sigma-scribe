# S9 — Release (GitHub Releases + MobileRead)

**Release:** v1.0.0 · **Depends on:** S8

## Goal
Package and publish v1.0.0 for Kobo Forma (firmware 4.x) users.

## Scope
- Final regression on the Forma; release workflow on tags (build ARM + FBInk, tarball, sha256, source)
- README + screenshots (public screenshots allowed), install/uninstall docs
- GitHub Release `v1.0.0` (AGPL-3.0, SIGMArecon); MobileRead thread draft (post only with approval)

## Deliverables
- `sigma-scribe-1.0.0-kobo.tar.gz` + `.sha256` + release notes

## Definition of done
- Fresh install from the release tarball works on the Forma; update over a previous version keeps notes; uninstall leaves the stock reader and notes intact.

## Test checklist
- [ ] Install from release tarball · [ ] update keeps notes · [ ] uninstall clean · [ ] install plan checklist passes

Tasks: `tasks/S9-tasks.md`

## As built (2026-10-21, v1.0.0 release candidate — install PENDING APPROVAL, not tagged, not released)
- `.github/workflows/release.yml`, `scripts/check-package.sh`, reproducible tarball in `scripts/package-kobo.sh`, `THIRD-PARTY.txt`.
- README, `docs/install.md`, `docs/release/` (v1.0.0 notes, MobileRead draft, Forma notes draft, checklist), `tests/emu/v1.0-tour.script`, `docs/screenshots/v1.0/`.
- Tests: `app/tests/release.rs` (CHANGELOG/notes for the version, README images exist, package tree and notices, FBInk pin in both workflows, About version).
- Not done (by instruction / no device): T01 device regression, T04 publishing, T05/T06 posting.

## Decisions
- S9-D1: the release candidate and the device test use the CI-built package; local builds are for development (different toolchain → different binary hash).
- S9-D2: the release workflow only creates a **draft** release, and only for a tag the maintainer pushes; publishing, repo visibility and announcements are manual, approved steps (`docs/release/checklist.md`).
- S9-D3: the fresh-install and uninstall tests delete the app folder on the device, so they are separate approval items after the upgrade test.
- S9-D4: the package carries `THIRD-PARTY.txt`; the upgrade still replaces the binary only.
