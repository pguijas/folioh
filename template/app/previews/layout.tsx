import { Footer, Layout, Navbar } from "nextra-theme-docs"
import { getPageMap } from "nextra/page-map"
// __PROJECT_REPO_IMPORTS_START__
import { GithubIcon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
// __PROJECT_REPO_IMPORTS_END__
import { MobileNavDrawer } from "@/components/mobile-nav-drawer"
import { SidebarIndexLinks } from "@/components/sidebar-index-links"
import { ThemeGallery } from "@/components/theme-gallery"
import { VersionSelector } from "@/components/version-selector"

export default async function PreviewsLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <Layout
      navbar={
        <Navbar
          logo={
            <span className="flex items-center gap-2.5">
              <span className="folioh-monogram flex size-7 items-center justify-center rounded-md bg-primary font-mono text-[11px] font-bold text-primary-foreground">
                __PROJECT_MONOGRAM__
              </span>
              <span className="text-sm font-semibold tracking-tight">
                __PROJECT_NAME__
              </span>
            </span>
          }
        >
          {/* __PROJECT_REPO_LINK_START__ */}
          <a
            href="__PROJECT_REPO__"
            target="_blank"
            rel="noreferrer"
            aria-label="GitHub repository"
            title="GitHub repository"
            className="inline-flex size-8 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          >
            <HugeiconsIcon
              icon={GithubIcon}
              size={16}
              strokeWidth={1.8}
              aria-hidden="true"
            />
          </a>
          {/* __PROJECT_REPO_LINK_END__ */}
          <VersionSelector />
          <ThemeGallery />
        </Navbar>
      }
      darkMode={false}
      feedback={{ content: null }}
      pageMap={await getPageMap("/docs")}
      footer={<Footer />}
    >
      <SidebarIndexLinks />
      <MobileNavDrawer />
      <div id="main-content">{children}</div>
    </Layout>
  )
}
