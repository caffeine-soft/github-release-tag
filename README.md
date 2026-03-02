# 🏷️ GitHub Release Tag

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/caffeine-soft/github-release-tag?style=flat-square)](https://github.com/caffeine-soft/github-release-tag/releases)
[![Marketplace](https://img.shields.io/badge/Marketplace-GitHub_Release_Tag-blue?style=flat-square&logo=github)](https://github.com/marketplace/actions/github-release-tag)

`github-release-tag` is a blazing fast GitHub Action written in Rust 🦀 that automatically determines the next semantic version, pushes a git tag, creates a GitHub Release, and uploads release assets.

## Features

- **Semantic Versioning:** Analyzes commit messages (Conventional Commits) since the last tag to automatically determine if a `major`, `minor`, or `patch` bump is required.
- **Auto-Changelog:** Generates a Markdown changelog based on the commit history.
- **GitHub Release Creation:** Optionally creates a GitHub release to accompany the pushed tag.
- **Asset Uploading:** Supports glob-based matching (e.g., `release_assets/*`) to automatically upload binaries and other files directly to the GitHub Release.
- **Dry Run:** Allows testing the version bump and changelog generation without pushing tags or creating releases.

## Usage

Create a workflow file (e.g., `.github/workflows/release.yml`) in your repository:

```yaml
name: Release

on:
  push:
    branches:
      - main

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write # Needed to push tags and create releases
    steps:
      - name: Checkout code
        uses: actions/checkout@v6

      # ... (build your project and store artifacts in release_assets/ here)

      - name: Create Release and Upload Assets
        id: release
        uses: caffeine-soft/github-release-tag@v0
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          default_bump: "patch"
          generate_release: "true"
          release_name: "Release {tag}"
          artifacts: "release_assets/*"
```

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `github_token` | Required for permission to tag the repo. | Yes | |
| `default_bump` | Which type of bump to use when none explicitly provided (`major`, `minor`, `patch`, `false`). | No | `patch` |
| `tag_prefix` | A prefix to the tag name. | No | `v` |
| `custom_tag` | Custom tag name. If specified, it overrides bump settings. | No | |
| `release_branches` | Comma separated list of branches that will generate the release tags. | No | `master,main` |
| `dry_run` | Do not perform tagging/release creation, just calculate next version and changelog. | No | `false` |
| `generate_release` | Whether to create a GitHub release object. | No | `false` |
| `release_name` | Template for the release name. Use `{tag}` to insert the generated tag. | No | `Release {tag}` |
| `release_body` | Optional explicit body for the release. If empty, the generated changelog is used. | No | |
| `draft` | Create a draft release. | No | `false` |
| `prerelease` | Create a prerelease. | No | `false` |
| `artifacts` | A comma/newline separated list of glob patterns (e.g., `release_assets/*`) to upload as release assets. | No | |

## Outputs

| Name | Description |
|------|-------------|
| `new_tag` | The newly generated tag (e.g., `v1.2.3`). |
| `new_version` | The generated tag without the prefix (e.g., `1.2.3`). |
| `previous_tag` | The previously highest tag found (or `0.0.0`). |
| `release_type` | The computed release type (`major`, `minor`, `patch`, `custom`, or `none`). |
| `changelog` | The newly generated Markdown changelog since the previous tag. |

## How It Works

This action downloads a pre-compiled Rust binary for the runner's architecture and executes it natively. Because it is written in Rust rather than Node.js or a shell script, it generally boots and runs extremely fast.

### Supported Platforms
- `ubuntu-latest` (x86_64, aarch64)
- `macos-latest` (x86_64, aarch64)
- `windows-latest` (x86_64)
