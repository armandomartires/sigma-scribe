# ADR 0005 — D4 — Create and protect plugins `main` as the default branch

- **Status:** Accepted
- **Date:** 2026-11-17 (Europe/Brussels)
- **Source:** Armando's answers on [dev!1 note 2166](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/merge_requests/1#note_2166), relayed by Alcott, quoting his own words. The plan MR was merged with "Merge !1 with these answers".
- **Plan:** [2026-11-17-cross-platform-remediation.md](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/blob/main/docs/plans/2026-11-17-cross-platform-remediation.md). Tracking: [dev#1](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/issues/1).

## Context

The plugins repository had no `main`; the default branch was `pane-plugins`, and GitLab protected `landing-page` rather than the default. The recommended path was to create `main` from `pane-plugins`, switch the default, protect `main`, and leave `pane-plugins` in place for a time.

## Decision

Armando: "create and protect main as default".

## Consequences

Plugins `main` is the default protected branch. Further tidy-up of stale remote branches still needs a separate approval (ONG-08).
