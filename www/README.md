# Tea Docs Website

This directory contains the Next.js docs site for the Tea language repository.

## Principles

- Site content should be grounded in checked-in sources from the repo.
- `stdlib/`, `examples/`, and `tea-cli/src/main.rs` are the primary inputs for the public docs surface.
- Avoid placeholder pages for unsupported features.

## Useful Commands

```bash
cd www
npm run audit
npm run typecheck
npm run build
```

## Audit Coverage

`npm run audit` validates:

- internal `href` values resolve to real routes
- reference pages map to real stdlib sources
- example pages map to real checked-in examples
- banned stale snippet patterns are not present in the app source
