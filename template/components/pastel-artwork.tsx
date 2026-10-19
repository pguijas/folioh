"use client"

import { useEffect, useRef } from "react"
import { watchThemeArtwork } from "@/lib/theme-artwork-motion"

import { pastelShapes } from "@/theme/pastel-shapes"

export { pastelShapes } from "@/theme/pastel-shapes"

// One composition shared by the landing, document and picker. Motion uses
// the same visibility boundary as Omarchy, with a static SVG fallback.
export function PastelArtwork({ seed = "folioh" }: { seed?: string }) {
  const host = useRef<HTMLSpanElement>(null)
  const shapes = pastelShapes(seed)

  useEffect(() => {
    const element = host.current
    if (!element) return
    return watchThemeArtwork(element, () => {
      const animations: Animation[] = []
      element.querySelectorAll("svg").forEach((svg, i) => {
        animations.push(svg.animate([
          { transform: "translateY(0) rotate(-5deg)" },
          { transform: `translateY(${i % 2 ? -8 : 6}px) rotate(5deg)` },
        ], { duration: 5500 + i * 1100, iterations: Infinity, direction: "alternate", easing: "ease-in-out" }))
        svg.querySelectorAll<SVGPathElement>("path[data-morph]").forEach(path => {
          if (!CSS.supports("d", `path("${path.getAttribute("d")}")`)) return
          animations.push(path.animate([
            { d: `path("${path.getAttribute("d")}")` },
            { d: `path("${path.dataset.morph}")` },
          ], { duration: 7000 + i * 2300, iterations: Infinity, direction: "alternate", easing: "ease-in-out" }))
        })
      })
      const follow = (event: PointerEvent) => {
        if (event.pointerType !== "mouse" || (event.target as Element)?.closest?.('[role="dialog"]')) return
        const box = element.getBoundingClientRect()
        const dx = event.clientX - box.left - box.width / 2
        const dy = event.clientY - box.top - box.height / 2
        const influence = Math.max(0, 1 - Math.hypot(dx, dy) / 520)
        element.style.translate = `${dx * .045 * influence}px ${dy * .04 * influence}px`
      }
      const reset = () => { element.style.removeProperty("translate") }
      if (!element.closest(".theme-gallery-page")) {
        document.addEventListener("pointermove", follow, { passive: true })
        document.addEventListener("pointerleave", reset)
      }
      element.dataset.live = "true"
      return () => {
        animations.forEach(animation => animation.cancel())
        document.removeEventListener("pointermove", follow)
        document.removeEventListener("pointerleave", reset)
        reset()
        delete element.dataset.live
      }
    }, "pastel")
  }, [seed])

  return (
    <span ref={host} className="folioh-pastel-artwork" aria-hidden="true" data-pagefind-ignore="all">
      {shapes.map((shape, i) => (
        <svg key={i} viewBox="0 0 100 100" focusable="false">
          <path d={shape.from} data-morph={shape.to} fill="currentColor" />
        </svg>
      ))}
    </span>
  )
}
