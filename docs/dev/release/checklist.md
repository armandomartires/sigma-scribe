# Release checklist (v1.0.0) — every step after "device test" needs Armando's approval

1. Device test: the 0.4.0 → 1.0.0 install (`.ai/install-plan-forma.md`) passes on the Forma, plus fresh install from the tarball, update keeps notes, uninstall clean (S9 DoD). Use the **CI-built** package (release workflow artifact of the merge commit), because that is what the release ships.
2. Record the device result in `CHANGELOG.md` and `.ai/plan.md` (decision), merge.
3. *(approval)* Tag: `git tag -a v1.0.0 -m "Σscribe 1.0.0" && git push origin v1.0.0`. The release workflow checks tag = Cargo version, rebuilds, and creates a **draft** release with the tarball and hashes. Compare the draft's hashes with the tested package.
4. *(approval)* Decide repo visibility (it is private; an AGPL release needs public source — making it public is a separate decision).
5. *(approval)* Publish the draft release (`gh release edit v1.0.0 --draft=false` or in the web UI), notes from `docs/dev/release/v1.0.0-notes.md`.
6. *(approval)* MobileRead: post `docs/dev/release/mobileread-draft.md` in Kobo Developer's Corner; optionally `docs/dev/release/forma-device-notes-draft.md`.
Nothing is sent to Cobalt.
