# Releasing

Releases are created by GitHub Actions from tags named `v*`.

## One-time setup

In the GitHub repository settings, enable Actions for the repository. The
release workflow uses the repository `GITHUB_TOKEN` with `contents: write`, so
no personal access token is required for creating the GitHub release.

`BUILDBUDDY_API_KEY` is optional for the fork. If it is missing, Bazel still
runs; remote cache access may just be unavailable.

## Create a Release

From a clean working tree on the commit you want to release:

```sh
git switch main
git pull --ff-only upstream main
git merge --ff-only codex/custom-builtins
```

Choose the next fork prerelease version and matching tag:

```sh
git tag v0.1.23-kwargs.4
git push origin main
git push origin v0.1.23-kwargs.4
```

Pushing the `v*` tag starts `.github/workflows/release.yml`. The workflow builds
Linux, macOS arm64, and Windows binaries, creates archives for install tools,
and publishes a GitHub release with generated release notes.

## Re-run a Release

If the tag exists but the release job needs to be retried, use the manual
`Release` workflow in GitHub Actions and pass the existing tag, for example:

```text
v0.1.23-kwargs.4
```

The workflow checks out that tag and runs:

```sh
gh release create "$RELEASE_TAG" release/* --title "$RELEASE_TAG" --generate-notes --verify-tag
```

If a release already exists for the tag, delete the failed or partial release in
GitHub first, then run the workflow again.
