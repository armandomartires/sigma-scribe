# S9 — Release (GitHub Releases + MobileRead)

**Release:** v1.0.0 · **Depends on:** S8

## Goal
Package and publish v1.0.0 for Kobo Forma (firmware 4.x) users.

## Scope
- Final regression on the Forma; release workflow on tags (build ARM + FBInk, tarball, sha256, source)
- README + screenshots (public screenshots allowed), install/uninstall docs
- GitHub Release `v1.0.0` (AGPL-3.0, Sigma Recon); MobileRead thread draft (post only with approval)

## Deliverables
- `sigma-scribe-1.0.0-kobo.tar.gz` + `.sha256` + release notes

## Definition of done
- Fresh install from the release tarball works on the Forma; update over a previous version keeps notes; uninstall leaves the stock reader and notes intact.

## Test checklist
- [ ] Install from release tarball · [ ] update keeps notes · [ ] uninstall clean · [ ] install plan checklist passes

Tasks: `tasks/S9-tasks.md`
