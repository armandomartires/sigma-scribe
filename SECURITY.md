# Security policy

## Supported versions

The default branch (`main`) and the latest release tag of Σscribe are supported for security fixes. Older release tags are not regularly backported to unless a specific version is named here later.

The current package version is in the root `Cargo.toml` (today 1.21.1). The latest tag at the time of writing is `v1.21.1`.

## Reporting a vulnerability

Please do **not** open a public issue for a report that could put users at risk if it were disclosed early.

**Non-sensitive reports** (for example a typo in docs, or a bug with no realistic security impact): open a GitLab issue on this project:

https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/app/-/issues/new

**Sensitive reports**: open a *confidential* GitLab issue on the same project (the “This issue is confidential” option when creating the issue). Confidential issues are visible only to members of the project. This is the private channel until a separate contact is named here.

In every report, and in patches or logs you attach:

- Mask device serials as `<FORMA-SERIAL>`.
- Do not include tokens, passwords, private keys, or other secrets.
- Mask hostnames if they are not already public.

We will acknowledge a sensitive report on the confidential issue and say what we plan to do. Please give us a reasonable time to fix and release before any public disclosure.

GitLab is the canonical place to report. The GitHub mirror is not monitored as a security inbox.
