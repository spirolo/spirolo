# Website

The landing page and docs, built with [Fumadocs](https://fumadocs.dev) on Next.js as a static
export (`out/`).

```sh
pnpm install
pnpm dev            # http://localhost:3000
just website        # from the repo root: public check, type check, build
```

## Where things come from

- **Pictures:** `../branding`. `scripts/sync.mjs` copies them into `public/brand/` before every
  dev and build, so they are never duplicated in git. New pictures come from the
  maintainers' brand guide, which is not part of this repository.
- **Rule status:** `../corpus/rule-status.json`, the table the binary embeds, copied into
  `lib/generated/`. Tiers, counts and the rules table are computed from it, never typed.
- **The report in the docs and the health score on the home page:** `data/sample-check.json`, the
  unedited output of a real run. Its `_note` says how to regenerate it.
- **The hero's numbers and replay:** `data/hero-runs.json`, real runs on open-source suites. Its
  `_note` says how they were measured.
- **The logo and the replay's coil:** `data/coil.json`, generated with the logo files in
  `../branding/logo/` by the maintainers' logo script (not in this repository).
- **Product name, package and commands:** `lib/product.ts`. Commands in `content/docs/` follow it.

## Nothing private

The site is public. `pnpm check:public` fails on machine paths, email addresses, host logins,
internal tooling, business notes and keys in any tracked file here. Words that are private in
themselves go one per line in a gitignored `.public-denylist`, or in `PUBLIC_DENYLIST`
(comma-separated), so the list itself is never published.

## Deploy

The site is a static export served at https://spirolo.com from the Vercel project
`spirolo-website`. Build locally, because the build reads `../branding` and `../corpus`, then
upload `out/`:

```sh
VERCEL_PROJECT_PRODUCTION_URL=spirolo.com pnpm build
pnpm check:public
cd out && vercel deploy --prod
```

`VERCEL_PROJECT_PRODUCTION_URL` sets the absolute URLs in the page metadata; without it they
point at localhost. `public/vercel.json` lands in `out/` and turns `docs.html` into `/docs`:
the export writes both a `docs.html` and a `docs/` folder, and without the rewrite `/docs`
answers 404.
