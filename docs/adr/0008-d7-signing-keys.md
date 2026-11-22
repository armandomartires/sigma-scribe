# ADR 0008 — D7 — Armando holds signing keys and generates them only on his go

- **Status:** Accepted
- **Date:** 2026-11-17 (Europe/Brussels)
- **Source:** Armando's answers on [dev!1 note 2166](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/merge_requests/1#note_2166), relayed by Alcott, quoting his own words. The plan MR was merged with "Merge !1 with these answers".
- **Plan:** [2026-11-17-cross-platform-remediation.md](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/blob/main/docs/plans/2026-11-17-cross-platform-remediation.md). Tracking: [dev#1](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/issues/1).

## Context

Plugin-index Ed25519 signatures (P1-06) and release signatures (P3-01) need a holder and a store. The recommendation was that Armando holds the keys, they are generated only after this decision, and private keys never enter the repositories, CI logs, or the plan.

## Decision

Armando: "I hold the keys, generate only on my go".

## Consequences

No signing keys are generated or stored in this repository or in CI. Plugin-index signing and release signatures wait for a later, explicit go. Public verification material may be added only after that.
