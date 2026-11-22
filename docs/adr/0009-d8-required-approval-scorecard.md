# ADR 0009 — D8 — Require one approval and a Scorecard job

- **Status:** Accepted
- **Date:** 2026-11-17 (Europe/Brussels)
- **Source:** Armando's answers on [dev!1 note 2166](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/merge_requests/1#note_2166), relayed by Alcott, quoting his own words. The plan MR was merged with "Merge !1 with these answers".
- **Plan:** [2026-11-17-cross-platform-remediation.md](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/blob/main/docs/plans/2026-11-17-cross-platform-remediation.md). Tracking: [dev#1](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/issues/1).

## Context

GitLab already restricts who can push to protected branches. Required approvals and a Scorecard job had not been measured. The recommendation was to require at least one approval on `main` (and plugins `main`) after GitLab CI existed, and to add a Scorecard (or GitLab security-dashboard) job.

## Decision

Armando: "require 1 approval plus Scorecard job".

## Consequences

This is accepted policy. GitLab CE does not offer required approvals as a Premium feature; the practical enforcement is therefore limited until that is available or a substitute is agreed. A Scorecard job needs a GitHub read-only token, which Armando said he would provide later. Neither is implemented by this record alone.
