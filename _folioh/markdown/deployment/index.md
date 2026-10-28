# Deployment

Folioh builds a static site into `_site/`. Deploy that folder to any static host.
The internal `.build/` workspace is only a cache and should not be committed or
used as the public artifact.

Deployment has one artifact and several delivery paths:

| Path | Use when |
|------|----------|
| Static hosts | You want Vercel, Netlify, or a static web server to build or serve `_site/`. |
| GitHub Pages | You want production docs and optional PR previews hosted by GitHub Pages. |
| CI/CD | You want GitHub Actions or another pipeline to build, check, and publish docs automatically. |

- **[Static Hosts](/docs/deployment/static-hosts)**: Deploy _site/ to Vercel, Netlify, or a small self-hosted static server.

- **[GitHub Pages](/docs/deployment/github-pages)**: Configure base paths, Pages artifacts, production deploys, and branch previews.

- **[CI/CD](/docs/deployment/ci-cd)**: Automate deployment, branch previews, pull request checks, and coverage gates.

## Build Once

With Folioh [installed](/docs/installation), run the build from your project root:

```bash
folioh build --clean
```

The `_site/` directory contains HTML, assets, Pagefind search data, `llms.txt`,
`llms-full.txt`, and the Markdown mirrors and authoring contract under `_folioh/`.

```mermaid
flowchart LR
    Source["Source code and Markdown"] --> Build["folioh build --clean"]
    Build --> Site["_site/ static artifact"]
    Site --> Host["Static host"]
    Site --> Pages["GitHub Pages"]
    Site --> Pipeline["CI/CD publish job"]
```

## Navigation

HTTP links use directory URLs such as `/docs/installation/`, including after
client navigation. The exported HTML retains explicit `index.html` targets for
links opened directly from the filesystem. Configure the host to serve each
directory's index file and use the exported `404.html` for missing pages.
Client-loaded stylesheets and JavaScript chunks keep the deployment base when
navigation changes the URL depth, including recovery from a missing page.

## Base Paths

Most hosts serve docs at the root. Use a base path when the generated site is
published under a subpath such as `/my-repo`.

Base path priority is:

1. `FOLIOH_BASE_PATH` environment variable.
2. `deploy.base_path` in `docs.yaml`.
3. GitHub Pages inference when `deploy.provider: "github-pages"` or
   `FOLIOH_DEPLOY_PROVIDER=github-pages` is active in GitHub Actions.
4. No base path.

`project.url` is metadata only. It feeds sitemap and canonical URLs, but it does
not control local routing or static asset paths.

## Choose a Strategy

- Use [Static Hosts](/docs/deployment/static-hosts) for Vercel, Netlify, Docker, Caddy, Nginx,
  or any static file server.
- Use [GitHub Pages](/docs/deployment/github-pages) when the public artifact is a GitHub Pages
  site and you need Pages-specific base path behavior.
- Use [CI/CD](/docs/deployment/ci-cd) when deployment should happen automatically on pushes,
  releases, pull requests, or scheduled workflows.
