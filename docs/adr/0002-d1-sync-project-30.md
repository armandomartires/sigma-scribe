# ADR 0002 — D1 — Sync project 30 by fast-forward

- **Status:** Accepted
- **Date:** 2026-11-17 (Europe/Brussels)
- **Source:** Armando's answers on [dev!1 note 2166](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/merge_requests/1#note_2166), relayed by Alcott, quoting his own words. The plan MR was merged with "Merge !1 with these answers".
- **Plan:** [2026-11-17-cross-platform-remediation.md](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/blob/main/docs/plans/2026-11-17-cross-platform-remediation.md). Tracking: [dev#1](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/issues/1).

## Context

GitHub `main` was 83 commits ahead of GitLab `main`. The push-mirror had `keep_divergent_refs=false`, so a non-fast-forward push to GitLab could have wiped those commits on GitHub.

The recommended option (A) was: disable the push-mirror; fetch GitHub; if GitLab `main` is a strict ancestor, fast-forward GitLab `main` to the GitHub SHA; verify both remotes report the same SHA; re-enable the mirror; only then open app merge requests.

## Decision

Armando: "approve the recommended fast-forward sequence".

## Consequences

That sequence was carried out in Phase 0. GitLab is canonical; GitHub remains a push-mirror only. Later app work proceeds on GitLab `main`. This decision does not authorise a force-push.
