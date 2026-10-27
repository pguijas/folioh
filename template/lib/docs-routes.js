import routeData from "./folioh-routes.json" with { type: "json" }

/** The same canonical docs URLs published in the authoring contract.
 * @param {string} route
 * @param {{ docsRouteBase: string, routes: string[] }} data
 */
export function docsRoute(route = "", data = routeData) {
  const base = data.docsRouteBase.replace(/\/+$/, "") || "/docs"
  const target = route.replace(/^\/+|\/+$/g, "").replace(/\/index$/, "")
  return !target || target === "index" ? `${base}/` : `${base}/${target}/`
}

/** @param {string} route
 * @param {{ docsRouteBase: string, routes: string[] }} data
 */
export function hasDocsRoute(route, data = routeData) {
  return data.routes.includes(docsRoute(route, data))
}

/** @param {{ docsRouteBase: string, routes: string[], apiReference: boolean }} data */
export function defaultDocsRoutes(data = routeData) {
  const base = docsRoute("", data).replace(/\/$/, "")
  return [
    { label: "Guide", href: docsRoute("", data), path: base, detail: "Markdown pages" },
    ...(data.apiReference && hasDocsRoute("api-reference", data) ? [
      { label: "API", href: docsRoute("api-reference", data), path: `${base}/api-reference`, detail: "From source" },
    ] : []),
    { label: "Search", href: `${docsRoute("", data)}?q=`, path: "/_pagefind", detail: "Static index" },
  ]
}
