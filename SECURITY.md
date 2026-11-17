# Security policy

## Supported versions

The default branch (`main`) and the latest release tag of Σscribe are supported for security fixes. Older release tags are not regularly backported unless a specific version is named here later.

The current package version is in the root `Cargo.toml` (today 1.21.1). The latest tag at the time of writing is `v1.21.1`.

## Reporting a vulnerability

Please do **not** open a public issue for a report that could put users at risk if it were disclosed early.

**Sensitive reports**: use GitHub private vulnerability reporting:

https://github.com/armandomartires/sigma-scribe/security/advisories/new

**Non-sensitive reports** (for example a typo in docs, or a bug with no realistic security impact): open a GitHub issue:

https://github.com/armandomartires/sigma-scribe/issues

Project members may also open a GitLab issue on this project. The GitLab project is private and sign-up is disabled, so this path is for project members only:

https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/app/-/issues/new

In every report, and in patches or logs you attach:

- Mask device serials as `<FORMA-SERIAL>`.
- Do not include tokens, passwords, private keys, or other secrets.
- Mask hostnames if they are not already public.

We will acknowledge a sensitive report on the GitHub advisory and say what we plan to do. Please give us a reasonable time to fix and release before any public disclosure.
