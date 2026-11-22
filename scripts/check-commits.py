#!/usr/bin/env python3
"""Check that new commit subjects follow Conventional Commits.

Plan item P3-05: only the commits on this branch / merge request are
checked, not existing history, and merge commits are skipped. On the
default branch the job is a no-op, because merges to main are GitLab
merge commits and history is not rewritten.

Usage:
  python3 scripts/check-commits.py              # origin/main..HEAD, or CI range
  python3 scripts/check-commits.py BASE..HEAD
  python3 scripts/check-commits.py --self-test
"""
from __future__ import annotations

import os
import re
import subprocess
import sys

# type is required; scope is optional, as in `feat(app):` or `ci:`.
# A breaking-change bang is allowed: `feat!:` or `feat(api)!:`.
SUBJECT = re.compile(
    r"^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)"
    r"(\([a-z0-9][a-z0-9_/.-]*\))?!?: .+"
)

TYPES = (
    "feat",
    "fix",
    "docs",
    "style",
    "refactor",
    "perf",
    "test",
    "build",
    "ci",
    "chore",
    "revert",
)


def is_conventional(subject: str) -> bool:
    subject = subject.strip()
    return bool(SUBJECT.match(subject))


def is_merge_subject(subject: str) -> bool:
    s = subject.strip()
    return s.startswith("Merge ") or s.startswith("Merge:")


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True).rstrip("\n")


def default_range() -> str | None:
    """Return a git revision range, or None to skip the check."""
    if len(sys.argv) > 1 and not sys.argv[1].startswith("-"):
        return sys.argv[1]

    source = os.environ.get("CI_PIPELINE_SOURCE", "")
    default_branch = os.environ.get("CI_DEFAULT_BRANCH", "main")
    branch = os.environ.get("CI_COMMIT_BRANCH") or os.environ.get(
        "CI_MERGE_REQUEST_TARGET_BRANCH_NAME", ""
    )
    # A pipeline on main itself is a merge or a direct push; do not
    # judge historical or merge-commit subjects.
    if source in {"push", "web", "schedule", "api", "trigger"} and (
        os.environ.get("CI_COMMIT_BRANCH") == default_branch
    ):
        return None

    base = os.environ.get("CI_MERGE_REQUEST_DIFF_BASE_SHA")
    head = os.environ.get("CI_COMMIT_SHA")
    if base and head:
        return f"{base}..{head}"

    for cand in ("origin/main", "origin/" + default_branch, default_branch, "main"):
        try:
            git("rev-parse", "--verify", cand)
            return f"{cand}..HEAD"
        except subprocess.CalledProcessError:
            continue
    raise SystemExit("error: cannot find a main branch to compare against")


def commits_in(rev_range: str) -> list[tuple[str, str, bool]]:
    """Return (sha, subject, is_merge) for each commit in the range."""
    out = git(
        "log",
        "--format=%H%x00%s%x00%P",
        rev_range,
    )
    rows: list[tuple[str, str, bool]] = []
    if not out:
        return rows
    for entry in out.split("\n"):
        if not entry:
            continue
        sha, subject, parents = entry.split("\x00", 2)
        is_merge = len(parents.split()) > 1 or is_merge_subject(subject)
        rows.append((sha, subject, is_merge))
    return rows


def check_range(rev_range: str) -> int:
    rows = commits_in(rev_range)
    checked = 0
    bad: list[tuple[str, str]] = []
    for sha, subject, merge in rows:
        if merge:
            print(f"skip {sha[:12]} (merge): {subject}")
            continue
        checked += 1
        if is_conventional(subject):
            print(f"ok   {sha[:12]}: {subject}")
        else:
            print(f"BAD  {sha[:12]}: {subject}", file=sys.stderr)
            bad.append((sha, subject))
    if bad:
        print(
            f"error: {len(bad)} of {checked} new commit subjects are not "
            "Conventional Commits (`type(scope): subject`; scope is optional).\n"
            "Allowed types: " + ", ".join(TYPES) + ".\n"
            "Merge commits are ignored. Existing history is not rewritten.",
            file=sys.stderr,
        )
        return 1
    print(f"{checked} new commit(s) in {rev_range} match Conventional Commits")
    return 0


def self_test() -> int:
    good = [
        "feat: add a setting",
        "feat(app): extract sigma-app",
        "fix(core)!: reject a bad ABI",
        "docs: describe the platform seam",
        "ci: attach a CycloneDX SBOM for the Kobo binary (P3-01)",
        "chore(release): 1.21.1",
        "revert: undo the broken parser",
        "refactor(plugin-api): tidy the decoder",
    ]
    bad = [
        "Add a setting",
        "FEAT: shouty",
        "feat:missing space",
        "feat(): empty scope",
        "feat(Upper): bad scope",
        "updated changelog",
        "Merge branch 'feat/x' into 'main'",
        "",
    ]
    failed = 0
    for s in good:
        if not is_conventional(s):
            print(f"self-test: should accept: {s!r}", file=sys.stderr)
            failed += 1
    for s in bad:
        if is_conventional(s):
            print(f"self-test: should reject: {s!r}", file=sys.stderr)
            failed += 1
    if not is_merge_subject("Merge branch 'feat/x' into 'main'"):
        print("self-test: merge subject not detected", file=sys.stderr)
        failed += 1
    if failed:
        print(f"self-test: {failed} failure(s)", file=sys.stderr)
        return 1
    print("self-test: ok")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    if "--help" in sys.argv or "-h" in sys.argv:
        print(__doc__.strip())
        return 0
    rev_range = default_range()
    if rev_range is None:
        print("skipping commit-message check on the default branch")
        return 0
    print(f"checking {rev_range}")
    return check_range(rev_range)


if __name__ == "__main__":
    raise SystemExit(main())
