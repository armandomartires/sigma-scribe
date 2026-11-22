# ADR 0003 — D2 — Mask the Forma serial in current files only

- **Status:** Accepted
- **Date:** 2026-11-17 (Europe/Brussels)
- **Source:** Armando's answers on [dev!1 note 2166](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/merge_requests/1#note_2166), relayed by Alcott, quoting his own words. The plan MR was merged with "Merge !1 with these answers".
- **Plan:** [2026-11-17-cross-platform-remediation.md](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/blob/main/docs/plans/2026-11-17-cross-platform-remediation.md). Tracking: [dev#1](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/issues/1).

## Context

The Forma serial was unmasked in a few public files and remained in git history from 2026-10-24 even after a masking commit. Option A was a normal merge request that replaces current mentions with `<FORMA-SERIAL>`. Option B was a history rewrite and force-push, only if Armando said so in his own words.

## Decision

Armando: "mask in current files only" (no history rewrite; P0-03 is won't-do).

## Consequences

Current files use `<FORMA-SERIAL>`. History is not rewritten. A force-push to purge the serial from history remains out of scope unless he later approves it in his own words.
