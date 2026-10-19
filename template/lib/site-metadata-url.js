// Preview examples have their own deploy path under the same preview origin.
export function siteMetadataUrl(configuredUrl) {
  const previewUrl = process.env.FOLIOH_PREVIEW_URL
  const basePath = process.env.NEXT_PUBLIC_FOLIOH_BASE_PATH
  const sharedUrl = previewUrl
    ? basePath === undefined
      ? previewUrl
      : new URL(basePath || "/", previewUrl).href
    : configuredUrl
  return sharedUrl.startsWith("http") ? sharedUrl.replace(/\/$/, "") : ""
}
