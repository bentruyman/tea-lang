# Domain Setup for Tea Language

This document outlines how to set up `tea-lang.com` and `tea-lang.dev` for the Tea Language project.

## Domain Strategy

### tea-lang.dev

**Purpose**: Developer-focused technical content and installation

**Recommended setup**:

- Host the installation script at `https://tea-lang.dev/install.sh`
- Serve documentation at `https://tea-lang.dev/docs`
- API reference at `https://tea-lang.dev/api`
- Possibly host compiled binaries at `https://tea-lang.dev/releases`

### tea-lang.com

**Purpose**: Main marketing/landing page

**Recommended setup**:

- Marketing-focused landing page
- "Why Tea?" content
- Showcase/examples gallery
- Link to docs at tea-lang.dev
- Blog/announcements

## Hosting Options

### Option 1: GitHub Pages (Free, Simple)

**For tea-lang.dev:**

1. Create a new repo `tea-lang/tea-lang.dev` (or use a `gh-pages` branch in this repo)
2. Add `install.sh` to the root
3. Enable GitHub Pages in repo settings
4. Configure custom domain:
   - Add CNAME record: `tea-lang.dev` → `tea-lang.github.io`
   - Add `CNAME` file to repo with content: `tea-lang.dev`
5. Enable HTTPS in GitHub Pages settings

**Benefits**: Free, automatic HTTPS, simple CI/CD via GitHub Actions

### Option 2: Cloudflare Pages (Free, Fast CDN)

**For tea-lang.dev:**

1. Connect your Cloudflare account to GitHub
2. Set up Cloudflare Pages to deploy from this repo
3. Configure build settings to copy install.sh to output
4. Point `tea-lang.dev` DNS to Cloudflare Pages
5. Cloudflare provides automatic HTTPS and global CDN

**Benefits**: Faster globally, more flexibility, analytics included

### Option 3: Static Site Host (Netlify, Vercel)

Similar to Cloudflare Pages but with different features/pricing.

## DNS Configuration

### Minimum Setup (GitHub Pages)

**tea-lang.dev:**

```
Type    Name    Value                   TTL
CNAME   @       tea-lang.github.io      3600
CNAME   www     tea-lang.github.io      3600
```

**tea-lang.com:**

```
Type    Name    Value                   TTL
CNAME   @       [your-landing-page]     3600
CNAME   www     [your-landing-page]     3600
```

## Installation Script Hosting

### Quick Setup with GitHub Pages

1. Create `gh-pages` branch:

   ```bash
   git checkout --orphan gh-pages
   git rm -rf .
   cp install.sh .
   git add install.sh
   git commit -m "Add install script"
   git push origin gh-pages
   ```

2. Enable GitHub Pages:
   - Go to repo Settings → Pages
   - Source: `gh-pages` branch
   - Custom domain: `tea-lang.dev`
   - Enforce HTTPS: ✓

3. DNS setup in your registrar:
   - Add CNAME: `tea-lang.dev` → `bentruyman.github.io`
   - Wait for DNS propagation (may take up to 24 hours)

4. Test:
   ```bash
   curl -fsSL https://tea-lang.dev/install.sh | head -5
   ```

### Alternative: Direct GitHub Raw URL

For immediate testing before domain setup:

```bash
curl -fsSL https://raw.githubusercontent.com/bentruyman/tea-lang/main/install.sh | bash
```

Update README to use this until domain is configured.

## Documentation Hosting

Consider using one of these for tea-lang.dev/docs:

1. **mdBook** - Rust documentation tool (used by Rust official docs)
   - Beautiful, searchable docs
   - Automatic table of contents
   - Easy CI/CD with GitHub Actions

2. **Docusaurus** - Modern documentation framework
   - React-based
   - Versioning support
   - Good for growing projects

3. **VitePress** - Vue-powered static site
   - Fast, modern
   - Markdown-based
   - Good developer experience

## Recommended Action Plan

### Phase 1: Get Install Script Live (This Week)

1. Set up GitHub Pages with `gh-pages` branch
2. Configure DNS for tea-lang.dev → GitHub Pages
3. Test installation URL works
4. Update README with working URL

### Phase 2: Documentation Site (Next 2 Weeks)

1. Choose documentation framework (recommend mdBook for Rust projects)
2. Convert existing docs to framework
3. Deploy to tea-lang.dev/docs
4. Set up automatic deployment from main branch

### Phase 3: Landing Page (Future)

1. Design and build marketing site for tea-lang.com
2. Showcase examples, features, benefits
3. Link to docs and installation

## File Locations

For serving from tea-lang.dev, the following structure is recommended:

```
/
├── install.sh              # https://tea-lang.dev/install.sh
├── install.ps1             # https://tea-lang.dev/install.ps1 (Windows)
├── docs/                   # https://tea-lang.dev/docs
│   ├── index.html
│   └── ...
├── releases/               # https://tea-lang.dev/releases
│   ├── latest/
│   │   ├── tea-macos-aarch64
│   │   ├── tea-macos-x86_64
│   │   ├── tea-linux-x86_64
│   │   └── ...
│   └── v0.1.0/
│       └── ...
└── index.html             # Landing page or redirect to docs
```

## Next Steps

1. Decide on hosting approach (GitHub Pages recommended for simplicity)
2. Configure DNS records
3. Set up automated deployment
4. Test installation URL
5. Update all references in documentation

---

**Questions or issues?** Open a discussion in the repository.
