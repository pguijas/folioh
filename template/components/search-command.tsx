"use client"

import { useEffect, useId, useRef, useState } from "react"
import { Search } from "nextra/components"
import { Search01Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import { NEXTRA_CLASS } from "@/lib/nextra-dom"
import {
  type FoliohSearchDocument,
  foliohSearchDocuments,
  foliohSearchPlaceholder,
} from "@/lib/search-index"

interface SearchCommandProps {
  placeholder?: string
  inline?: boolean
}

const editableSelector = [
  "input",
  "textarea",
  "select",
  "button",
  '[contenteditable="true"]',
].join(",")

interface SearchData {
  url: string
  meta: { title: string }
  sub_results: Array<{
    title: string
    url: string
    excerpt: string
  }>
}

interface SearchResult {
  id: string
  score: number
  words: string[]
  data: () => Promise<SearchData>
}

interface SearchResponse {
  results: SearchResult[]
  unfilteredResultCount: number
  filters: Record<string, never>
  totalFilters: Record<string, never>
  timings: {
    preload: number
    search: number
    total: number
  }
}

interface PagefindLikeApi {
  options: () => Promise<void>
  preload: () => Promise<null>
  search: (term: string) => Promise<SearchResponse>
  debouncedSearch: (term: string) => Promise<SearchResponse>
}

declare global {
  interface Window {
    pagefind?: PagefindLikeApi
    __foliohStaticSearch?: PagefindLikeApi
  }
}

function normalize(value: string) {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
}

function escapeHtml(value: string) {
  return value.replace(/[&<>"']/g, (char) => {
    const entities: Record<string, string> = {
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#39;",
    }
    return entities[char] || char
  })
}

function escapeRegex(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}

function excerpt(content: string, terms: string[]) {
  const normalized = normalize(content)
  let index = -1
  for (const term of terms) {
    index = normalized.indexOf(term)
    if (index !== -1) break
  }

  const start = Math.max(0, index - 80)
  const raw = content.slice(start, start + 220).trim()
  let html = escapeHtml(
    `${start > 0 ? "... " : ""}${raw}${start + 220 < content.length ? " ..." : ""}`
  )

  for (const term of terms) {
    if (!term) continue
    html = html.replace(
      new RegExp(`(${escapeRegex(escapeHtml(term))})`, "ig"),
      "<mark>$1</mark>"
    )
  }

  return html
}

function createSearchApi(documents: FoliohSearchDocument[]): PagefindLikeApi {
  function runSearch(term: string): SearchResult[] {
    const terms = normalize(term).split(/\s+/).filter(Boolean)
    if (!terms.length) return []

    const results: SearchResult[] = []
    for (const doc of documents) {
      const title = normalize(doc.title)
      const content = normalize(doc.content)
      const haystack = `${title} ${content}`

      if (!terms.every((token) => haystack.includes(token))) {
        continue
      }

      const titleHits = terms.filter((token) => title.includes(token)).length
      const score =
        titleHits * 10 +
        terms.reduce(
          (total, token) => total + (content.includes(token) ? 1 : 0),
          0
        )

      results.push({
        id: doc.url,
        score,
        words: [],
        data: async () => ({
          url: doc.url,
          meta: { title: doc.title },
          sub_results: [
            {
              title: doc.title,
              url: doc.url,
              excerpt: excerpt(doc.content, terms),
            },
          ],
        }),
      })
    }

    return results.sort((a, b) => b.score - a.score)
  }

  const api: PagefindLikeApi = {
    options: async () => undefined,
    preload: async () => null,
    search: async (term) => {
      const results = runSearch(term)
      return {
        results,
        unfilteredResultCount: results.length,
        filters: {},
        totalFilters: {},
        timings: { preload: 0, search: 0, total: 0 },
      }
    },
    debouncedSearch: async (term) => api.search(term),
  }

  return api
}

function installDevelopmentSearchIndex() {
  if (
    process.env.NODE_ENV === "production" ||
    window.pagefind ||
    foliohSearchDocuments.length === 0
  ) {
    return
  }

  const api = createSearchApi(foliohSearchDocuments)
  window.__foliohStaticSearch = api
  window.pagefind = api
}

export function SearchCommand({
  placeholder = foliohSearchPlaceholder,
  inline = false,
}: SearchCommandProps) {
  const searchRef = useRef<HTMLDivElement>(null)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const panelId = useId()
  const [open, setOpen] = useState(false)

  function closeSearch() {
    setOpen(false)
    triggerRef.current?.focus({ preventScroll: true })
  }

  useEffect(() => {
    installDevelopmentSearchIndex()
  }, [])

  useEffect(() => {
    if (!open) return
    searchRef.current?.querySelector<HTMLInputElement>('input[type="search"]')?.focus()

    function onPointerDown(event: PointerEvent) {
      const target = event.target
      if (!(target instanceof Element)) return
      if (searchRef.current?.contains(target) || target.closest(`.${NEXTRA_CLASS.searchResults}`)) return
      setOpen(false)
    }
    function onResultClick(event: MouseEvent) {
      if (event.target instanceof Element && event.target.closest(`.${NEXTRA_CLASS.searchResults} a`)) setOpen(false)
    }
    function onResize() {
      setOpen(false)
    }
    document.addEventListener("pointerdown", onPointerDown)
    document.addEventListener("click", onResultClick)
    window.addEventListener("resize", onResize)
    return () => {
      document.removeEventListener("pointerdown", onPointerDown)
      document.removeEventListener("click", onResultClick)
      window.removeEventListener("resize", onResize)
    }
  }, [open])

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (
        event.key.toLowerCase() !== "k" ||
        event.shiftKey ||
        event.altKey ||
        (!event.metaKey && !event.ctrlKey)
      ) {
        return
      }

      const target = event.target instanceof HTMLElement ? event.target : null
      if (target !== triggerRef.current && target?.closest(editableSelector)) {
        return
      }

      if (triggerRef.current?.getClientRects().length) {
        event.preventDefault()
        setOpen(true)
        return
      }

      const input = searchRef.current?.querySelector<HTMLInputElement>(
        'input[type="search"]'
      )
      if (!input?.getClientRects().length) {
        return
      }

      event.preventDefault()
      input.focus({ preventScroll: true })
    }

    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [])

  if (inline) {
    return (
      <div ref={searchRef} data-folioh-search className="flex w-full justify-center text-left">
        <Search
          className="[&_kbd]:hidden"
          placeholder={placeholder}
          emptyResult="No matching docs or API pages."
          errorText="Search index unavailable."
          loading="Searching docs…"
        />
      </div>
    )
  }

  return (
    <div
      ref={searchRef}
      data-folioh-search
      data-folioh-search-open={open}
      className="contents"
      onKeyDownCapture={(event) => {
        if (open && event.key === "Escape") {
          // Let the combobox release its modal focus before returning to the rail.
          requestAnimationFrame(closeSearch)
        }
      }}
      onBlurCapture={(event) => {
        const next = event.relatedTarget
        if (!(next instanceof Element) || (!event.currentTarget.contains(next) && !next.closest(`.${NEXTRA_CLASS.searchResults}`))) setOpen(false)
      }}
    >
      <button
        ref={triggerRef}
        type="button"
        className="folioh-search-trigger"
        aria-label="Search documentation"
        title="Search documentation"
        aria-expanded={open}
        aria-controls={panelId}
        onClick={() => setOpen(!open)}
      >
        <HugeiconsIcon icon={Search01Icon} size={18} strokeWidth={1.8} aria-hidden="true" />
      </button>
      <div id={panelId} className="folioh-search-panel" role="search">
        <Search
          placeholder={placeholder}
          emptyResult="No matching docs or API pages."
          errorText="Search index unavailable."
          loading="Searching docs…"
        />
      </div>
    </div>
  )
}
