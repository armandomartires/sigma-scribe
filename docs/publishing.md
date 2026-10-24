# Distribution (replaces Cobalt Store publishing)

The Cobalt Store no longer applies (Plan B, 2026-10-20). Proposed distribution:

## GitHub Releases (primary)
- Repo `sigma-scribe` (AGPL-3.0, author **SIGMArecon**). Tag `vX.Y.Z` per roadmap version.
- Assets: `sigma-scribe-X.Y.Z-kobo.tar.gz` (the `.adds/` tree: binary, `launch.sh`, `touch.conf`, NickelMenu entry, `LICENSE`, OFL texts), `.sha256`, and the source archive (AGPL: source for the exact binary). Release notes = user-visible changes.
- Built by `.github/workflows/release.yml` (pinned FBInk 886f25f1, same container as CI): an artifact on every PR / main push; a **draft** release only when the maintainer pushes a `v*` tag. The CI build is reproducible with itself; local builds differ (toolchain), so the CI artifact is what gets tested and shipped. Steps: `docs/release/checklist.md`; drafts in `docs/release/`.
- Install instructions: (1) install NickelMenu from its own release, (2) unpack our tarball onto the Kobo drive root, (3) eject. Uninstall = delete two paths.
- Public screenshots allowed (decision 2026-10-20): use `docs/screenshots/` and on-device photos.

## MobileRead (announcement & support)
- Thread in *Kobo Developer's Corner* linking the GitHub release, listing tested model/firmware (Forma 4.38.23697), NickelMenu requirement and the uninstall steps. Posting needs Armando's approval.

## Licences
- Our code: AGPL-3.0-only. FBInk (statically linked): GPL-3.0-or-later — compatible, combined work distributed under AGPL-3.0 with FBInk's notice. Fonts: SIL OFL 1.1 (texts shipped). NickelMenu is not bundled (users fetch it; MIT).

## Still open
- Display name (“Σscribe”) and release cadence; whether to also offer a one-file `KoboRoot.tgz` that unpacks into `/mnt/onboard/.adds` (works, but `.adds` copy is simpler and safer).
