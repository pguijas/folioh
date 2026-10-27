// JS-side mirror of the feature gates: these entries must match
// DISABLED_DOC_ROUTES in crates/folioh-docs/src/features.rs
// (parity is enforced by folioh-site's template.rs test
// `the_bundled_template_carries_the_injection_points`).
// Configuration gates do not reserve names in a project's documentation.
const DISABLED_DOC_STATIC_PATHS = []

const DISABLED_DOC_STATIC_PATH_KEYS = new Set(
  DISABLED_DOC_STATIC_PATHS.map((path) => path.join("/"))
)

const API_REFERENCE_SEGMENT = "api-reference"

function isApiReferencePath(mdxPath) {
  return mdxPath[0] === API_REFERENCE_SEGMENT
}

function normalizeDocsPathSegments(mdxPath) {
  if (isApiReferencePath(mdxPath)) {
    return mdxPath
  }
  return mdxPath.map((segment) =>
    typeof segment === "string" ? segment.replaceAll("_", "-") : segment
  )
}

function underscoreDocsPathAlias(mdxPath) {
  if (isApiReferencePath(mdxPath)) {
    return null
  }

  const alias = mdxPath.map((segment) =>
    typeof segment === "string" ? segment.replaceAll("-", "_") : segment
  )
  return alias.join("/") === mdxPath.join("/") ? null : alias
}

// Whether a requested path is the underscore alias of a docs page
// (/docs/code_group/ for /docs/code-group/); API reference paths keep their
// underscores, so they are never aliases.
export function isUnderscoreAlias(mdxPath) {
  if (!Array.isArray(mdxPath) || isApiReferencePath(mdxPath)) {
    return false
  }
  return mdxPath.some(
    (segment) => typeof segment === "string" && segment.includes("_")
  )
}

function sourceMdxPath(mdxPath) {
  if (!Array.isArray(mdxPath)) {
    return []
  }

  if (mdxPath.length === 1 && mdxPath[0] === "") {
    return []
  }

  let normalized = mdxPath
  if (mdxPath.at(-1) === "index.html") {
    normalized = mdxPath.slice(0, -1)
  }

  return normalized
}

export function normalizeMdxPath(mdxPath) {
  return normalizeDocsPathSegments(sourceMdxPath(mdxPath))
}

// URLs use hyphens, but Nextra imports by the actual content route. Keep
// that route intact, including mixed underscore/hyphen paths and API names.
function contentRoutes(params) {
  const routes = new Map()
  for (const param of params) {
    const source = sourceMdxPath(param.mdxPath)
    const key = normalizeMdxPath(source).join("/")
    const previous = routes.get(key)
    if (previous && previous.join("/") !== source.join("/")) {
      throw new Error(
        `Conflicting docs routes "${previous.join("/")}" and "${source.join("/")}" normalize to "${key}". Rename one of the pages.`
      )
    }
    routes.set(key, source)
  }
  return routes
}

export function resolveMdxPath(mdxPath, params) {
  const normalized = normalizeMdxPath(mdxPath)
  return contentRoutes(params).get(normalized.join("/")) ?? normalized
}

export function isDisabledMdxPath(mdxPath) {
  return DISABLED_DOC_STATIC_PATH_KEYS.has(normalizeMdxPath(mdxPath).join("/"))
}

export function normalizeStaticParam(param) {
  return { ...param, mdxPath: normalizeMdxPath(param.mdxPath) }
}

function staticParamKey(param) {
  return JSON.stringify(
    Object.keys(param)
      .sort()
      .map((key) => [key, param[key]])
  )
}

export function expandStaticParams(params, options = {}) {
  // Refuse ambiguous source pages before aliases can hide one of them.
  contentRoutes(params)
  const includeIndexHtmlAliases =
    options.includeIndexHtmlAliases ?? process.env.NODE_ENV === "development"
  const includeDisabledParams =
    options.includeDisabledParams ?? process.env.NODE_ENV === "development"
  const expanded = []
  const seen = new Set()

  function push(param) {
    const key = staticParamKey(param)
    if (seen.has(key)) {
      return
    }
    seen.add(key)
    expanded.push(param)
  }

  for (const param of params) {
    const normalized = normalizeStaticParam(param)
    push(normalized)

    const underscoreAlias = underscoreDocsPathAlias(normalized.mdxPath)
    const aliasParam = underscoreAlias
      ? { ...normalized, mdxPath: underscoreAlias }
      : null
    if (aliasParam) {
      push(aliasParam)
    }

    if (includeIndexHtmlAliases) {
      push({
        ...normalized,
        mdxPath: [...normalized.mdxPath, "index.html"],
      })
      if (aliasParam) {
        push({
          ...aliasParam,
          mdxPath: [...aliasParam.mdxPath, "index.html"],
        })
      }
    }

    // Source links and the sidebar may retain a mixed spelling which is
    // neither the canonical path nor its all-underscore alias.
    const sourceParam = { ...param, mdxPath: sourceMdxPath(param.mdxPath) }
    push(sourceParam)
    if (includeIndexHtmlAliases) {
      push({ ...sourceParam, mdxPath: [...sourceParam.mdxPath, "index.html"] })
    }
  }

  if (includeDisabledParams) {
    for (const mdxPath of DISABLED_DOC_STATIC_PATHS) {
      push({ mdxPath })

      if (includeIndexHtmlAliases) {
        push({ mdxPath: [...mdxPath, "index.html"] })
      }
    }
  }

  return expanded
}
