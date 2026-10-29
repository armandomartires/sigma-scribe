# Distribution (replaces Cobalt Store publishing)

The Cobalt Store no longer applies (Plan B, 2026-10-20). Proposed distribution:

## GitHub Releases (primary)
- Repo `sigma-scribe` (AGPL-3.0, author **SIGMArecon**). Tag `vX.Y.Z` per roadmap version.
- Assets: `sigma-scribe-X.Y.Z-kobo.tar.gz` (the `.adds/` tree: binary, `launch.sh`, `touch.conf`, NickelMenu entry, `LICENSE`, OFL texts), `.sha256`, and the source archive (AGPL: source for the exact binary). Release notes = user-visible changes.
- Built by `.github/workflows/release.yml` (pinned FBInk 886f25f1, same container as CI): an artifact on every PR / main push; a **draft** release only when the maintainer pushes a `v*` tag. The CI build is reproducible with itself; local builds differ (toolchain), so the CI artifact is what gets tested and shipped. Steps: `docs/dev/release/checklist.md`; drafts in `docs/dev/release/`.
- Install instructions: (1) install NickelMenu from its own release, (2) unpack our tarball onto the Kobo drive root, (3) eject. Uninstall = delete two paths.
- Public screenshots allowed (decision 2026-10-20): use `docs/screenshots/` and on-device photos.

## MobileRead (announcement & support)
- Thread in *Kobo Developer's Corner* linking the GitHub release, listing tested model/firmware (Forma 4.38.23697), NickelMenu requirement and the uninstall steps. Posting needs Armando's approval.

## Licences
- Our code: AGPL-3.0-only. FBInk (statically linked): GPL-3.0-or-later — compatible, combined work distributed under AGPL-3.0 with FBInk's notice. Fonts: SIL OFL 1.1 (texts shipped). NickelMenu is not bundled (users fetch it; MIT).

## Project page
- `site/index.html` is one static page (inline CSS, no JavaScript) published at https://armandomartires.github.io/sigma-scribe/ by `.github/workflows/pages.yml` on every push to `main` that touches `site/`, `docs/screenshots/` or the workflow (or by hand: *Run workflow*).
- The screenshots are not in `site/`. The workflow copies them into `_site/screenshots/` from `docs/screenshots/readme/` without the `readme-` prefix, and fails if any `screenshots/...` image the page uses is missing. To show a newer set, regenerate `docs/screenshots/readme/` (`tests/emu/readme.script`) under the names the page uses (write, preview, drawer, graph, settings, handwriting, arabic, japanese, night, landscape).
- One-time setup: Settings → Pages → Source: GitHub Actions. Local preview: `site/README.md`.
- Keep the tested-model line on the page in step with `docs/user/install.md`.

## Wiki
- The user guide wiki (https://github.com/armandomartires/sigma-scribe/wiki) is generated from `docs/user/`; never edit it on GitHub. After a change to `docs/user/`:
  ```sh
  git clone https://github.com/armandomartires/sigma-scribe.wiki.git /tmp/wiki   # first time; enable the Wiki in repository settings and create a first page so the repository exists
  scripts/wiki-sync.sh /tmp/wiki          # add --force to overwrite unmarked pages
  cd /tmp/wiki && git add -A && git commit -m "Sync from docs/user" && git push
  ```
- The script writes one page per `docs/user/*.md` (`README.md` becomes `Home`; other names come from the first heading), rewrites links between those pages to wiki links, links to other repository files to `github.com/.../blob/main/...` and images to `raw.githubusercontent.com/...`, and writes `_Sidebar.md` and `_Footer.md`. Pages it wrote start with a marker comment; ones whose source is gone are removed, other wiki pages are left alone. If a page it would write already exists without the marker on its first line (a hand-written wiki page), it stops before writing anything and exits non-zero; `--force` overwrites such pages. It prints what changed and warns about links whose target is missing. It does not commit or push.

## Still open
- Display name (“Σscribe”) and release cadence; whether to also offer a one-file `KoboRoot.tgz` that unpacks into `/mnt/onboard/.adds` (works, but `.adds` copy is simpler and safer).
