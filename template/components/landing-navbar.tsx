"use client"

import { useEffect, useState, type ReactNode } from "react"
import { Moon02Icon, Sun03Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import { useTheme } from "next-themes"

import { ThemeGallery } from "@/components/theme-gallery"
import {
  GitHubMark,
  isGitHubHref,
  normalizeLandingHref,
} from "@/components/landing/actions"
import { switchScheme } from "@/lib/scheme-transition"

const projectName = __PROJECT_NAME_JSON__
const projectMonogram = __PROJECT_MONOGRAM_JSON__
/* Root-relative URL of `theme.logo`, or null: the monogram is the fallback. */
const projectLogo: string | null = __PROJECT_LOGO_JSON__
const secondaryCtaText = __LANDING_CTA_SECONDARY_TEXT_JSON__
const secondaryCtaLink: string | null = __LANDING_CTA_SECONDARY_LINK_JSON__
/* The navbar always points at the docs — the hero owns the configured CTA. */

interface LandingNavbarProps {
  /** Relative path from the current page back to the site root — "." on the
   * landing itself, ".." on single-segment public views. Keeps every href
   * relative so exports stay portable (file://, GitHub Pages subpaths). */
  pathToRoot?: string
  /** Retained for view-call compatibility; landing headers are full width. */
  workspace?: boolean
  /** Keep only Folioh identity and the theme control on product-index pages. */
  minimal?: boolean
  /** Sites that ship more than one product hang a switcher off the wordmark,
   * so a reader can cross from one product's landing to the next without
   * going back through the cover. Empty for the single-product case, which
   * is every site but this one. */
  productSwitcher?: ReactNode
}

function ThemeToggle() {
  const { resolvedTheme, setTheme } = useTheme()
  const [mounted, setMounted] = useState(false)
  useEffect(() => {
    const frame = requestAnimationFrame(() => setMounted(true))
    return () => cancelAnimationFrame(frame)
  }, [])
  const isDark = resolvedTheme === "dark"
  return (
    <button
      type="button"
      onClick={(event) =>
        switchScheme(setTheme, isDark ? "light" : "dark", event.currentTarget)
      }
      aria-label="Toggle theme"
      title="Toggle theme"
      data-theme-toggle
      className="p-2 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
    >
      {mounted ? (
        <HugeiconsIcon
          icon={isDark ? Sun03Icon : Moon02Icon}
          size={16}
          strokeWidth={1.8}
          aria-hidden="true"
        />
      ) : (
        <span className="block size-4" aria-hidden="true" />
      )}
    </button>
  )
}

export function LandingNavbar({
  pathToRoot = ".",
  minimal = false,
  productSwitcher = null,
}: LandingNavbarProps) {
  // Resolved here rather than at module scope: a configured "/path" CTA has to
  // be relative to the page carrying the bar, and only the component knows how
  // deep that page sits.
  const normalizedSecondaryCtaLink = secondaryCtaLink
    ? normalizeLandingHref(secondaryCtaLink, pathToRoot)
    : null
  const secondaryCtaIsExternal =
    normalizedSecondaryCtaLink?.startsWith("http") ?? false
  // The bar takes the docs navbar's height, which a theme may change
  // (Omarchy's is 52px), and holds its border inside it, as Nextra's does.
  return (
    <header className="landing-navbar fixed top-0 z-50 box-border h-[var(--nextra-navbar-height,4rem)] w-full border-b border-border bg-background">
      <div className="flex h-full w-full items-center justify-between px-6">
        <div className="flex min-w-0 items-center gap-3">
          <a href={`${pathToRoot}/`} className="flex items-center gap-2.5">
            {projectLogo ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img
                src={normalizeLandingHref(projectLogo, pathToRoot)}
                alt=""
                className="h-7 w-auto"
              />
            ) : (
              <span className="folioh-monogram flex size-7 items-center justify-center bg-primary font-mono text-[11px] font-bold text-primary-foreground">
                {projectMonogram}
              </span>
            )}
            <span className="text-sm font-semibold text-foreground">
              {projectName}
            </span>
          </a>
          {productSwitcher}
        </div>
        <nav className="flex items-center gap-1">
          {!minimal ? (
            <a
              href={`${pathToRoot}/docs/`}
              className="px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
            >
              Docs
            </a>
          ) : null}
          {!minimal && normalizedSecondaryCtaLink ? (
            isGitHubHref(normalizedSecondaryCtaLink) ? (
              <a
                href={normalizedSecondaryCtaLink}
                target="_blank"
                rel="noopener noreferrer"
                aria-label={secondaryCtaText}
                title={secondaryCtaText}
                className="p-2 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
              >
                <GitHubMark />
              </a>
            ) : (
              <a
                href={normalizedSecondaryCtaLink}
                target={secondaryCtaIsExternal ? "_blank" : undefined}
                rel={secondaryCtaIsExternal ? "noopener noreferrer" : undefined}
                className="px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
              >
                {secondaryCtaText}
              </a>
            )
          ) : null}
          <ThemeGallery />
          <ThemeToggle />
        </nav>
      </div>
    </header>
  )
}
