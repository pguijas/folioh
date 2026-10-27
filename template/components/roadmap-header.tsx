import type { ReactNode } from "react"

import { cn } from "@/lib/utils"

/* The stateless roadmap masthead sits directly on the page. Project cards
 * own their selection and release numerals; the header supplies the title
 * and links in the site's sans typography.
 */

interface RoadmapHeaderProps {
  /** The page's name. Drawn once, as the h1, and nowhere else. */
  title?: string
  description?: string
  /** Sibling public views, as quiet text links. */
  links?: { label: string; href: string }[]
  /** The page body: the project cards. */
  children?: ReactNode
}

export const FOCUS_RING =
  "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"

/**
 * The default docs key gets a page mark. Any other project key from docs.yaml
 * gets the same neutral dot, independent of its name or label.
 */
export function ProjectIcon({
  projectKey,
  className,
}: {
  projectKey: string
  className?: string
}) {
  const common = {
    viewBox: "0 0 16 16",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.5,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true,
    className: cn("size-4 shrink-0", className),
  }

  if (projectKey === "docs") {
    return (
      <svg {...common}>
        <path d="M4 2h5l3 3v9H4z" />
        <path d="M9 2v3h3" />
      </svg>
    )
  }
  return (
    <svg {...common}>
      <circle cx="8" cy="8" r="3.25" />
    </svg>
  )
}

export function RoadmapHeader({
  title,
  description,
  links,
  children,
}: RoadmapHeaderProps) {
  return (
    <div className="not-prose">
      <header className="pt-4 pb-2">
        <div className="flex flex-wrap items-start justify-between gap-x-8 gap-y-5">
          {title ? (
            <h1 className="m-0 max-w-3xl text-4xl leading-[1.05] font-bold tracking-[-0.03em] text-balance text-foreground sm:text-5xl">
              {title}
            </h1>
          ) : null}
          {/* Same border, radius and hover as the project cards below, so the
              header reads as one family rather than a title with a stray link
              beside it. It leaves the page, so it keeps an arrow. */}
          {links?.length ? (
            <nav className="flex flex-wrap items-center gap-2 lg:mt-2">
              {links.map((link) => (
                <a
                  key={link.href}
                  href={link.href}
                  className={cn(
                    "group inline-flex items-center gap-2 rounded-lg border border-border px-3.5 py-2 text-sm font-medium text-muted-foreground transition-colors",
                    "hover:border-foreground/30 hover:bg-card hover:text-foreground",
                    FOCUS_RING
                  )}
                >
                  {link.label}
                  <svg
                    viewBox="0 0 16 16"
                    fill="none"
                    aria-hidden="true"
                    className="size-3.5 transition-transform duration-150 group-hover:translate-x-0.5 motion-reduce:transition-none motion-reduce:group-hover:translate-x-0"
                  >
                    <path
                      d="M3 8h9M8.5 4.5 12 8l-3.5 3.5"
                      stroke="currentColor"
                      strokeWidth="1.5"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                </a>
              ))}
            </nav>
          ) : null}
        </div>

        {description ? (
          <p className="mt-5 max-w-2xl text-lg leading-8 text-pretty text-muted-foreground">
            {description}
          </p>
        ) : null}
      </header>

      <div className="mt-7">{children}</div>
    </div>
  )
}
