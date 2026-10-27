"use client"

import type { ComponentProps } from "react"
import { BrowserFrame } from "@/components/browser-frame"
import { FOLIOH_BANNER, ThemeWordmark } from "@/components/theme-artwork"
import { PastelArtwork } from "@/components/pastel-artwork"
import {
  GitHubMark,
  isGitHubHref,
  LandingActions,
  LandingCommand,
  normalizeLandingHref,
} from "@/components/landing/actions"
import { defaultRoutes } from "@/components/landing/defaults"
import type { LandingLink, LandingPipelineStep } from "@/components/landing/types"
import { cn } from "@/lib/utils"

function LandingHeroVisual({ embedded = false, ...props }: ComponentProps<"aside"> & { embedded?: boolean }) {
  if (embedded) return <aside {...props} />
  return (
    <div className="landing-hero-visual">
      <PastelArtwork seed="landing" />
      <aside {...props} />
    </div>
  )
}

type LandingHeroProps = {
  tagline: string
  headline: string
  description: string
  actionLinks: LandingLink[]
  actionGridClassName: string
  installCommands: string[]
  buildSteps: LandingPipelineStep[]
  /* The heartbeat hero's docs-window mock renders the real project —
     name, monogram, version, and its actual install command — so the
     demo is an artifact, not a fiction. Optional for other variants. */
  projectName?: string
  projectMonogram?: string
  projectVersion?: string
  /* Optional announcement chip above the kicker: one plain configured
     message ("landing.hero.notice" in docs.yaml), nothing derived. */
  noticeText?: string
  noticeLink?: string
  /* Reuse the heartbeat window inside a product section without rendering a
     second hero, headline, or CTA block. */
  embedded?: boolean
  /* Relative path from the page back to the site root — "." on the landing
     itself, ".." for a landing served one route below it. The notice chip is
     the only href the hero builds from a configured "/path". */
  pathToRoot?: string
}

type LandingHeroCopyProps = Omit<LandingHeroProps, "buildSteps"> & {
  className?: string
}

function LandingHeroCopy({
  tagline,
  headline,
  description,
  actionLinks,
  actionGridClassName,
  installCommands,
  projectName,
  className,
}: LandingHeroCopyProps) {
  const hasTagline = tagline.trim() !== ""
  const copyClassName = [
    "landing-hero-copy max-w-3xl min-w-0",
    className,
  ]
    .filter(Boolean)
    .join(" ")

  return (
    <div className={copyClassName}>
      <ThemeWordmark name={projectName || "Docs"} />
      {hasTagline ? (
        <p className="landing-kicker font-mono text-[11px] tracking-[0.14em] text-primary uppercase">
          {tagline}
        </p>
      ) : null}
      <h1
        className={cn(
          "text-[2.35rem] leading-[0.94] font-extrabold text-foreground sm:text-6xl xl:text-[4.55rem]",
          hasTagline ? "mt-5" : "m-0"
        )}
      >
        {headline}
      </h1>
      <p className="mt-7 max-w-2xl text-lg leading-8 text-muted-foreground sm:text-xl">
        {description}
      </p>
      <LandingActions
        actionLinks={actionLinks}
        actionGridClassName={actionGridClassName}
      />
      <LandingCommand
        installCommands={installCommands}
        className="mt-6 max-w-2xl sm:mt-8"
      />
    </div>
  )
}

export function DocsMapLandingHero({
  tagline,
  headline,
  description,
  actionLinks,
  actionGridClassName,
  installCommands,
  projectName,
  pathToRoot,
}: LandingHeroProps) {
  return (
    <section className="landing-surface border-b border-border">
      <div className="mx-auto grid max-w-site gap-10 px-6 pt-24 pb-16 sm:pt-28 lg:min-h-[700px] lg:grid-cols-[minmax(0,0.9fr)_minmax(340px,0.72fr)] lg:items-end xl:gap-16">
        <LandingHeroCopy
          tagline={tagline}
          headline={headline}
          description={description}
          actionLinks={actionLinks}
          actionGridClassName={actionGridClassName}
          installCommands={installCommands}
          projectName={projectName}
          className="pb-4 lg:pb-16"
        />

        <LandingHeroVisual
          className="border border-border bg-card"
          aria-label="Documentation routes"
        >
          <div className="flex items-center justify-between border-b border-border px-5 py-4">
            <p className="font-mono text-[10px] text-muted-foreground uppercase">
              Documentation routes
            </p>
            <span className="size-2 bg-primary" aria-hidden="true" />
          </div>

          <div className="grid gap-px bg-border">
            {defaultRoutes.map((route, index) => (
              <a
                key={route.path}
                href={normalizeLandingHref(route.href, pathToRoot)}
                className="group grid grid-cols-[4.5rem_1fr] gap-4 bg-card px-5 py-5 transition-colors hover:bg-muted/50"
                style={{ animationDelay: `${220 + index * 80}ms` }}
              >
                <span className="font-mono text-xs font-semibold text-primary">
                  {String(index + 1).padStart(2, "0")}
                </span>
                <span>
                  <span className="block text-base font-semibold text-foreground">
                    {route.label}
                  </span>
                  <span className="mt-2 block font-mono text-[11px] text-muted-foreground">
                    {route.path}
                  </span>
                  <span className="mt-3 block text-sm leading-6 text-muted-foreground">
                    {route.detail}
                  </span>
                </span>
              </a>
            ))}
          </div>

          <div className="grid gap-px bg-border sm:grid-cols-3">
            {["docs.yaml", "Markdown", "Source code"].map((item) => (
              <div key={item} className="bg-card p-5">
                <span className="font-mono text-[10px] text-muted-foreground uppercase">
                  input
                </span>
                <p className="mt-3 text-sm font-semibold text-foreground">
                  {item}
                </p>
              </div>
            ))}
          </div>
        </LandingHeroVisual>
      </div>
    </section>
  )
}

/**
 * "build-pipeline" hero: split layout with headline + CTAs on the left and a
 * live-looking docstring -> `folioh build` -> rendered-reference pipeline on
 * the right. The example content is intentionally generic (a small HTTP
 * client) so any Folioh project can ship this variant unmodified.
 */
export function BuildPipelineLandingHero({
  tagline,
  headline,
  description,
  actionLinks,
  installCommands,
  projectName,
}: LandingHeroProps) {
  const hasTagline = tagline.trim() !== ""
  return (
    <section className="landing-surface border-b border-border">
      <div className="mx-auto grid max-w-site gap-12 px-6 pt-20 pb-16 sm:pt-24 lg:min-h-[680px] lg:grid-cols-[minmax(0,0.94fr)_minmax(380px,1fr)] lg:items-center xl:gap-16">
        <div className="landing-hero-copy min-w-0 max-w-2xl">
          <ThemeWordmark name={projectName || "Docs"} />
          {hasTagline ? (
            <p className="landing-kicker font-mono text-[11px] tracking-[0.14em] text-primary uppercase">
              {tagline}
            </p>
          ) : null}
          <h1
            className={cn(
              "text-4xl leading-[1.02] font-extrabold text-foreground sm:text-5xl xl:text-6xl",
              hasTagline ? "mt-5" : "m-0"
            )}
          >
            {headline}
          </h1>
          <p className="mt-6 max-w-xl text-lg leading-8 text-muted-foreground">
            {description}
          </p>

          <div className="landing-hero-actions mt-8 flex flex-wrap items-center gap-3">
            {actionLinks.map((action) => (
              <a
                key={action.title}
                href={action.href}
                target={action.external ? "_blank" : undefined}
                rel={action.external ? "noopener noreferrer" : undefined}
                className={
                  action.primary
                    ? "inline-flex items-center gap-2 rounded-md bg-primary px-5 py-2.5 text-sm font-semibold text-primary-foreground transition-colors hover:bg-foreground"
                    : "inline-flex items-center gap-2 rounded-md border border-border bg-card px-5 py-2.5 text-sm font-semibold text-foreground transition-colors hover:bg-muted"
                }
              >
                {isGitHubHref(action.href) ? (
                  <GitHubMark className="size-4" />
                ) : null}
                {action.title}
              </a>
            ))}
          </div>

          {installCommands.length > 0 ? (
            <LandingCommand
              installCommands={installCommands}
              className="mt-7 max-w-md rounded-md"
            />
          ) : null}
        </div>

        <LandingHeroVisual
          className="landing-artifact min-w-0 rounded-lg"
          aria-label="Docstrings rendered into an API reference"
        >
          <BrowserFrame url="src/payments/client.py">
            <pre className="m-0 overflow-x-auto font-mono text-xs leading-[1.75] text-foreground">
              <code>
                <span className="text-primary">class</span>{" "}
                <span className="font-semibold">Client</span>:{"\n"}
                <span className="text-muted-foreground">
                  {'    """HTTP client for the Payments API.\n'}
                  {"\n"}
                  {"    Args:\n"}
                  {"        base_url: API origin, e.g. https://api.example.com\n"}
                  {"        timeout: Seconds to wait before giving up.\n"}
                  {'    """\n'}
                </span>
                {"\n"}
                {"    "}
                <span className="text-primary">def</span>{" "}
                <span className="font-semibold">__init__</span>
                (self, base_url: <span className="text-primary">str</span>,
                timeout: <span className="text-primary">float</span> = 30.0):
                ...
              </code>
            </pre>
          </BrowserFrame>

          <div className="flex items-center gap-3 py-2.5 pl-4">
            <span
              aria-hidden="true"
              className="ml-2 h-8 w-px bg-gradient-to-b from-border to-primary/70"
            />
            <span className="rounded-md border border-primary/30 bg-primary/10 px-2.5 py-1 font-mono text-[11px] text-primary">
              folioh build
            </span>
            <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
              docstrings + type hints &rarr; rendered reference
            </span>
          </div>

          <BrowserFrame url="yourdocs.dev/reference/payments/client">
            <div className="min-w-0">
              <p className="m-0 font-mono text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                Reference / payments
              </p>
              <p className="mt-2 mb-0 text-lg font-semibold tracking-tight text-foreground">
                Client
                <span className="ml-2 rounded-md border border-primary/30 bg-primary/10 px-1.5 py-0.5 align-[3px] font-mono text-[10px] font-normal text-primary">
                  class
                </span>
              </p>
              <p className="mt-3 mb-0 overflow-x-auto rounded-md border border-border bg-background px-3.5 py-2.5 font-mono text-xs whitespace-nowrap text-muted-foreground">
                <span className="text-primary">Client</span>
                (base_url: str, timeout: float = 30.0)
              </p>
              <div className="mt-3 divide-y divide-border border-t border-border text-sm">
                <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1 py-2">
                  <code className="font-mono text-xs text-foreground">
                    base_url
                  </code>
                  <span className="font-mono text-[11px] text-primary">str</span>
                  <span className="min-w-0 text-[13px] text-muted-foreground">
                    API origin, e.g. https://api.example.com
                  </span>
                </div>
                <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1 py-2">
                  <code className="font-mono text-xs text-foreground">
                    timeout
                  </code>
                  <span className="font-mono text-[11px] text-primary">
                    float
                  </span>
                  <span className="min-w-0 text-[13px] text-muted-foreground">
                    Seconds to wait before giving up.
                  </span>
                </div>
              </div>
            </div>
          </BrowserFrame>
        </LandingHeroVisual>
      </div>
    </section>
  )
}

/**
 * "heartbeat" hero: one window that is first a terminal and then the docs.
 * `$ folioh serve` types itself, the whole build lands as one receipt stamp,
 * and the terminal surface lifts like a shutter to reveal the served docs
 * home beneath — same window, same size, new identity. The
 * animation is pure CSS on one master cycle (see `landing-hb-*` styles in
 * globals.css); reduced motion rests on the finished page. The docs-window
 * mock is project-driven: it shows the configured project's own name,
 * version, and install command, so every string in the demo is true for
 * whichever site ships it.
 */
export function HeartbeatLandingHero({
  tagline,
  headline,
  description,
  actionLinks,
  installCommands,
  projectName,
  projectMonogram,
  projectVersion,
  noticeText,
  noticeLink,
  embedded = false,
  pathToRoot,
}: LandingHeroProps) {
  const hasTagline = tagline.trim() !== ""
  const mockName = projectName || "Docs"
  const mockSlug =
    mockName.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") ||
    "docs"
  const mockMark = (projectMonogram || mockSlug.slice(0, 2)).slice(0, 2)
  // No install command configured: the row goes, rather than a guess at the
  // project's package manager.
  const mockInstall = installCommands[0]
  const Root = embedded ? "div" : "section"
  return (
    <Root
      className={cn(
        embedded ? "min-w-0" : "landing-surface border-b border-border"
      )}
    >
      {/* In hero mode, the copy column sets the row height and the window
          stretches to match it. Embedded mode renders only the window. */}
      <div
        className={cn(
          embedded
            ? "min-w-0"
            : "mx-auto grid max-w-site gap-12 px-6 pt-20 pb-16 sm:pt-24 lg:grid-cols-[minmax(0,0.94fr)_minmax(380px,1fr)] xl:gap-16"
        )}
      >
        {!embedded ? (
          <div className="landing-hero-copy min-w-0 max-w-2xl">
            {noticeText ? (
              noticeLink ? (
                <a
                  href={normalizeLandingHref(noticeLink, pathToRoot)}
                  className="group mb-5 inline-flex items-center gap-2 rounded-md border border-border bg-card px-2.5 py-1 font-mono text-[11px] text-muted-foreground transition-colors hover:border-foreground/40 hover:text-foreground"
                >
                  {noticeText}
                  <span
                    aria-hidden="true"
                    className="transition-transform group-hover:translate-x-0.5"
                  >
                    &rarr;
                  </span>
                </a>
              ) : (
                <span className="mb-5 inline-flex items-center rounded-md border border-border bg-card px-2.5 py-1 font-mono text-[11px] text-muted-foreground">
                  {noticeText}
                </span>
              )
            ) : null}
            <ThemeWordmark name={mockName} />
            {hasTagline ? (
              <p className="landing-kicker font-mono text-[11px] tracking-[0.14em] text-primary uppercase">
                {tagline}
              </p>
            ) : null}
            <h1
              className={cn(
                "text-4xl leading-[1.02] font-extrabold text-foreground sm:text-5xl xl:text-6xl",
                hasTagline || noticeText ? "mt-5" : "m-0"
              )}
            >
              {headline}
            </h1>
            <p className="mt-6 max-w-xl text-lg leading-8 text-muted-foreground">
              {description}
            </p>

            <div
              className={`landing-hero-actions mt-8 grid w-full max-w-xl gap-3 ${actionLinks.length > 1 ? "grid-cols-2" : "grid-cols-1"}`}
            >
              {actionLinks.map((action) => (
                <a
                  key={action.title}
                  href={action.href}
                  target={action.external ? "_blank" : undefined}
                  rel={action.external ? "noopener noreferrer" : undefined}
                  className={
                    action.primary
                      ? "inline-flex items-center justify-center gap-2 rounded-md bg-primary px-5 py-2.5 text-sm font-semibold text-primary-foreground transition-colors hover:bg-foreground"
                      : "inline-flex items-center justify-center gap-2 rounded-md border border-border bg-card px-5 py-2.5 text-sm font-semibold text-foreground transition-colors hover:bg-muted"
                  }
                >
                  {isGitHubHref(action.href) ? (
                    <GitHubMark className="size-4" />
                  ) : null}
                  {action.title}
                </a>
              ))}
            </div>

            {installCommands.length > 0 ? (
              <LandingCommand
                installCommands={installCommands}
                className="mt-7 max-w-xl rounded-md"
              />
            ) : null}
          </div>
        ) : null}

        <LandingHeroVisual
          embedded={embedded}
          className={cn(
            "landing-artifact landing-heartbeat min-w-0 lg:flex lg:flex-col",
            embedded && "h-full"
          )}
          aria-label="A terminal types folioh serve, stamps the build receipt, then lifts to reveal the served docs home in the same window"
        >
          <div className="flex flex-col overflow-hidden rounded-lg border border-border bg-card lg:flex-1">
            {/* chrome bar: the dots never leave; the terminal title and the
                address pill trade places on the cycle */}
            <div className="flex items-center gap-3 border-b border-border bg-muted/40 px-3.5 py-2.5">
              <span aria-hidden="true" className="flex shrink-0 gap-1.5">
                <span className="size-[9px] rounded-full bg-border" />
                <span className="size-[9px] rounded-full bg-border" />
                <span className="size-[9px] rounded-full bg-border" />
              </span>
              <span className="relative grid min-w-0 flex-1 items-center">
                <span
                  className="landing-hb-titlebar col-start-1 row-start-1 min-w-0 truncate font-mono text-[11px] leading-4 text-muted-foreground"
                  aria-hidden="true"
                >
                  folioh serve &mdash; ~/{mockSlug}
                </span>
                <span className="landing-hb-url col-start-1 row-start-1 truncate font-mono text-[11px] leading-4 text-muted-foreground">
                  localhost:4321
                </span>
              </span>
            </div>

            <div className="relative overflow-hidden lg:flex-1">
              {/* the served page — the rest state and the reveal payoff */}
              <div className="flex min-w-0 flex-col lg:h-full">
                <div className="flex shrink-0 items-center justify-between gap-4 border-b border-border px-4 py-2.5">
                  <span className="flex items-center gap-2 text-xs font-semibold text-foreground">
                    <span
                      className="folioh-monogram grid size-[18px] place-items-center rounded-[5px] bg-primary font-mono text-[8px] font-semibold text-primary-foreground"
                      aria-hidden="true"
                    >
                      {mockMark}
                    </span>
                    {mockName}
                  </span>
                  <span className="landing-hb-glint inline-flex items-center gap-5 rounded-md border border-border bg-background px-2.5 py-0.5 font-mono text-[10px] leading-relaxed text-muted-foreground">
                    Search
                    <kbd className="rounded border border-border bg-card px-1 font-mono text-[9px]">
                      &#8984;K
                    </kbd>
                  </span>
                </div>

                <div className="grid grid-cols-[minmax(0,1fr)] sm:grid-cols-[8.5rem_minmax(0,1fr)] lg:flex-1 xl:grid-cols-[8.5rem_minmax(0,1fr)_7.5rem]">
                  <nav
                    className="hidden border-r border-border bg-muted/30 px-3 py-4 text-xs sm:block"
                    aria-hidden="true"
                  >
                    <p className="m-0 font-mono text-[9px] tracking-[0.08em] text-muted-foreground uppercase">
                      Guides
                    </p>
                    <p className="mt-1.5 mb-0 rounded-[5px] bg-primary/10 px-2 py-1 font-semibold text-primary">
                      Introduction
                    </p>
                    <p className="mt-0.5 mb-0 rounded-[5px] px-2 py-1 text-muted-foreground">
                      Quick Start
                    </p>
                    <p className="mt-3 mb-0 font-mono text-[9px] tracking-[0.08em] text-muted-foreground uppercase">
                      Reference
                    </p>
                    <p className="mt-1.5 mb-0 px-2 py-1 text-muted-foreground">
                      {mockSlug}
                    </p>
                  </nav>

                  <div className="min-w-0 px-4 py-4 sm:px-5">
                    <p className="m-0 font-mono text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                      Docs
                    </p>
                    <p className="mt-2 mb-0 text-xl font-semibold tracking-tight text-foreground">
                      {mockName}
                      {projectVersion ? (
                        <span className="ml-2 rounded-md border border-primary/30 bg-primary/10 px-1.5 py-0.5 align-[3px] font-mono text-[10px] font-normal text-primary">
                          v{projectVersion.replace(/^v/i, "")}
                        </span>
                      ) : null}
                    </p>
                    <p className="mt-1.5 mb-0 text-[13px] italic leading-6 text-muted-foreground">
                      Docstrings, signatures, and guides — this page is
                      generated from the source on every build.
                    </p>
                    {mockInstall && (
                      <p className="mt-3 mb-0 overflow-x-auto rounded-md border border-border bg-background px-3.5 py-2.5 font-mono text-xs whitespace-nowrap text-muted-foreground">
                        <span className="text-primary">$</span> {mockInstall}
                      </p>
                    )}
                    <p className="mt-4 mb-0 font-mono text-[10px] uppercase tracking-[0.14em] text-foreground">
                      Start here
                    </p>
                    <div className="mt-2 grid gap-2 sm:grid-cols-2">
                      <span className="rounded-md border border-border bg-background px-3 py-2.5">
                        <span className="block text-[13px] font-semibold text-foreground">
                          Quick Start &rarr;
                        </span>
                        <span className="mt-1 block text-xs leading-5 text-muted-foreground">
                          Point Folioh at your module and serve.
                        </span>
                      </span>
                      <span className="rounded-md border border-border bg-background px-3 py-2.5">
                        <span className="block text-[13px] font-semibold text-foreground">
                          API Reference &rarr;
                        </span>
                        <span className="mt-1 block text-xs leading-5 text-muted-foreground">
                          Every public symbol — signatures, parameters, returns.
                        </span>
                      </span>
                    </div>
                  </div>

                  <nav
                    className="hidden border-l border-border px-3 py-4 text-[11px] xl:block"
                    aria-hidden="true"
                  >
                    <p className="m-0 font-mono text-[9px] tracking-[0.08em] text-muted-foreground uppercase">
                      On this page
                    </p>
                    <p className="mt-2 mb-0 font-semibold text-primary">{mockName}</p>
                    <p className="mt-1 mb-0 ml-2.5 text-muted-foreground">Install</p>
                    <p className="mt-1 mb-0 ml-2.5 text-muted-foreground">Start here</p>
                  </nav>
                </div>
              </div>

              {/* the terminal shutter: covers the page at the seam, types the
                  command, stamps the build receipt, then lifts up and out. The
                  transcript reproduces folioh's real CLI output for a project
                  without examples (banner, 12-char label column, step order
                  and wording), minus the version the CLI prints beside the
                  banner and the elapsed time it prints on the Done line, which
                  depends on the machine. */}
              <div
                className="landing-hb-shutter absolute inset-0 z-10 flex flex-col px-5 py-4 font-mono text-xs leading-[1.95]"
                aria-hidden="true"
              >
                <span className="whitespace-pre">
                  <span className="text-muted-foreground">$ </span>
                  <span className="landing-hb-cmd font-semibold text-foreground">
                    folioh serve
                  </span>
                  <span className="landing-hb-caret" />
                </span>
                <pre className="landing-hb-stamp m-0 mt-2 self-center text-[10px] text-primary">
                  {FOLIOH_BANNER}
                </pre>
                <span className="landing-hb-line landing-hb-s1 mt-2 whitespace-pre">
                  <span className="font-semibold text-primary">&#10003;</span>
                  {" "}
                  <span className="font-semibold text-foreground">{"Sources     "}</span>
                  {" "}
                  <span className="text-muted-foreground">&#8250; 1 module, 2 doc pages</span>
                </span>
                <span className="landing-hb-line landing-hb-s2 whitespace-pre">
                  <span className="font-semibold text-primary">&#10003;</span>
                  {" "}
                  <span className="font-semibold text-foreground">{"Template    "}</span>
                  {" "}
                  <span className="text-muted-foreground">&#8250; .build/ workspace ready</span>
                </span>
                <span className="landing-hb-line landing-hb-s3 whitespace-pre">
                  <span className="font-semibold text-primary">&#10003;</span>
                  {" "}
                  <span className="font-semibold text-foreground">{"Pages       "}</span>
                  {" "}
                  <span className="text-muted-foreground">&#8250; 3 pages</span>
                </span>
                <span className="landing-hb-line landing-hb-s4 whitespace-pre">
                  <span className="font-semibold text-primary">&#10003;</span>
                  {" "}
                  <span className="font-semibold text-foreground">{"Links       "}</span>
                  {" "}
                  <span className="text-muted-foreground">&#8250; valid</span>
                </span>
                <span className="landing-hb-line landing-hb-s5 whitespace-pre-wrap">
                  <span className="font-semibold text-primary">&#10003;</span>
                  {" "}
                  <span className="font-semibold text-foreground">{"Dependencies"}</span>
                  {" "}
                  <span className="text-muted-foreground">&#8250; up to date, the lockfile has not moved</span>
                </span>
                <span className="landing-hb-line landing-hb-s6 whitespace-pre">
                  <span className="font-semibold text-primary">&#10003;</span>
                  {" "}
                  <span className="font-semibold text-primary">{"Done        "}</span>
                  {" "}
                  <span className="text-muted-foreground">&#8250; 3 pages, ready</span>
                </span>
                <span className="landing-hb-line landing-hb-s7 whitespace-pre font-semibold text-foreground">
                  {"  "}Starting dev server...
                </span>
              </div>
            </div>
          </div>
        </LandingHeroVisual>
      </div>
    </Root>
  )
}

export function SourcePipelineLandingHero({
  tagline,
  headline,
  description,
  actionLinks,
  actionGridClassName,
  installCommands,
  buildSteps,
  projectName,
}: LandingHeroProps) {
  return (
    <section className="landing-surface border-b border-border">
      <div className="mx-auto grid max-w-site gap-12 px-6 pt-24 pb-20 sm:pt-28 lg:min-h-[760px] lg:grid-cols-[minmax(0,1fr)_minmax(360px,0.78fr)] lg:items-center xl:gap-16">
        <LandingHeroCopy
          tagline={tagline}
          headline={headline}
          description={description}
          actionLinks={actionLinks}
          actionGridClassName={actionGridClassName}
          installCommands={installCommands}
          projectName={projectName}
        />

        <LandingHeroVisual
          className="landing-artifact border border-border bg-card"
          aria-label="Build pipeline overview"
        >
          <div className="landing-artifact-top flex items-center justify-between gap-4 border-b border-border px-5 py-4">
            <div className="flex items-center gap-2 font-mono text-[10px] text-muted-foreground uppercase">
              <span className="size-2 bg-primary" aria-hidden="true" />
              source
              <span className="text-border">/</span>
              site
            </div>
            <span className="font-mono text-[10px] text-muted-foreground uppercase">
              build ready
            </span>
          </div>

          <div className="landing-artifact-stage grid gap-px bg-border md:grid-cols-[0.95fr_1.05fr]">
            <div className="bg-card p-5">
              <p className="font-mono text-[10px] text-muted-foreground uppercase">
                Source code
              </p>
              <div className="mt-5 space-y-2 font-mono text-xs">
                <div className="landing-code-line w-[76%]" />
                <div className="landing-code-line w-[58%]" />
                <div className="landing-code-line w-[88%]" />
                <div className="landing-code-line landing-code-line-muted w-[64%]" />
                <div className="landing-code-line w-[44%]" />
              </div>
            </div>

            <div className="relative overflow-hidden bg-card p-5">
              <div className="landing-scan" aria-hidden="true" />
              <p className="font-mono text-[10px] text-muted-foreground uppercase">
                Generated docs
              </p>
              <div className="mt-5 space-y-3">
                <div className="h-6 w-3/4 bg-foreground" />
                <div className="h-2 w-full bg-muted" />
                <div className="h-2 w-5/6 bg-muted" />
                <div className="grid grid-cols-3 gap-2 pt-2">
                  <span className="h-8 border border-border bg-background" />
                  <span className="h-8 border border-border bg-background" />
                  <span className="h-8 border border-border bg-background" />
                </div>
              </div>
            </div>
          </div>

          <ol className="landing-sequence divide-y divide-border">
            {buildSteps.map((step) => (
              <li
                key={step.label}
                className="grid grid-cols-[3rem_1fr] gap-4 px-5 py-5"
                style={{ animationDelay: `${240 + Number(step.label) * 90}ms` }}
              >
                <span className="font-mono text-xs font-semibold text-primary">
                  {step.label}
                </span>
                <div>
                  <h3 className="text-base font-semibold text-foreground">
                    {step.title}
                  </h3>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    {step.detail}
                  </p>
                </div>
              </li>
            ))}
          </ol>
        </LandingHeroVisual>
      </div>
    </section>
  )
}
