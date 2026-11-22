# ADR 0006 — D5 — GitLab CI is the canonical gate

- **Status:** Accepted
- **Date:** 2026-11-17 (Europe/Brussels)
- **Source:** Armando's answers on [dev!1 note 2166](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/merge_requests/1#note_2166), relayed by Alcott, quoting his own words. The plan MR was merged with "Merge !1 with these answers".
- **Plan:** [2026-11-17-cross-platform-remediation.md](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/blob/main/docs/plans/2026-11-17-cross-platform-remediation.md). Tracking: [dev#1](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/issues/1).

## Context

Neither the app nor the plugins repository had a `.gitlab-ci.yml` when the plan was written. The recommendation was to make GitLab CI the canonical gate and keep GitHub Actions only as a temporary mirror until GitLab CI was green on the default branches, then retire Actions in a follow-up.

## Decision

Armando: "GitLab CI canonical, retire Actions once green".

## Consequences

GitLab CI is the canonical gate (app and plugins). GitHub Actions remain until a documented green streak on `main` and a separate approval to retire them (P1-04).
