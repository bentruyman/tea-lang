# Beta Release TODO

This document tracks what's needed to complete the initial beta release of Tea Language.

## ✅ Completed

### Documentation & Project Files

- [x] **LICENSE** - MIT license file
- [x] **CHANGELOG.md** - Release tracking with v0.1.0 template
- [x] **CONTRIBUTING.md** - Comprehensive contributor guide
- [x] **RELEASE_CHECKLIST.md** - Step-by-step release process
- [x] **README.md** - Enhanced with badges, better installation docs
- [x] **examples/README.md** - Detailed example catalog with learning path
- [x] **DOMAINS.md** - Domain setup strategy for tea-lang.com/dev
- [x] **docs/QUICKSTART-DOMAINS.md** - Quick domain setup guide

### Installation Experience

- [x] **install.sh** - Automated installer for macOS/Linux
  - Dependency checking
  - Helpful error messages
  - Colored output
  - Tested and working
- [x] README installation section updated
- [x] Troubleshooting guide added

## 🚧 In Progress / High Priority

### Critical Issues

- [ ] **Fix SIGBUS crash in aot_examples test** (HIGH PRIORITY)
  - Test fails with signal 10 (SIGBUS)
  - May indicate memory access issue in AOT compiler
  - Blocks clean test runs
  - Location: `tea-compiler/tests/aot_examples.rs`

### Infrastructure

- [ ] **Set up GitHub Pages for tea-lang.dev** (HIGH PRIORITY)
  - Create `gh-pages` branch with install.sh
  - Configure DNS CNAME records
  - Enable HTTPS
  - See: `docs/QUICKSTART-DOMAINS.md` for instructions

## 📋 Nice to Have (Before Beta)

### Testing

- [ ] Run full test suite on Linux
- [ ] Test installation on Ubuntu/Debian
- [ ] Test installation on RHEL/CentOS
- [ ] Verify all examples run correctly
- [ ] Cross-platform CI for multiple OS versions

### Documentation Improvements

- [ ] Getting Started tutorial (mentioned as "coming soon" in docs)
- [ ] Video or GIF demo for README
- [ ] Language cheat sheet / quick reference
- [ ] More comprehensive troubleshooting scenarios

### Tooling

- [ ] Windows PowerShell installer (install.ps1)
- [ ] Automated release script (package binaries, create archives)
- [ ] Pre-built binaries for GitHub Releases
  - macOS (Apple Silicon)
  - macOS (Intel)
  - Linux (x86_64)
  - Linux (ARM64)

### Polish

- [ ] Verify all doc links work
- [ ] Spellcheck documentation
- [ ] Consistent version numbers across all Cargo.toml files
- [ ] Review error messages for clarity
- [ ] Add more usage examples to docs

## 🔮 Post-Beta (Future Releases)

### Features

- [ ] Package manager / dependency management
- [ ] Improved LSP features (autocomplete, go-to-definition)
- [ ] Debug symbol support
- [ ] Profiling tools
- [ ] REPL (Read-Eval-Print Loop)

### Infrastructure

- [ ] Documentation site (mdBook or similar)
- [ ] Landing page for tea-lang.com
- [ ] Automated binary builds in CI
- [ ] Homebrew formula
- [ ] APT/RPM packages

### Community

- [ ] Discord or community chat
- [ ] Blog for announcements
- [ ] Social media presence
- [ ] Example project showcase

## 🎯 Minimum Viable Beta

To ship a beta, we absolutely need:

1. **LICENSE** ✅
2. **CHANGELOG.md** ✅
3. **Working tests** ⚠️ (SIGBUS issue needs fixing)
4. **Installation script** ✅
5. **Updated README** ✅
6. **Domain setup** ⏳ (in progress)

### Critical Path to Beta

```
1. Fix SIGBUS test failure
   └─> Ensures code quality

2. Set up tea-lang.dev with install.sh
   └─> Enables easy installation

3. Final smoke test
   └─> Install on clean system
   └─> Run examples
   └─> Build a program

4. Create v0.1.0-beta.1 tag
   └─> Push to GitHub

5. Create GitHub Release
   └─> Upload binaries (optional for beta)
   └─> Copy CHANGELOG content

6. Announce
   └─> GitHub Discussions
   └─> README badge update
```

## 📝 Next Steps

**Immediate (This Week):**

1. Debug and fix SIGBUS crash
2. Set up GitHub Pages
3. Test installation end-to-end

**Short Term (Next Week):**

1. Create beta release (v0.1.0-beta.1)
2. Gather early feedback
3. Fix critical issues

**Medium Term (Next Month):**

1. Iterate based on feedback
2. Add Windows support
3. Improve documentation
4. Release v0.1.0 stable

## 💭 Questions to Consider

- **Version number:** Start with v0.1.0-beta.1 or v0.0.1?
- **Supported platforms:** macOS + Linux only, or add Windows?
- **Binary distribution:** GitHub Releases only, or also Homebrew?
- **Documentation hosting:** GitHub wiki, separate site, or just README?
- **Community platform:** GitHub Discussions, Discord, or both?

## 🐛 Known Issues to Document

Add these to CHANGELOG.md known limitations:

1. SIGBUS crash in aot_examples test (investigating)
2. Windows requires manual build (no installer yet)
3. Test execution in AOT mode is experimental
4. Some LLVM optimizations not enabled (LTO)
5. LSP has basic features only (no autocomplete yet)

---

**Last Updated:** 2025-11-11
**Target Beta Date:** TBD (after fixing critical issues)
