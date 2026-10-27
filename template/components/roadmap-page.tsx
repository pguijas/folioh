import { roadmapPhases, type RoadmapPhase } from "@/lib/roadmap-data"
import { groupPhasesByProject, type RoadmapProject } from "@/lib/roadmap-utils"
import { RoadmapHeader } from "@/components/roadmap-header"
import { RoadmapCards, ROADMAP_PRODUCT_PARAM } from "@/components/roadmap-cards"

/**
 * The /roadmap page: the masthead, then one card per product.
 *
 * Both roadmaps are on the page at once. The version before this one put a
 * switcher in the header and drew a single project at a time, which meant the
 * product you were not reading was not on the page — and with two products
 * that is half the roadmap behind a control nobody asked for. It also kept the
 * unselected releases out of the served HTML.
 *
 * `?product=<key>` collapses every card but one, so a product landing can link
 * straight to its own plan. Without it every card is open, which is what the
 * server renders.
 *
 * A server component. `roadmap.tsx` still draws the landing's miniature and
 * the embeds in the guides, unchanged.
 */
interface RoadmapPageProps {
  phases?: RoadmapPhase[]
  projects?: Record<string, RoadmapProject>
  title?: string
  description?: string
  links?: { label: string; href: string }[]
}

/* A project key reaches this file from docs.yaml, and it is about to be
 * interpolated into a stylesheet and a script. Anything outside this alphabet
 * is dropped rather than escaped: a key that needs escaping is a key nobody
 * should be writing, and the cards still work for it — they just do not get
 * the pre-paint path. */
const SAFE_KEY = /^[A-Za-z0-9_-]+$/

/**
 * Decide the collapsed state before the first paint.
 *
 * The static HTML renders every card open, and the query string only reaches
 * React after hydration. Arriving at `/roadmap/?product=<key>` must select
 * that project's card before paint, so the other cards do not briefly open
 * and move the content when hydration finishes.
 *
 * So the page ships one rule per project and five lines that set an attribute,
 * the same shape the template already uses to settle the theme before paint.
 * The React effect then takes over and `RoadmapCards` drops the attribute on
 * the commit that follows, so CSS and state hand over with nothing moving.
 */
function ProductBootstrap({ keys }: { keys: string[] }) {
  if (!keys.length) return null

  const css = keys
    .map(
      /* `rotate`, not `transform`: Tailwind v4 compiles `rotate-180` to the
         standalone `rotate` property, so resetting `transform` here would
         leave the hidden card's chevron pointing at a body that is not there
         for as long as the bootstrap is in charge. */
      (key) => `html[data-roadmap-product="${key}"] [data-roadmap-card]:not([data-roadmap-card="${key}"])>:not(summary){display:none}
html[data-roadmap-product="${key}"] [data-roadmap-card]:not([data-roadmap-card="${key}"]) [data-roadmap-chevron]{rotate:none;transform:none}`
    )
    .join("\n")

  const script = `(function(){try{var k=${JSON.stringify(keys)},p=new URLSearchParams(location.search).get(${JSON.stringify(ROADMAP_PRODUCT_PARAM)});if(p&&k.indexOf(p)>-1){document.documentElement.setAttribute("data-roadmap-product",p)}}catch(e){}})()`

  return (
    <>
      <style dangerouslySetInnerHTML={{ __html: css }} />
      <script dangerouslySetInnerHTML={{ __html: script }} />
    </>
  )
}

export function RoadmapPage({
  phases = roadmapPhases,
  projects,
  title,
  description,
  links,
}: RoadmapPageProps) {
  /* A project declared in docs.yaml but carried by no phase has no releases,
   * so it gets no card. `groupPhasesByProject` has already ordered every group
   * by version ascending, and docs.yaml's key order is the card order. */
  const groups = groupPhasesByProject(phases, projects).filter(
    (group) => group.phases.length > 0
  )
  const named = groups
    .filter((group) => Boolean(projects?.[group.key]?.label))
    .map((group) => group.key)

  return (
    <RoadmapHeader title={title} description={description} links={links}>
      <ProductBootstrap
        keys={groups.map((group) => group.key).filter((key) => SAFE_KEY.test(key))}
      />
      <RoadmapCards groups={groups} named={named} />
    </RoadmapHeader>
  )
}
