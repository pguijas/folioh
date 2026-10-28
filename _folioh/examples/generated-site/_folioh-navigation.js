// Exported anchors retain index.html for local files. HTTP navigation uses
// the corresponding directory URL, including anchors added by client routing.
(() => {
  if (!/^https?:$/.test(location.protocol)) return

  function normalize(anchor) {
    if (!anchor.matches?.("a[href]") || anchor.hasAttribute("download")) return
    let url
    try { url = new URL(anchor.getAttribute("href"), document.baseURI) }
    catch { return }
    if (url.origin !== location.origin || !url.pathname.endsWith("/index.html")) return
    url.pathname = url.pathname.slice(0, -"index.html".length)
    anchor.setAttribute("href", url.pathname + url.search + url.hash)
  }

  function scan(node) {
    if (!(node instanceof Element)) return
    normalize(node)
    node.querySelectorAll("a[href]").forEach(normalize)
  }

  scan(document.documentElement)
  new MutationObserver((records) => {
    for (const record of records) {
      if (record.type === "attributes") normalize(record.target)
      else record.addedNodes.forEach(scan)
    }
  }).observe(document.documentElement, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ["href"],
  })
})()
