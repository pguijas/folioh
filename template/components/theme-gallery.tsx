"use client"

import {
  type CSSProperties,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type TouchEvent as ReactTouchEvent,
  useCallback,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from "react"
import { createPortal } from "react-dom"
import { useTheme } from "next-themes"
import { HugeiconsIcon } from "@hugeicons/react"
import {
  ArrowLeft01Icon,
  ArrowRight01Icon,
  Cancel01Icon,
  PaintBoardIcon,
} from "@hugeicons/core-free-icons"
import { darkModeEnabled, isTypingTarget } from "@/components/theme-provider"
import { ThemePixelField, ThemeWordmark } from "@/components/theme-artwork"
import { PastelArtwork } from "@/components/pastel-artwork"
import {
  DEFAULT_CONFIG,
  SELECTABLE_PRESETS,
  type ThemeConfig,
  configForPreset,
  configVars,
  readThemeConfig,
  saveThemeConfig,
} from "@/components/theme-configurator"
import {
  CustomizeSections,
  ModeRadios,
  VariantRadios,
  themeDefaults,
  themeVariantConfig,
  themeVariants,
} from "@/components/theme-adjustments"
import { switchScheme } from "@/lib/scheme-transition"
import { builtinGalleryVariants, getGroups, groupPresetsForDisplay } from "@/theme/preset-registry"
import {
  type PresetControl,
  type PresetControlOption,
  type ThemePreset,
  colorControl,
  normalizePresetOptions,
  resolvePresetTheme,
} from "@/theme/preset-types"

// The theme picker as a gallery of slides. The idea of a gallery comes from
// omarchy.org; the drawing and every value here are Folioh's own. One slide
// per color-and-style recipe, grouped into families. Customize
// is a page inside the same dialog; every choice applies immediately.

interface GalleryTheme {
  id: string
  name: string
  presets: ThemePreset[]
}

// The config each theme would apply, kept while the picker is open, so a
// colour picked on one theme is still there after stepping away and back.
type Drafts = Record<string, ThemeConfig>

type Scheme = "light" | "dark" | undefined

type Step = "themes" | "adjust"

interface Variant {
  control: PresetControl
  option: PresetControlOption
}

// Group Folioh's styles without changing stored preset IDs.
// Project overrides and independently registered presets remain selectable.
export function listThemes(): GalleryTheme[] {
  const groups = groupPresetsForDisplay(getGroups(), SELECTABLE_PRESETS)
  const project = groups.find((group) => group.id === "project")?.presets ?? []
  const remaining = groups.flatMap((group) => group.presets).filter((preset) => !project.includes(preset))
  const folioh = remaining.filter((preset) => builtinGalleryVariants.has(preset) && preset.id !== "omarchy" && preset.id !== "pastel")
  const single = (preset: ThemePreset): GalleryTheme => ({ id: `preset:${preset.id}`, name: preset.name, presets: [preset] })
  const standalone = remaining.filter((preset) => !folioh.includes(preset))
  return [
    ...project.map(single),
    ...(folioh.length ? [{ id: "folioh", name: "Folioh", presets: folioh }] : []),
    ...standalone.map(single),
  ]
}

// The applied theme starts from the saved config; every other theme from
// what the reader last stored for it, or its own defaults.
function firstDrafts(saved: ThemeConfig, themes: GalleryTheme[]): Drafts {
  return Object.fromEntries(
    themes.map((family) => [family.id, family.presets.some((preset) => preset.id === saved.presetId)
      ? saved : configForPreset(saved, family.presets[0]!.id)])
  )
}

function draftOptions(draft: ThemeConfig, preset: ThemePreset) {
  return normalizePresetOptions(preset, draft.optionsByPreset[preset.id])
}

function draftScheme(draft: ThemeConfig, preset: ThemePreset): Scheme {
  return resolvePresetTheme(preset, draftOptions(draft, preset)).scheme
}

// The colour a draft picks on its theme's colour control, if it has one.
function draftVariant(draft: ThemeConfig, preset: ThemePreset): Variant | undefined {
  const control = colorControl(preset)
  if (!control) return undefined
  const value = draftOptions(draft, preset)[control.id]
  const index = Math.max(0, control.options.findIndex((option) => option.value === value))
  const option = control.options[index]
  return option ? { control, option } : undefined
}

// The slides and swatches share one order. Keep a retired recipe reachable
// until another variant replaces its family's draft, without adding a swatch.
export function gallerySlides(themes: GalleryTheme[], drafts: Drafts) {
  return themes.flatMap((family, familyIndex) => {
    const draft = drafts[family.id]!
    const preset = family.presets.find((preset) => preset.id === draft.presetId) ?? family.presets[0]!
    const selected = draftVariant(draft, preset)
    const variants = themeVariants(family.presets)
    const matches = (variant: (typeof variants)[number]) =>
      variant.preset.id === preset.id && variant.option?.value === selected?.option.value
    if (!variants.some(matches)) {
      variants.unshift({
        preset, control: colorControl(preset), option: selected?.option,
        label: [preset.name, selected?.option.label].filter(Boolean).join(" · "),
      })
    }
    return variants.map((variant) => ({
      ...variant,
      familyIndex,
      selected: matches(variant),
      key: `${family.id}:${variant.preset.id}:${variant.option?.value ?? ""}`,
      draft: matches(variant) ? draft : themeVariantConfig(draft, variant.preset, variant.option?.value),
    }))
  })
}

// Whether a draft looks as the saved theme does: the same theme, options,
// customization and corners.
function looksSaved(draft: ThemeConfig, saved: ThemeConfig | null, preset: ThemePreset) {
  if (!saved || saved.presetId !== preset.id) return false
  const options = draftOptions(draft, preset)
  const savedOptions = draftOptions(saved, preset)
  const keys = Object.keys(draft.customization) as Array<keyof ThemeConfig["customization"]>
  return (
    draft.radiusIndex === saved.radiusIndex &&
    JSON.stringify(draft.colorOverrides ?? {}) === JSON.stringify(saved.colorOverrides ?? {}) &&
    Object.keys(options).every((key) => options[key] === savedOptions[key]) &&
    keys.every((key) => draft.customization[key] === saved.customization[key])
  )
}

function wrap(index: number, length: number) {
  return ((index % length) + length) % length
}

// The slides on stage as [offset, index] pairs: the centre and up to two on
// each side, never the same slide twice when there are fewer than five.
function visibleSlides(index: number, length: number): Array<[number, number]> {
  const seen = new Set<number>()
  const visible: Array<[number, number]> = []

  for (const offset of [0, 1, -1, 2, -2]) {
    const slide = wrap(index + offset, length)
    if (seen.has(slide)) continue
    seen.add(slide)
    visible.push([offset, slide])
  }

  return visible
}

const MODE_LABELS = { light: "Light", dark: "Dark", system: "System" }

const FOCUSABLE = "button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), [tabindex]:not([tabindex='-1'])"
const SWIPE_DISTANCE = 40

// The elements Tab stops on, in order. Of a group of radios only the checked
// one stops, or the first when none is checked, as the browser does.
function tabStops(container: HTMLElement) {
  const elements = Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE))
  return elements.filter((element) => {
    if (element.tabIndex < 0 || element.getClientRects().length === 0) return false
    if (!(element instanceof HTMLInputElement) || element.type !== "radio") return true
    const group = elements.filter(
      (other): other is HTMLInputElement =>
        other instanceof HTMLInputElement && other.type === "radio" && other.name === element.name
    )
    return (group.find((radio) => radio.checked) ?? group[0]) === element
  })
}

// Keep the selected swatch in view without scrolling the dialog.
function revealInRow(item: Element | null) {
  const row = item?.parentElement
  if (!item || !row || row.scrollWidth <= row.clientWidth) return
  const box = item.getBoundingClientRect()
  const rowBox = row.getBoundingClientRect()
  if (box.left >= rowBox.left && box.right <= rowBox.right) return
  row.scrollLeft += box.left + box.width / 2 - (rowBox.left + rowBox.width / 2)
}

// A click on a slide or an arrow leaves focus where it was: a slide can leave
// the stage while it would hold focus, and Enter should apply the front slide
// rather than press the arrow again.
function keepFocus(event: ReactMouseEvent<HTMLElement>) {
  event.preventDefault()
}

// The only theme control of the landing, the docs and the previews. The
// trigger matches the other icon buttons of the navbars.
export function ThemeGallery() {
  const { theme: mode, setTheme, systemTheme } = useTheme()
  const [open, setOpen] = useState(false)
  const [themes, setThemes] = useState<GalleryTheme[]>([])
  const [saved, setSaved] = useState<ThemeConfig | null>(null)
  const [drafts, setDrafts] = useState<Drafts>({})
  const [index, setIndex] = useState(0)
  const [step, setStep] = useState<Step>("themes")
  const [draftMode, setDraftMode] = useState("system")
  const titleId = useId()
  const modeNoteId = useId()
  const stageHintId = useId()
  const triggerRef = useRef<HTMLButtonElement | null>(null)
  const returnFocusRef = useRef<HTMLElement | null>(null)
  const dialogRef = useRef<HTMLDivElement | null>(null)
  const stageRef = useRef<HTMLDivElement | null>(null)
  const touchRef = useRef<{ x: number; y: number } | null>(null)

  const openGallery = useCallback(() => {
    const active = document.activeElement
    returnFocusRef.current = active instanceof HTMLElement && active !== document.body ? active : null
    const config = readThemeConfig()
    const nextThemes = listThemes()
    setSaved(config)
    setThemes(nextThemes)
    setDrafts(firstDrafts(config, nextThemes))
    setIndex(Math.max(0, nextThemes.findIndex((family) => family.presets.some((preset) => preset.id === config.presetId))))
    setStep("themes")
    setDraftMode(darkModeEnabled ? mode ?? "system" : "light")
    setOpen(true)
  }, [mode])

  const close = useCallback(() => {
    setOpen(false)
    const previous = returnFocusRef.current
    returnFocusRef.current = null
    if (previous?.isConnected) previous.focus()
    else triggerRef.current?.focus()
  }, [])

  const family = themes[index]
  const frontDraft = family ? drafts[family.id] : undefined
  const front = useMemo(() => family && frontDraft
    ? { ...family, preset: family.presets.find((preset) => preset.id === frontDraft.presetId) ?? family.presets[0]! }
    : undefined, [family, frontDraft])

  const slides = useMemo(() => gallerySlides(themes, drafts), [themes, drafts])
  const slideIndex = slides.findIndex((slide) => slide.familyIndex === index && slide.selected)

  // Only deliberate edits apply a theme; reopening keeps the saved selection.
  const commitDraft = useCallback((familyId: string, draft: ThemeConfig) => {
    const next = { ...draft, optionsByPreset: {
      ...readThemeConfig().optionsByPreset,
      [draft.presetId]: draft.optionsByPreset[draft.presetId]!,
    } }
    setDrafts((current) => ({ ...current, [familyId]: next }))
    saveThemeConfig(next)
    setSaved(next)
  }, [])

  const goToSlide = useCallback((target: number) => {
    const slide = slides[target]
    if (!slide) return
    const family = themes[slide.familyIndex]!
    commitDraft(family.id, slide.draft)
    setIndex(slide.familyIndex)
  }, [commitDraft, slides, themes])

  const move = useCallback((delta: number) => {
    goToSlide(wrap(slideIndex + delta, slides.length || 1))
  }, [goToSlide, slideIndex, slides.length])

  const pickVariant = useCallback((preset: ThemePreset, value?: string) => {
    const family = themes.find((entry) => entry.presets.includes(preset))
    if (!family) return
    const draft = drafts[family.id]
    if (draft) commitDraft(family.id, themeVariantConfig(draft, preset, value))
  }, [commitDraft, drafts, themes])

  const stepVariant = useCallback((delta: number) => {
    if (!front || !frontDraft) return
    const variants = themeVariants(front.presets)
    if (!variants.length) return
    const selected = draftVariant(frontDraft, front.preset)
    const selectedIndex = variants.findIndex(({ preset, option }) => preset.id === front.preset.id && option?.value === selected?.option.value)
    const nextIndex = selectedIndex < 0
      ? (delta > 0 ? 0 : variants.length - 1)
      : wrap(selectedIndex + delta, variants.length)
    const next = variants[nextIndex]!
    pickVariant(next.preset, next.option?.value)
  }, [front, frontDraft, pickVariant])

  // Every change of the Customize step goes to the front theme's draft.
  const updateFront = useCallback((change: (draft: ThemeConfig, preset: ThemePreset) => ThemeConfig) => {
    if (front && frontDraft) commitDraft(front.id, change(frontDraft, front.preset))
  }, [commitDraft, front, frontDraft])

  const setPresetOption = useCallback((controlId: string, value: string) => {
    updateFront((draft, preset) =>
      configForPreset(draft, preset.id, { ...draftOptions(draft, preset), [controlId]: value })
    )
  }, [updateFront])

  const setCustomization = useCallback((patch: Partial<ThemeConfig["customization"]>) => {
    updateFront((draft) => ({ ...draft, customization: { ...draft.customization, ...patch } }))
  }, [updateFront])

  const setRadius = useCallback((radiusIndex: number) => {
    updateFront((draft) => ({ ...draft, radiusIndex }))
  }, [updateFront])

  const setColor = useCallback((mode: "light" | "dark", token: "--background" | "--foreground" | "--primary", value: string) => {
    updateFront((draft) => ({ ...draft, colorOverrides: {
      ...draft.colorOverrides, [mode]: { ...draft.colorOverrides?.[mode], [token]: value },
    } }))
  }, [updateFront])

  const resetAdjustments = useCallback(() => {
    updateFront(themeDefaults)
  }, [updateFront])

  const changeMode = useCallback((nextMode: string, from?: Element) => {
    if (!darkModeEnabled) return
    if (front && frontDraft) commitDraft(front.id, frontDraft)
    setDraftMode(nextMode)
    switchScheme(setTheme, nextMode, from)
  }, [commitDraft, front, frontDraft, setTheme])

  const chooseFront = useCallback(() => {
    if (front && frontDraft) commitDraft(front.id, frontDraft)
    close()
  }, [close, commitDraft, front, frontDraft])

  // Restore and apply the site's own theme with its configured defaults.
  const resetToSiteDefault = useCallback(() => {
    const siteIndex = themes.findIndex((entry) => entry.presets.some((preset) => preset.id === DEFAULT_CONFIG.presetId))
    const family = themes[siteIndex]
    const site = family?.presets.find((preset) => preset.id === DEFAULT_CONFIG.presetId)
    if (!site) return
    const siteOptions = normalizePresetOptions(site, DEFAULT_CONFIG.optionsByPreset[site.id])
    const optionsByPreset = { ...drafts[family.id]?.optionsByPreset, [site.id]: siteOptions }
    commitDraft(family.id, { ...DEFAULT_CONFIG, optionsByPreset })
    setIndex(siteIndex)
  }, [commitDraft, drafts, themes])

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.defaultPrevented || event.repeat || open) return
      if (event.metaKey || event.ctrlKey || event.altKey) return
      if (event.key?.toLowerCase() !== "t" || isTypingTarget(event.target)) return
      // Every mounted gallery hears the key; the first one to take it wins.
      event.preventDefault()
      openGallery()
    }

    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [open, openGallery])

  useEffect(() => {
    if (!open) return
    const root = document.documentElement
    const overflow = root.style.overflow
    root.style.overflow = "hidden"
    return () => {
      root.style.overflow = overflow
    }
  }, [open])

  // The stage takes focus on open, so the arrow keys and Enter work at once;
  // focus that a step moved out of the dialog comes back to it. Customize
  // starts on its first row.
  useEffect(() => {
    const dialog = dialogRef.current
    if (!open || !dialog) return
    if (step === "adjust") {
      const row = dialog.querySelector("[data-picker-row]")
      const radio =
        row?.querySelector<HTMLInputElement>("input:checked:not(:disabled)") ??
        row?.querySelector<HTMLInputElement>("input:not(:disabled)")
      // The panel stays at its top, so its heading and colours are in view.
      radio?.focus({ preventScroll: true })
    } else if (!dialog.contains(document.activeElement)) {
      stageRef.current?.focus()
    }
  }, [open, index, step])

  // The selected colour stays in view as the reader moves.
  useEffect(() => {
    const dialog = dialogRef.current
    if (!open || !dialog) return
    revealInRow(dialog.querySelector("[data-picker-variants] input:checked")?.parentElement ?? null)
  }, [open, index, drafts, step])

  function moveFocus(dialog: HTMLElement, delta: number) {
    const stops = tabStops(dialog)
    if (stops.length === 0) return
    const current = stops.indexOf(document.activeElement as HTMLElement)
    const next = current < 0 ? (delta > 0 ? 0 : stops.length - 1) : wrap(current + delta, stops.length)
    stops[next]!.focus()
  }

  function onDialogKeyDown(event: ReactKeyboardEvent<HTMLDivElement>) {
    const target = event.target
    const onRadio = target instanceof HTMLInputElement && target.type === "radio"
    const modified = event.metaKey || event.ctrlKey || event.altKey
    const moves: Record<string, () => void> = {
      ArrowLeft: () => move(-1),
      ArrowRight: () => move(1),
      ArrowUp: () => stepVariant(-1),
      ArrowDown: () => stepVariant(1),
      Home: () => goToSlide(0),
      End: () => goToSlide(slides.length - 1),
    }

    if (event.key === "Escape") {
      // Escape leaves Customize first, keeping its draft, then closes.
      event.preventDefault()
      if (step === "adjust") setStep("themes")
      else close()
    } else if (event.key === "Tab") {
      // Focus stays inside the dialog while it is open.
      event.preventDefault()
      moveFocus(event.currentTarget, event.shiftKey ? -1 : 1)
    } else if (event.key?.toLowerCase() === "t" && !modified && !event.repeat && !isTypingTarget(target)) {
      event.preventDefault()
      close()
    } else if (event.key?.toLowerCase() === "d" && darkModeEnabled && !modified && !event.repeat && !isTypingTarget(target)) {
      event.preventDefault()
      if (!frontScheme) changeMode(readerDark ? "light" : "dark")
    } else if (event.key?.toLowerCase() === "c" && !modified && !event.repeat && !isTypingTarget(target)) {
      // c opens Customize for the front theme, and leaves it again.
      event.preventDefault()
      setStep(step === "adjust" ? "themes" : "adjust")
    } else if (event.key === "Enter") {
      // Enter chooses the front slide and closes. In Customize only
      // Cmd/Ctrl+Enter closes, leaving the controls' native Enter behavior.
      if (!(event.metaKey || event.ctrlKey) && (step === "adjust" || target instanceof HTMLButtonElement || isTypingTarget(target))) return
      event.preventDefault()
      if (step === "adjust") close()
      else chooseFront()
    } else if (step === "themes" && moves[event.key] && !modified && !onRadio && !isTypingTarget(target)) {
      // A focused radio group moves through its own options instead.
      event.preventDefault()
      moves[event.key]!()
    }
  }

  function onStageTouchEnd(event: ReactTouchEvent<HTMLDivElement>) {
    const start = touchRef.current
    const touch = event.changedTouches[0]
    touchRef.current = null
    if (!start || !touch) return
    const dx = touch.clientX - start.x
    const dy = touch.clientY - start.y
    if (Math.abs(dx) >= SWIPE_DISTANCE && Math.abs(dx) > Math.abs(dy)) {
      move(dx < 0 ? 1 : -1)
    }
  }

  const readerDark = darkModeEnabled && (draftMode === "system" ? systemTheme : draftMode) === "dark"
  const frontScheme = front && frontDraft ? draftScheme(frontDraft, front.preset) : undefined
  const frontVariant = front && frontDraft ? draftVariant(frontDraft, front.preset) : undefined
  // A palette can fix the page’s light or dark mode.
  const fixedBy = frontScheme ? frontVariant?.option.label ?? front?.preset.name : undefined
  const modeNote = frontScheme ? `${fixedBy} is ${frontScheme} only` : undefined
  // The colour by its own name, as the header names it; the control's name
  // for its colours is read in the legend and the live region.
  const selectedVariant = front && themeVariants(front.presets).find(({ preset, option }) =>
    preset.id === front.preset.id && option?.value === frontVariant?.option.value)
  const variantLine = selectedVariant?.label ?? (front?.presets.length && front.presets.length > 1
    ? [front.preset.name, frontVariant?.option.label].filter(Boolean).join(" · ")
    : frontVariant?.option.label)
  const frontApplied = front && frontDraft ? looksSaved(frontDraft, saved, front.preset) : false
  const frontSiteDefault = front?.preset.id === DEFAULT_CONFIG.presetId
  const checkedMode = MODE_LABELS[(frontScheme ?? draftMode) as keyof typeof MODE_LABELS]

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        data-theme-gallery-trigger
        onClick={openGallery}
        aria-haspopup="dialog"
        aria-label="Choose a theme"
        title="Choose a theme"
        className="inline-flex size-8 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
      >
        <HugeiconsIcon icon={PaintBoardIcon} size={16} strokeWidth={1.8} aria-hidden="true" />
      </button>
      {open && front && frontDraft
        ? createPortal(
            <div
              ref={dialogRef}
              role="dialog"
              aria-modal="true"
              aria-labelledby={titleId}
              tabIndex={-1}
              className="theme-gallery"
              data-step={step}
              onKeyDown={onDialogKeyDown}
              onClick={(event) => {
                if (event.target === event.currentTarget) close()
              }}
            >
              <div className="theme-gallery-box">
                <header className="theme-gallery-header">
                  {step === "adjust" ? (
                    <button
                      type="button"
                      className="theme-gallery-back"
                      data-picker-action="back"
                      aria-label="Back to themes"
                      onClick={() => setStep("themes")}
                    >
                      <HugeiconsIcon icon={ArrowLeft01Icon} size={14} strokeWidth={1.8} aria-hidden="true" />
                    </button>
                  ) : null}
                  <div className="theme-gallery-heading">
                    <h2 id={titleId}>{step === "adjust" ? "Customize" : "Theme"}</h2>
                  </div>
                  {darkModeEnabled ? (
                    <div className="theme-gallery-mode">
                      {/* The note line is always there, so the header keeps its
                          height; a phone names the checked mode in it when no
                          colour fixes the scheme. */}
                      {modeNote ? (
                        <p id={modeNoteId} className="theme-gallery-mode-note" data-forced="true">
                          <span className="theme-gallery-mode-note-by">{fixedBy} is </span>
                          {frontScheme} only
                        </p>
                      ) : (
                        <p className="theme-gallery-mode-note" aria-hidden="true">
                          {checkedMode}
                        </p>
                      )}
                      <ModeRadios
                        mode={draftMode}
                        forced={frontScheme}
                        note={modeNote}
                        noteId={modeNoteId}
                        onChange={changeMode}
                      />
                    </div>
                  ) : null}
                  <button
                    type="button"
                    className="theme-gallery-close"
                    data-picker-action="close"
                    aria-label="Close"
                    title="Close"
                    onClick={close}
                  >
                    <HugeiconsIcon icon={Cancel01Icon} size={16} strokeWidth={1.8} aria-hidden="true" />
                  </button>
                </header>
                {step === "themes" ? (
                  <>
                    <div className="theme-gallery-track">
                      <div
                        ref={stageRef}
                        className="theme-gallery-stage"
                        data-total={slides.length}
                        tabIndex={0}
                        role="group"
                        aria-roledescription="carousel"
                        aria-label="Themes"
                        aria-describedby={stageHintId}
                        onTouchStart={(event) => {
                          const touch = event.touches[0]
                          touchRef.current = touch ? { x: touch.clientX, y: touch.clientY } : null
                        }}
                        onTouchEnd={onStageTouchEnd}
                      >
                        {visibleSlides(slideIndex, slides.length).map(([offset, target]) => {
                          const { key, draft, preset, label } = slides[target]!
                          const isFront = offset === 0
                          return (
                            <button
                              key={key}
                              type="button"
                              className="theme-gallery-slide"
                              data-preset={preset.id}
                              data-offset={offset}
                              tabIndex={-1}
                              aria-hidden={isFront ? undefined : true}
                              aria-label={isFront ? `Choose ${label}` : `Go to ${label}`}
                              onMouseDown={keepFocus}
                              onClick={() => (isFront ? chooseFront() : goToSlide(target))}
                            >
                              <ThemeGalleryPage config={draft} dark={readerDark} />
                              <AppliedMark applied={looksSaved(draft, saved, preset)} />
                            </button>
                          )
                        })}
                      </div>
                      <button
                        type="button"
                        className="theme-gallery-arrow"
                        data-direction="previous"
                        aria-label="Previous theme"
                        onMouseDown={keepFocus}
                        onClick={() => move(-1)}
                      >
                        <HugeiconsIcon icon={ArrowLeft01Icon} size={20} strokeWidth={1.5} aria-hidden="true" />
                      </button>
                      <button
                        type="button"
                        className="theme-gallery-arrow"
                        data-direction="next"
                        aria-label="Next theme"
                        onMouseDown={keepFocus}
                        onClick={() => move(1)}
                      >
                        <HugeiconsIcon icon={ArrowRight01Icon} size={20} strokeWidth={1.5} aria-hidden="true" />
                      </button>
                    </div>
                    <span id={stageHintId} className="sr-only">
                      Arrow keys apply variants immediately, up and down stay within the family, Enter chooses and closes,
                      C customizes
                    </span>
                    <div className="theme-gallery-caption">
                      <span className="theme-gallery-caption-name">{front.name}</span>
                      {variantLine && variantLine !== front.name ? <span className="theme-gallery-caption-variant">{variantLine}</span> : null}
                    </div>
                    <VariantRadios
                      presets={front.presets}
                      draft={frontDraft}
                      dark={readerDark}
                      onPick={pickVariant}
                      onCustomize={() => setStep("adjust")}
                    />
                    <footer className="theme-gallery-footer">
                      <button type="button" data-picker-action="reset" title="Reset to site default" onClick={resetToSiteDefault}>
                        Reset
                      </button>
                      <button type="button" data-picker-action="done" onClick={close}>
                        Done
                      </button>
                    </footer>
                  </>
                ) : (
                  <>
                    <div className="theme-adjust-body">
                      <div className="theme-adjust-preview">
                        <ThemeGalleryPage config={frontDraft} dark={readerDark} />
                      </div>
                      <CustomizeSections
                        preset={front.preset}
                        draft={frontDraft}
                        dark={readerDark}
                        onPresetOption={setPresetOption}
                        onCustomization={setCustomization}
                        onRadius={setRadius}
                        onColor={setColor}
                      />
                    </div>
                    <footer className="theme-gallery-footer">
                      <button type="button" data-picker-action="reset-adjustments" onClick={resetAdjustments}>
                        Reset
                      </button>
                      <button type="button" data-picker-action="done" onClick={close}>
                        Done
                      </button>
                    </footer>
                  </>
                )}
                <span className="sr-only" aria-live="polite">
                  {step === "adjust" ? (
                    `Customize ${front.name}, ${variantLine ?? front.preset.name}`
                  ) : (
                    <>
                      {`${front.name}, ${variantLine ?? ""}, ${slideIndex + 1} of ${slides.length}`}
                      {frontScheme ? `, ${frontScheme} only` : ""}
                      {frontApplied ? ", in use" : ""}
                      {frontSiteDefault ? ", site default" : ""}
                    </>
                  )}
                </span>
              </div>
            </div>,
            document.body
          )
        : null}
    </>
  )
}

function AppliedMark({ applied }: { applied: boolean }) {
  if (!applied) return null
  return (
    <span className="theme-gallery-tags" title="Applied">
      <span data-picker-tag="applied" role="img" aria-label="Applied">✓</span>
    </span>
  )
}

// A small Folioh page drawn in the slide's own theme: its custom properties are
// scoped to this element, so everything inside resolves to that theme while
// the page around it keeps the reader's.
function ThemeGalleryPage({ config, dark }: { config: ThemeConfig; dark: boolean }) {
  const vars = configVars(config, dark) as CSSProperties

  return (
    <span className="theme-gallery-page" data-preset={config.presetId} style={vars} aria-hidden="true">
      {config.presetId === "omarchy" ? <ThemePixelField /> : null}
      <span className="theme-gallery-page-bar">
        <span className="theme-gallery-page-mark" />
        <span className="theme-gallery-page-brand">Folioh</span>
        <span className="theme-gallery-page-icons">
          <span />
          <span />
          <span />
        </span>
      </span>
      <span className="theme-gallery-page-body">
        <span className="theme-gallery-page-nav">
          {["Overview", "Quick start", "Configuration", "API reference"].map((label, row) => (
            <span key={label} data-active={row === 1 ? "true" : undefined}>
              {label}
            </span>
          ))}
        </span>
        <span className="theme-gallery-page-main">
          <span className="theme-gallery-page-content">
            {config.presetId === "omarchy" ? <ThemeWordmark name="Folioh" interactive={false} /> : null}
            {config.presetId === "pastel" ? <PastelArtwork /> : null}
            <span className="theme-gallery-page-title">Getting started</span>
            <span className="theme-gallery-page-text">
              Build clear docs from your source. Browse the <span className="theme-gallery-page-link">API reference</span>.
            </span>
            <span className="theme-gallery-page-code">
              <span><span className="theme-gallery-page-prompt">$</span> folioh build</span>
              <span><span className="theme-gallery-page-prompt">$</span> folioh serve</span>
            </span>
            <span className="theme-gallery-page-heading">Explore the docs</span>
            <span className="theme-gallery-page-cards">
              <span>
                <strong>Guides</strong>
                <span>Learn the basics</span>
              </span>
              <span>
                <strong>API</strong>
                <span>Types and methods</span>
              </span>
            </span>
          </span>
        </span>
      </span>
    </span>
  )
}
