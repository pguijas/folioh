// Preview examples have their own deploy path under the same preview origin.
export function siteMetadataUrl(configuredUrl) {
  const previewUrl = process.env.FOLIOH_PREVIEW_URL
  const basePath = process.env.NEXT_PUBLIC_FOLIOH_BASE_PATH
  const exampleUrl = process.env.FOLIOH_EXAMPLE_SITE_URL
  const deploymentUrl = previewUrl || (exampleUrl !== undefined ? exampleUrl : undefined)
  const sharedUrl = deploymentUrl !== undefined
    ? basePath === undefined
      ? deploymentUrl
      : deploymentUrl ? new URL(basePath || "/", deploymentUrl).href : ""
    : configuredUrl
  return sharedUrl.startsWith("http") ? sharedUrl.replace(/\/$/, "") : ""
}

// Only an explicitly originless nested example suppresses bundled social images.
export function siteSocialImagesEnabled() {
  return process.env.FOLIOH_EXAMPLE_SITE_URL !== "" || Boolean(process.env.FOLIOH_PREVIEW_URL)
}
