# Architecture decision records

Short records of decisions that shape the code. The crate map and the
Phase 2 seam are in [`docs/architecture/ARCHITECTURE.md`](../architecture/ARCHITECTURE.md).

| ADR | Title | Status |
|---|---|---|
| [0001](0001-platform-seam.md) | Platform seam: Cargo features and Paths | Accepted |
| [0002](0002-d1-sync-project-30.md) | D1 — Sync project 30 by fast-forward | Accepted |
| [0003](0003-d2-mask-serial-current-files.md) | D2 — Mask the Forma serial in current files only | Accepted |
| [0004](0004-d3-noreply-going-forward.md) | D3 — Leave personal email in history; noreply going forward | Accepted |
| [0005](0005-d4-plugins-main-default.md) | D4 — Create and protect plugins `main` as the default branch | Accepted |
| [0006](0006-d5-gitlab-ci-canonical.md) | D5 — GitLab CI is the canonical gate | Accepted |
| [0007](0007-d6-second-target-deferred.md) | D6 — Second-target pilot, deferred to Phase 3 | Deferred |
| [0008](0008-d7-signing-keys.md) | D7 — Armando holds signing keys and generates them only on his go | Accepted |
| [0009](0009-d8-required-approval-scorecard.md) | D8 — Require one approval and a Scorecard job | Accepted |

D1–D8 were decided on 2026-11-17 (Europe/Brussels) in [dev!1 note 2166](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/merge_requests/1#note_2166). New decisions from this plan onward get a new file here.
