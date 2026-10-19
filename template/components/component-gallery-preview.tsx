"use client"

import { useLayoutEffect, useRef, type ReactNode } from "react"

/** Fit the complete example into its card, including after interactive changes. */
export function ComponentGalleryPreview({
  title,
  compact = false,
  children,
}: {
  title: string
  compact?: boolean
  children: ReactNode
}) {
  const frameRef = useRef<HTMLDivElement>(null)
  const specimenRef = useRef<HTMLDivElement>(null)

  useLayoutEffect(() => {
    const frame = frameRef.current
    const specimen = specimenRef.current
    if (!frame || !specimen) return

    const fit = () => {
      const width = Math.max(specimen.offsetWidth, specimen.scrollWidth)
      const height = Math.max(specimen.offsetHeight, specimen.scrollHeight)
      if (!width || !height) return
      const scale = Math.min(0.85, frame.clientWidth / width, frame.clientHeight / height)
      specimen.style.setProperty("--gallery-scale", String(scale))
    }

    const observer = new ResizeObserver(fit)
    observer.observe(frame)
    observer.observe(specimen)
    fit()
    return () => observer.disconnect()
  }, [])

  return (
    <div className="component-gallery-preview" role="group" aria-label={`${title} example`}>
      <div className="component-gallery-fit" ref={frameRef}>
        <div
          className="component-gallery-specimen"
          data-compact={compact || undefined}
          ref={specimenRef}
        >
          {children}
        </div>
      </div>
    </div>
  )
}
