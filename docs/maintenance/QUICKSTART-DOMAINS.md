# Quick Start: Setting Up tea-lang.dev for Installation

This is a quick guide to get the install script live at `https://tea-lang.dev/install.sh`.

## Option 1: GitHub Pages (Recommended - 10 minutes)

### Step 1: Create gh-pages Branch

```bash
cd /path/to/tea-lang
git checkout -b gh-pages
git rm -rf .
cp install.sh .
echo "tea-lang.dev" > CNAME
git add install.sh CNAME
git commit -m "Add install script for GitHub Pages"
git push origin gh-pages
git checkout main
```

### Step 2: Enable GitHub Pages

1. Go to https://github.com/bentruyman/tea-lang/settings/pages
2. Source: Deploy from a branch → `gh-pages` → `/root`
3. Custom domain: `tea-lang.dev`
4. Check "Enforce HTTPS" (may need to wait for DNS first)

### Step 3: Configure DNS

In your domain registrar (wherever you registered tea-lang.dev):

**Add these DNS records:**

```
Type    Name    Value                           TTL
A       @       185.199.108.153                 3600
A       @       185.199.109.153                 3600
A       @       185.199.110.153                 3600
A       @       185.199.111.153                 3600
CNAME   www     bentruyman.github.io            3600
```

**OR if CNAME is allowed at apex:**

```
Type    Name    Value                           TTL
CNAME   @       bentruyman.github.io            3600
CNAME   www     bentruyman.github.io            3600
```

### Step 4: Wait & Test

DNS propagation can take 5 minutes to 24 hours. Check with:

```bash
# Check DNS
dig tea-lang.dev

# Test once DNS is ready
curl -I https://tea-lang.dev/install.sh
```

Once it returns `200 OK`, test the installation:

```bash
curl -fsSL https://tea-lang.dev/install.sh | head -10
```

## Option 2: Cloudflare Pages (More Features - 15 minutes)

### Step 1: Connect GitHub to Cloudflare

1. Log into Cloudflare
2. Go to Workers & Pages → Create → Pages → Connect to Git
3. Select `tea-lang` repository
4. Build settings:
   - Build command: `mkdir -p dist && cp install.sh dist/`
   - Build output directory: `dist`
5. Deploy

### Step 2: Add Custom Domain

1. In Cloudflare Pages → Your project → Custom domains
2. Click "Set up a custom domain"
3. Enter `tea-lang.dev`
4. Cloudflare will automatically configure DNS if domain is in same account

### Step 3: Test

```bash
curl -fsSL https://tea-lang.dev/install.sh
```

## Temporary Solution: Use GitHub Raw URL

Until DNS is configured, update README.md to use:

```bash
curl -fsSL https://raw.githubusercontent.com/bentruyman/tea-lang/main/install.sh | bash
```

## What About tea-lang.com?

Save `tea-lang.com` for:

- Marketing/landing page
- Project showcase
- Blog/announcements

You can set it up later with a static site generator like:

- Astro (modern, fast)
- Next.js (React-based)
- Hugo (Go-based, very fast)

For now, you could just redirect `tea-lang.com` → `tea-lang.dev` or to the GitHub repo.

## Next Steps After DNS Setup

1. Update the README installation URL (already done - points to tea-lang.dev)
2. Test installation on fresh systems
3. Add Windows installer (install.ps1) to gh-pages branch
4. Consider adding prebuilt binaries to releases

## Troubleshooting

**DNS not propagating?**

- Check with `dig tea-lang.dev`
- Try `curl -I http://tea-lang.dev` (HTTP first, then HTTPS)
- Wait 24 hours for full global propagation

**GitHub Pages not working?**

- Verify CNAME file contains exactly: `tea-lang.dev`
- Check Settings → Pages shows "Your site is published at https://tea-lang.dev"
- Look for errors in the Pages deployment logs

**HTTPS certificate issues?**

- GitHub Pages provides free SSL via Let's Encrypt
- May take a few minutes after DNS propagation
- Force-refresh the Pages settings to trigger cert generation

## Questions?

Open an issue or discussion in the repo!
