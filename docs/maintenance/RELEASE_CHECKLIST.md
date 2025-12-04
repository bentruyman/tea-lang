# Release Checklist

Use this checklist when preparing for a Tea Language release.

## Pre-Release Preparation

### Code Quality

- [ ] All tests pass: `make test`
- [ ] E2E tests pass: `scripts/e2e.sh`
- [ ] Code is formatted: `cargo fmt --all --check`
- [ ] Tea files are formatted: `tea fmt . --check`
- [ ] No clippy warnings: `cargo clippy --all-targets --all-features`
- [ ] CI/CD pipeline is green on main branch

### Documentation

- [ ] CHANGELOG.md updated with all changes since last release
- [ ] README.md is accurate and up-to-date
- [ ] Version numbers updated in Cargo.toml files
- [ ] Examples run successfully
- [ ] Documentation reflects new features
- [ ] Breaking changes are clearly documented

### Version Bump

Update version in these files:

- [ ] `Cargo.toml` (workspace version)
- [ ] `tea-cli/Cargo.toml`
- [ ] `tea-compiler/Cargo.toml`
- [ ] `tea-runtime/Cargo.toml`
- [ ] `tea-lsp/Cargo.toml`
- [ ] `tea-intrinsics/Cargo.toml`
- [ ] `tea-support/Cargo.toml`

Run after version changes:

```bash
cargo update -w
cargo build --release
```

### Testing the Release Build

- [ ] Build release binaries: `cargo build --release`
- [ ] Test CLI: `./target/release/tea-cli --version`
- [ ] Test LSP: `./target/release/tea-lsp --version`
- [ ] Run examples with release binary
- [ ] Test installation script: `./install.sh`
- [ ] Verify build artifacts work on clean system (if possible)

## Release Process

### 1. Create Release Commit

```bash
git checkout main
git pull origin main

# Update CHANGELOG.md with release date
# Update version numbers in all Cargo.toml files

git add .
git commit -m "Release v0.1.0"
git push origin main
```

### 2. Create Git Tag

```bash
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

### 3. Build Release Artifacts

```bash
# Clean build
cargo clean
cargo build --release --workspace

# Create distribution archives
./scripts/package-release.sh v0.1.0
```

### 4. Create GitHub Release

1. Go to https://github.com/bentruyman/tea-lang/releases/new
2. Select the tag: `v0.1.0`
3. Release title: `Tea Language v0.1.0`
4. Copy relevant section from CHANGELOG.md to release notes
5. Upload binary artifacts:
   - [ ] `tea-cli` binary (macOS)
   - [ ] `tea-cli` binary (Linux x86_64)
   - [ ] `tea-lsp` binary (macOS)
   - [ ] `tea-lsp` binary (Linux x86_64)
   - [ ] Source code archive (auto-generated)
6. Mark as pre-release if beta/alpha
7. Publish release

### 5. Update Installation Infrastructure

- [ ] Update GitHub Pages with new install.sh (if changed)
- [ ] Update any version-specific documentation

### 6. Announcement

- [ ] Post release announcement (GitHub Discussions, social media, etc.)
- [ ] Update any showcase/demo sites
- [ ] Notify early adopters/testers

## Post-Release

### Verification

- [ ] Install via script on clean system
- [ ] Verify release binaries work correctly
- [ ] Check GitHub release page displays correctly
- [ ] Test installation instructions in README

### Start Next Version

- [ ] Create new `[Unreleased]` section in CHANGELOG.md
- [ ] Update version to next development version (e.g., 0.2.0-dev)
- [ ] Create milestone for next release
- [ ] Triage issues for next version

## Release Types

### Major Release (1.0.0)

- Breaking changes
- Significant new features
- API changes
- Full announcement and marketing push

### Minor Release (0.1.0)

- New features
- Non-breaking changes
- Performance improvements
- Blog post announcement

### Patch Release (0.0.1)

- Bug fixes
- Security updates
- Documentation fixes
- Minimal announcement

## Rollback Plan

If critical issues are found after release:

1. **Document the issue** - Create GitHub issue with details
2. **Assess severity** - Is immediate rollback needed?
3. **Quick fix if possible** - Release patch version
4. **Or rollback**:
   ```bash
   git revert [release-commit-hash]
   git push origin main
   ```
5. **Update release notes** - Mark problematic version as yanked
6. **Communicate** - Notify users of the issue and resolution

## Automation Opportunities (Future)

- [ ] Automated version bumping script
- [ ] Binary build automation (GitHub Actions)
- [ ] Cross-platform test matrix
- [ ] Automated changelog generation
- [ ] Release note template generation

---

**Version:** 1.0
**Last Updated:** 2025-11-11
