# Distribution (replaces Cobalt Store publishing)

The Cobalt Store no longer applies (Plan B, 2026-10-20). Proposed distribution:

## GitHub Releases (primary)
- Repo `sigma-scribe` (AGPL-3.0, author **Sigma Recon**). Tag `vX.Y.Z` per roadmap version.
- Assets: `sigma-scribe-X.Y.Z-kobo.tar.gz` (the `.adds/` tree: binary, `launch.sh`, `touch.conf`, NickelMenu entry, `LICENSE`, OFL texts), `.sha256`, and the source archive (AGPL: source for the exact binary). Release notes = user-visible changes.
- Built by CI (`.github/workflows/ci.yml` → later a `release.yml` on tags) so the binary is reproducible from the tagged source; FBInk version pinned.
- Install instructions: (1) install NickelMenu from its own release, (2) unpack our tarball onto the Kobo drive root, (3) eject. Uninstall = delete two paths.
- Public screenshots allowed (decision 2026-10-20): use `docs/screenshots/` and on-device photos.

## MobileRead (announcement & support)
- Thread in *Kobo Developer's Corner* linking the GitHub release, listing tested model/firmware (Forma 4.38.23697), NickelMenu requirement and the uninstall steps. Posting needs Armando's approval.

## Licences
- Our code: AGPL-3.0-only. FBInk (statically linked): GPL-3.0-or-later — compatible, combined work distributed under AGPL-3.0 with FBInk's notice. Fonts: SIL OFL 1.1 (texts shipped). NickelMenu is not bundled (users fetch it; MIT).

## Still open
- Display name (“Sigma Scribe”) and release cadence; whether to also offer a one-file `KoboRoot.tgz` that unpacks into `/mnt/onboard/.adds` (works, but `.adds` copy is simpler and safer).
