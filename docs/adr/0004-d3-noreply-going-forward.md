# ADR 0004 — D3 — Leave personal email in history; noreply going forward

- **Status:** Accepted
- **Date:** 2026-11-17 (Europe/Brussels)
- **Source:** Armando's answers on [dev!1 note 2166](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/merge_requests/1#note_2166), relayed by Alcott, quoting his own words. The plan MR was merged with "Merge !1 with these answers".
- **Plan:** [2026-11-17-cross-platform-remediation.md](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/blob/main/docs/plans/2026-11-17-cross-platform-remediation.md). Tracking: [dev#1](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/issues/1).

## Context

About 120 GitHub web-merge commits used a personal email; the other commits already used a GitHub noreply address. Option A was to leave history and use noreply identities going forward. Option B was to rewrite those merges (same force-push bar as D2-B).

## Decision

Armando: "leave history, noreply going forward".

## Consequences

Existing commits are left as they are. New GitLab commits use a noreply identity (for this bot, `naomi.grokbot@noreply.sigmarecon.pt`). GitHub email privacy is a setting only Armando can confirm on his account.
