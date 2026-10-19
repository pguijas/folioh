import { siteMetadataUrl } from "@/lib/site-metadata-url"
import { existsSync } from "fs"
import { join } from "path"
import { generateStaticParamsFor, importPage } from "nextra/pages"
import { notFound, permanentRedirect } from "next/navigation"
import {
  expandStaticParams,
  isDisabledMdxPath,
  isUnderscoreAlias,
  normalizeMdxPath,
} from "@/lib/docs-route-params"
import { useMDXComponents as getMDXComponents } from "@/mdx-components"
import { PageActionsButton } from "@/components/page-actions-button"
import { ThemeWordmark } from "@/components/theme-artwork"
import { PastelArtwork } from "@/components/pastel-artwork"

const _nextraParams = generateStaticParamsFor("mdxPath")
const configuredSiteUrl = "__SITE_URL__"
const siteUrl = siteMetadataUrl(configuredSiteUrl)
const projectName = "__PROJECT_NAME__"
const projectDescription = "__PROJECT_DESCRIPTION__"
const docsOgImageUrl = siteUrl
  ? `${siteUrl}/docs/opengraph-image`
  : "/docs/opengraph-image"
const docsIndexCanonicalPath = "__DOCS_INDEX_CANONICAL_PATH__"
const foliohBasePath = process.env.NEXT_PUBLIC_FOLIOH_BASE_PATH?.replace(/\/+$/, "") ?? ""
const markdownMirrorDir = join(process.cwd(), "public", "_folioh", "markdown")
const contentDir = join(process.cwd(), "content")

function docsRouteForMdxPath(mdxPath) {
  if (!mdxPath.length) {
    return docsIndexCanonicalPath === "/" ? "/" : "/docs/"
  }
  return `/docs/${mdxPath.join("/")}/`
}

// A project without a docs index page (an API-only example) has nothing to
// render at the docs root.
function isMissingDocsIndex(mdxPath) {
  return !mdxPath.length && !existsSync(join(contentDir, "index.mdx"))
}

// The docs root of such a project opens its API reference, or is a 404.
function leaveMissingDocsIndex() {
  if (existsSync(join(contentDir, "api-reference", "index.mdx"))) {
    permanentRedirect(docsRouteForMdxPath(["api-reference"]))
  }
  notFound()
}

function absoluteDocsUrl(mdxPath) {
  return siteUrl ? `${siteUrl}${docsRouteForMdxPath(mdxPath)}` : ""
}

// The build writes a Markdown mirror of every page into
// public/_folioh/markdown, named after the content file rather than the docs
// route: content/plugins/index.mdx mirrors to plugins/index.md, not
// plugins.md. Probe both shapes on disk and link nothing when neither is
// there, so the head never points at a file the build did not write.
function markdownMirrorPath(mdxPath) {
  const route = mdxPath.length ? mdxPath.join("/") : "index"
  for (const candidate of [`${route}.md`, `${route}/index.md`]) {
    if (existsSync(join(markdownMirrorDir, candidate))) {
      return `/_folioh/markdown/${candidate}`
    }
  }
  return ""
}

function markdownMirrorUrl(mdxPath) {
  const mirrorPath = markdownMirrorPath(mdxPath)
  if (!mirrorPath) {
    return ""
  }
  // A configured site URL already carries the deploy base path (it is what
  // canonical links are built from); without one, fall back to the base path
  // Next was configured with.
  return siteUrl ? `${siteUrl}${mirrorPath}` : `${foliohBasePath}${mirrorPath}`
}

function pageAlternates(metadata, canonical, markdownUrl) {
  if (!canonical && !markdownUrl) {
    return null
  }

  return {
    ...metadata.alternates,
    ...(canonical ? { canonical } : {}),
    ...(markdownUrl
      ? {
          types: {
            ...metadata.alternates?.types,
            "text/markdown": markdownUrl,
          },
        }
      : {}),
  }
}

// An underscore alias (/docs/code_group/) serves the page of its hyphenated
// route. The canonical names that route; without a site URL there is no
// canonical to give, so the alias is kept out of indexes instead.
function pageMetadata(metadata, mdxPath, isAlias) {
  const canonical = absoluteDocsUrl(mdxPath)
  const alternates = pageAlternates(
    metadata,
    canonical,
    markdownMirrorUrl(mdxPath)
  )
  if (!canonical) {
    const robots = isAlias ? { robots: { index: false, follow: true } } : {}
    return alternates
      ? { ...metadata, ...robots, alternates }
      : { ...metadata, ...robots }
  }

  const title = metadata.title ?? `${projectName} documentation`
  const description = metadata.description ?? projectDescription

  return {
    ...metadata,
    alternates,
    openGraph: {
      ...metadata.openGraph,
      title: metadata.openGraph?.title ?? title,
      description: metadata.openGraph?.description ?? description,
      url: canonical,
      images: metadata.openGraph?.images ?? [
        {
          url: docsOgImageUrl,
          width: 1200,
          height: 630,
          alt: `${title} - ${projectName}`,
        },
      ],
    },
    twitter: {
      ...metadata.twitter,
      card: metadata.twitter?.card ?? "summary_large_image",
      title: metadata.twitter?.title ?? title,
      description: metadata.twitter?.description ?? description,
      images: metadata.twitter?.images ?? [docsOgImageUrl],
    },
  }
}

export async function generateStaticParams() {
  const params = await _nextraParams()
  // Nextra lists only the pages that exist, so a docs root without an index
  // page is added here for its redirect to the API reference to be written.
  if (
    !existsSync(join(contentDir, "index.mdx")) &&
    existsSync(join(contentDir, "api-reference", "index.mdx"))
  ) {
    params.push({ mdxPath: [] })
  }
  return expandStaticParams(params)
}

export async function generateMetadata(props) {
  const params = await props.params
  const mdxPath = normalizeMdxPath(params.mdxPath)
  if (isDisabledMdxPath(mdxPath)) {
    return {
      robots: {
        index: false,
        follow: false,
      },
    }
  }
  if (isMissingDocsIndex(mdxPath)) {
    return { robots: { index: false, follow: true } }
  }
  const { metadata } = await importPage(mdxPath)
  return pageMetadata(metadata, mdxPath, isUnderscoreAlias(params.mdxPath))
}

const Wrapper = getMDXComponents().wrapper

export default async function Page(props) {
  const params = await props.params
  const mdxPath = normalizeMdxPath(params.mdxPath)
  if (isDisabledMdxPath(mdxPath)) {
    notFound()
  }
  if (isMissingDocsIndex(mdxPath)) {
    leaveMissingDocsIndex()
  }
  const normalizedParams = { ...params, mdxPath }
  const { default: MDXContent, toc, metadata } = await importPage(mdxPath)
  // Pagefind indexes only pages whose <main> carries data-pagefind-body,
  // which the wrapper drops for searchable: false; an alias is a duplicate.
  const wrapperMetadata = isUnderscoreAlias(params.mdxPath)
    ? { ...metadata, searchable: false }
    : metadata
  return (
    <Wrapper toc={toc} metadata={wrapperMetadata}>
      <div className="mb-2 flex justify-end" data-pagefind-ignore="all">
        <PageActionsButton markdownPath={markdownMirrorPath(mdxPath)} />
      </div>
      <ThemeWordmark name={projectName} />
      <PastelArtwork seed={mdxPath.join("/") || "folioh"} />
      <MDXContent {...props} params={normalizedParams} />
    </Wrapper>
  )
}
