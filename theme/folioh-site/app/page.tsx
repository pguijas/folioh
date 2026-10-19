"use client";

import "./product-navigation.css";

import { normalizeLandingHref } from "@/components/landing/actions";
import {
  BuildPipelineLandingHero,
  DocsMapLandingHero,
  HeartbeatLandingHero,
  SourcePipelineLandingHero,
} from "@/components/landing/hero";
import { FolioProductSwitcher } from "@/components/folio-product-switcher";
import { LandingNavbar } from "@/components/landing-navbar";
import {
  LandingSectionRenderer,
  SectionHeading,
} from "@/components/landing/sections";
import { ProductRoadmap } from "@/components/product-roadmap";
import type {
  LandingCatalogItem,
  LandingHeroVariant,
  LandingLink,
  LandingPipelineStep,
  LandingRouteItem,
  LandingSection,
} from "@/components/landing/types";

const installCommands: string[] = __LANDING_INSTALL_COMMANDS__;
const configuredSections = __LANDING_SECTIONS__ as LandingSection[];

const projectName = __PROJECT_NAME_JSON__;
const projectMonogram = __PROJECT_MONOGRAM_JSON__;
const projectVersion = __PROJECT_VERSION_JSON__;
const landingTagline = __LANDING_TAGLINE_JSON__;
const landingNoticeText = __LANDING_NOTICE_TEXT_JSON__;
const landingNoticeLink = __LANDING_NOTICE_LINK_JSON__;
const landingHeadline = __LANDING_HEADLINE_JSON__;
const landingDescription = __LANDING_DESCRIPTION_JSON__;
const primaryCtaText = __LANDING_CTA_PRIMARY_TEXT_JSON__;
const primaryCtaLink = __LANDING_CTA_PRIMARY_LINK_JSON__;
const secondaryCtaText = __LANDING_CTA_SECONDARY_TEXT_JSON__;
const secondaryCtaLink: string | null = __LANDING_CTA_SECONDARY_LINK_JSON__;
const landingHeroVariant = __LANDING_HERO_VARIANT_JSON__ as LandingHeroVariant;

/* The landing is the site root, so a root-relative href in the config needs
   no prefix; normalizeLandingHref still runs every one of them through
   pathToRoot so the template behaves the same wherever a project mounts it. */
const pathToRoot = ".";

function fromProductRoute(href: string) {
  return normalizeLandingHref(href, pathToRoot);
}
function normalizeCatalogItem(item: LandingCatalogItem): LandingCatalogItem {
  return item.href ? { ...item, href: fromProductRoute(item.href) } : item;
}

function normalizeRouteItem(item: LandingRouteItem): LandingRouteItem {
  return { ...item, href: fromProductRoute(item.href) };
}

const landingSections = configuredSections.map(
  (section): LandingSection => ({
    ...section,
    actions: section.actions?.map((action) => ({
      ...action,
      href: fromProductRoute(action.href),
    })),
    items: section.items?.map(normalizeCatalogItem),
    links: section.links?.map(normalizeCatalogItem),
    routes: section.routes?.map(normalizeRouteItem),
  }),
);

const actionLinks: LandingLink[] = [
  {
    href: fromProductRoute(primaryCtaLink),
    title: primaryCtaText,
    detail: "Start with the guide",
    primary: true,
  },
  ...(secondaryCtaLink
    ? [
        {
          href: fromProductRoute(secondaryCtaLink),
          title: secondaryCtaText,
          detail: "Open the source",
          external: secondaryCtaLink.startsWith("http"),
        },
      ]
    : []),
];

const actionGridClassName = [
  "landing-action-grid mt-8 grid w-full max-w-2xl gap-px overflow-hidden border border-border bg-border sm:mt-10",
  actionLinks.length > 1 ? "sm:grid-cols-2" : "sm:grid-cols-1",
].join(" ");

const buildSteps: LandingPipelineStep[] = [
  {
    label: "01",
    title: "Parse source",
    detail:
      "Python modules, docstrings, Markdown, and docs.yaml become one content graph.",
  },
  {
    label: "02",
    title: "Compose docs",
    detail:
      "Reference pages, sidebar data, MDX components, and search stay in sync.",
  },
  {
    label: "03",
    title: "Export static",
    detail:
      "The output is a deployable static site with Pagefind search and LLM files included.",
  },
];

const LandingHero =
  landingHeroVariant === "heartbeat"
    ? HeartbeatLandingHero
    : landingHeroVariant === "build-pipeline"
      ? BuildPipelineLandingHero
      : landingHeroVariant === "source-pipeline"
        ? SourcePipelineLandingHero
        : DocsMapLandingHero;

export default function FoliohDocsHome() {
  return (
    <div className="folio-product-site landing-shell flex min-h-screen flex-col overflow-hidden">
      <LandingNavbar
        pathToRoot={pathToRoot}
        productSwitcher={<FolioProductSwitcher homeHref={`${pathToRoot}/`} />}
      />

      <main id="main-content" className="flex-1">
        <LandingHero
          tagline={landingTagline}
          headline={landingHeadline}
          description={landingDescription}
          actionLinks={actionLinks}
          actionGridClassName={actionGridClassName}
          installCommands={installCommands}
          buildSteps={buildSteps}
          projectName={projectName}
          projectMonogram={projectMonogram}
          projectVersion={projectVersion}
          noticeText={landingNoticeText}
          noticeLink={landingNoticeLink}
          pathToRoot={pathToRoot}
        />

        <LandingSectionRenderer
          sections={landingSections}
          context={{ actionLinks, buildSteps, installCommands, pathToRoot }}
        />

        <section
          id="roadmap"
          className="landing-section border-b border-border bg-background"
        >
          <div className="mx-auto max-w-site px-6 py-20">
            <SectionHeading
              eyebrow="We run on it"
              title="The real roadmap, from the same build as this page."
              description="Not a screenshot: the roadmap plugin rendering YAML from this repository."
              centered
            />
            <div className="mt-10">
              <ProductRoadmap project="docs" />
            </div>
          </div>
        </section>
      </main>

      <footer className="border-t border-border bg-muted/20">
        <div className="mx-auto flex max-w-site flex-col items-start justify-between gap-8 px-6 py-10 sm:flex-row sm:items-center">
          <div className="flex items-center gap-3">
            <span
              className="folioh-monogram grid size-9 place-items-center border border-border bg-card font-mono text-[11px] font-bold text-primary"
              aria-hidden="true"
            >
              {projectMonogram}
            </span>
            <p className="leading-none">
              <span className="block text-sm font-semibold text-foreground">
                Made with Folioh
              </span>
              <span className="mt-1.5 block font-mono text-[10px] text-muted-foreground uppercase">
                docs from source
              </span>
            </p>
          </div>
          <nav className="flex gap-6" aria-label="Folioh Docs links">
            {actionLinks.map((link) => (
              <a
                key={link.title}
                href={link.href}
                target={link.external ? "_blank" : undefined}
                rel={link.external ? "noopener noreferrer" : undefined}
                className="text-sm text-muted-foreground transition-colors hover:text-foreground"
              >
                {link.title}
              </a>
            ))}
          </nav>
        </div>
      </footer>
    </div>
  );
}
