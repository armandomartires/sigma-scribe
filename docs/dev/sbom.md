# Software bill of materials (P3-01, keyless slice)

CI attaches a CycloneDX 1.6 JSON SBOM for the Kobo `sigma-scribe`
binary on every merge-request and `main` pipeline (the `sbom` job).
This is the start of plan item P3-01. It does not publish a release,
create a tag, or sign anything.

## What the file covers

`dist/sbom/sigma-scribe-kobo.cdx.json` is produced by
[cargo-cyclonedx](https://github.com/CycloneDX/cyclonedx-rust-cargo)
0.5.10 (Apache-2.0, so it is fine to use in this AGPL-3.0-only tree)
from `app/Cargo.toml` with `--no-default-features --features kobo` and
`--target armv7-unknown-linux-musleabihf`.

It lists the Cargo crates that link into that binary, with their SPDX
licence expressions. Licence policy stays in `deny.toml` (the
`deny:bans` job); every licence in the SBOM must already be on that
allow list.

It does **not** list non-crate files that the Kobo tarball also ships:

- FBInk (statically linked, GPL-3.0-or-later, pinned commit in CI)
- the musl.cc toolchain used to build
- the bundled Syncthing binary and its licence files
- fonts, dictionaries, and the bundled plugin wasm

Those remain documented in `docs/dev/publishing.md` and in the tarball
itself. Extending the SBOM to name them is a follow-up, as are
build-provenance attestations and a signature.

## Generate locally

Install the same pinned binary CI uses (x86_64 musl, sha256
`528be9dd99ae522c3ee14fb881fd0cbf51767e8f12b74248f437e6a66b1a3c1b`):

```sh
ver=0.5.10
archive="cargo-cyclonedx-x86_64-unknown-linux-musl.tar.xz"
url="https://github.com/CycloneDX/cyclonedx-rust-cargo/releases/download/cargo-cyclonedx-${ver}/${archive}"
curl -fsSL -o "/tmp/${archive}" "$url"
echo "528be9dd99ae522c3ee14fb881fd0cbf51767e8f12b74248f437e6a66b1a3c1b  /tmp/${archive}" | sha256sum -c -
tar -xJf "/tmp/${archive}" -C /tmp
install -m 0755 "/tmp/cargo-cyclonedx-x86_64-unknown-linux-musl/cargo-cyclonedx" "$HOME/.local/bin/cargo-cyclonedx"
```

Then, from the repository root, with the `armv7-unknown-linux-musleabihf`
Rust target installed (`rustup target add armv7-unknown-linux-musleabihf`):

```sh
./scripts/generate-sbom.sh
```

`SOURCE_DATE_EPOCH` defaults to the current commit's author time so the
`metadata.timestamp` is reproducible and no random `serialNumber` is
written.

## Verify

- Check `dist/sbom/sigma-scribe-kobo.cdx.json.sha256` against the JSON
  file (`sha256sum -c`).
- Confirm `bomFormat` is `CycloneDX`, `specVersion` is `1.6`, and
  `metadata.component` is `sigma-scribe` with licence `AGPL-3.0-only`.

CI does not compare the JSON against a committed golden file: the
contents move when `Cargo.lock` does. The checksum is for the artifact
of that pipeline.

## What is still waiting

Signatures (cosign or minisign — not chosen yet), provenance
attestations, and attaching the SBOM to a GitLab release all wait on
Armando generating keys (D7) and on a separate, explicit approval
before any tag or published release. This pipeline must not publish to
GitHub Releases or crates.io on its own.
