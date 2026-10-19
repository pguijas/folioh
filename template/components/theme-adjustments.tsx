"use client"

import type { CSSProperties, ReactNode } from "react"
import { HugeiconsIcon } from "@hugeicons/react"
import { Moon02Icon, Sun03Icon } from "@hugeicons/core-free-icons"
import {
  DEFAULT_CONFIG,
  type ThemeConfig,
  borderOptions,
  codeTreatmentOptions,
  configForPreset,
  configVars,
  fontOptions,
  getPresetDefaults,
  radiusOptions,
  rhythmOptions,
  shellPaddingOptions,
} from "@/components/theme-configurator"
import {
  type PresetOptionValues,
  type ThemePreset,
  adjustControls,
  colorControl,
  normalizePresetOptions,
  resolvePresetTheme,
} from "@/theme/preset-types"
import { builtinGalleryVariants } from "@/theme/preset-registry"

// The rows of the picker's Customize step and the colour row it shares with
// the Themes step. Every row is a fieldset of native radios, controlled by the
// gallery, which keeps the draft; nothing here reads or writes storage.

type Customization = ThemeConfig["customization"]

const MODES = [
  { id: "light", label: "Light", icon: Sun03Icon },
  { id: "dark", label: "Dark", icon: Moon02Icon },
  { id: "system", label: "System", icon: undefined },
] as const

function presetOptions(draft: ThemeConfig, preset: ThemePreset): PresetOptionValues {
  return normalizePresetOptions(preset, draft.optionsByPreset[preset.id])
}

// What "Reset to <Theme> defaults" returns a draft to, and what the dots mark:
// on the site preset the docs.yaml tune and radius, elsewhere the theme's own
// defaults. The colour picked in the colour row stays.
export function themeDefaults(draft: ThemeConfig, preset: ThemePreset): ThemeConfig {
  const isSitePreset = preset.id === DEFAULT_CONFIG.presetId
  const base = isSitePreset
    ? { radiusIndex: DEFAULT_CONFIG.radiusIndex, customization: DEFAULT_CONFIG.customization }
    : getPresetDefaults(preset)
  const options = normalizePresetOptions(
    preset,
    isSitePreset ? DEFAULT_CONFIG.optionsByPreset[preset.id] : undefined
  )
  const control = colorControl(preset)
  if (control) options[control.id] = presetOptions(draft, preset)[control.id]!
  return {
    ...draft,
    radiusIndex: base.radiusIndex,
    optionsByPreset: { ...draft.optionsByPreset, [preset.id]: options },
    customization: base.customization,
    colorOverrides: undefined,
  }
}

function AdjustSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="theme-adjust-section">
      <h3>{title}</h3>
      {children}
    </section>
  )
}

interface RowItem {
  value: string
  label: string
  face?: ReactNode
  style?: CSSProperties
}

// One row of the Customize step: a legend, then one pill per option, with a
// dot on the default. The kind picks the pill's shape: text, card or swatch.
function Row({
  id,
  label,
  items,
  value,
  fallback,
  kind = "text",
  disabled = false,
  note,
  onChange,
}: {
  id: string
  label: string
  items: RowItem[]
  value: string
  fallback: string
  kind?: "text" | "card" | "swatch"
  disabled?: boolean
  note?: string
  onChange: (value: string) => void
}) {
  return (
    <fieldset className="theme-adjust-row" data-picker-row={id} data-kind={kind} disabled={disabled}>
      <legend>{label}</legend>
      {note ? <p className="theme-adjust-note">{note}</p> : null}
      <span className="theme-adjust-options">
        {items.map((item) => (
          <label key={item.value} className="theme-adjust-option" style={item.style} title={item.label}>
            <input
              type="radio"
              name={`picker-${id}`}
              value={item.value}
              checked={item.value === value}
              onChange={() => onChange(item.value)}
              onClick={() => { if (item.value === value) onChange(item.value) }}
              disabled={disabled}
              className="sr-only"
            />
            {item.face}
            <span className="theme-adjust-option-label">{item.label}</span>
            {item.value === fallback ? (
              <>
                <span className="theme-adjust-default" aria-hidden="true" />
                <span className="sr-only"> (default)</span>
              </>
            ) : null}
          </label>
        ))}
      </span>
    </fieldset>
  )
}

// Light, dark or system for the draft. While the front
// theme fixes its own scheme the radios are disabled and show that scheme;
// the stored mode stays as it was.
export function ModeRadios({
  mode,
  forced,
  note,
  noteId,
  onChange,
}: {
  mode: string | undefined
  forced?: "light" | "dark"
  note?: string
  noteId?: string
  onChange: (mode: string, from: Element) => void
}) {
  const disabled = Boolean(forced)
  return (
    <fieldset
      data-picker-mode
      disabled={disabled}
      title={note}
      aria-describedby={note ? noteId : undefined}
    >
      <legend className="sr-only">Mode</legend>
      {MODES.map((option) => (
        <label key={option.id} title={option.label}>
          <input
            type="radio"
            name="picker-mode"
            value={option.id}
            checked={(forced ?? mode) === option.id}
            onChange={(event) =>
              onChange(option.id, event.currentTarget.closest("label") ?? event.currentTarget)
            }
            onClick={(event) => {
              if (mode === option.id) onChange(option.id, event.currentTarget.closest("label") ?? event.currentTarget)
            }}
            disabled={disabled}
            aria-label={option.label}
            className="sr-only"
          />
          {option.icon ? (
            <HugeiconsIcon icon={option.icon} size={15} strokeWidth={1.8} aria-hidden="true" />
          ) : (
            <SystemIcon />
          )}
          <span>{option.label}</span>
        </label>
      ))}
    </fieldset>
  )
}

// A circle half filled: the scheme follows the system, light or dark.
function SystemIcon() {
  return (
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <circle cx="12" cy="12" r="8.5" stroke="currentColor" strokeWidth="1.8" />
      <path d="M12 3.5a8.5 8.5 0 0 1 0 17z" fill="currentColor" />
    </svg>
  )
}

// A family's recipes pair a preset's style with each of its colors.
export function themeVariants(presets: ThemePreset[]) {
  const ordered = [
    ...Array.from(builtinGalleryVariants.keys()).filter((preset) => presets.includes(preset)),
    ...presets.filter((preset) => !builtinGalleryVariants.has(preset)),
  ]
  return ordered.flatMap((preset) => {
    const control = colorControl(preset)
    const catalog = builtinGalleryVariants.get(preset)
    const curated = catalog?.length === 0 && preset.id === DEFAULT_CONFIG.presetId ? undefined : catalog
    const values = curated ?? control?.options.map((option) => option.value) ?? [undefined]
    return values.map((value) => {
      const option = control?.options.find((option) => option.value === value)
      return {
        preset, control, option,
        label: curated?.length === 1 ? preset.name : presets.length > 1
          ? [preset.name, option?.label].filter(Boolean).join(" · ")
          : option?.label ?? preset.name,
      }
    })
  })
}

export function themeVariantConfig(draft: ThemeConfig, preset: ThemePreset, value?: string) {
  const options = normalizePresetOptions(preset)
  const control = colorControl(preset)
  if (control && value) options[control.id] = value
  return themeDefaults(configForPreset(draft, preset.id, options), preset)
}

export function VariantRadios({ presets, draft, dark, onPick, onCustomize }: {
  presets: ThemePreset[]
  draft: ThemeConfig
  dark: boolean
  onPick: (preset: ThemePreset, value?: string) => void
  onCustomize: () => void
}) {
  return (
    <fieldset className="theme-gallery-variants" data-picker-variants>
      <legend className="sr-only">Theme variants</legend>
      <span className="theme-gallery-variants-list">
        {themeVariants(presets).map(({ preset, control, option, label }) => {
          const recipe = themeVariantConfig(draft, preset, option?.value)
          const scheme = resolvePresetTheme(preset, recipe.optionsByPreset[preset.id]).scheme
          const vars = configVars(recipe, dark) as CSSProperties
          const checked = draft.presetId === preset.id && (!control || presetOptions(draft, preset)[control.id] === option?.value)
          return (
            <label key={`${preset.id}:${option?.value ?? ""}`} className="theme-gallery-swatch" title={label}>
              <input type="radio" name="picker-variant" value={`${preset.id}:${option?.value ?? ""}`}
                checked={checked} onChange={() => onPick(preset, option?.value)}
                onClick={() => { if (checked) onPick(preset, option?.value) }}
                aria-label={scheme ? `${label}, ${scheme} only` : label} className="sr-only" />
              <span className="theme-gallery-swatch-disc" style={vars} />
              {scheme ? (
                <span className="theme-gallery-swatch-scheme" style={vars}>
                  <HugeiconsIcon icon={scheme === "dark" ? Moon02Icon : Sun03Icon} size={9} strokeWidth={2.4} aria-hidden="true" />
                </span>
              ) : null}
            </label>
          )
        })}
        <button type="button" className="theme-gallery-swatch theme-gallery-customize"
          data-picker-action="adjust" aria-label="Customize" title="Customize" onClick={onCustomize}>
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
            <circle cx="12" cy="12" r="10" strokeDasharray="2 2" />
            <path d="M6 8h3m4 0h5M6 16h7m4 0h1" />
            <circle cx="11" cy="8" r="2" /><circle cx="15" cy="16" r="2" />
          </svg>
        </button>
      </span>
    </fieldset>
  )
}

// The theme's own controls other than its colour. Nothing when it has none.
function PresetOptionRows({
  preset,
  draft,
  defaults,
  onChange,
}: {
  preset: ThemePreset
  draft: ThemeConfig
  defaults: ThemeConfig
  onChange: (controlId: string, value: string) => void
}) {
  const controls = adjustControls(preset)
  if (controls.length === 0) return null
  const values = presetOptions(draft, preset)
  const fallbacks = presetOptions(defaults, preset)

  return (
    <AdjustSection title={`${preset.name} options`}>
      {controls.map((control) => (
        <Row
          key={control.id}
          id={control.id}
          label={control.label}
          items={control.options.map((option) => ({
            value: option.value,
            label: option.label,
            face: option.swatch ? (
              <span className="theme-adjust-disc" style={{ background: option.swatch }} aria-hidden="true" />
            ) : undefined,
          }))}
          value={values[control.id] ?? ""}
          fallback={fallbacks[control.id] ?? ""}
          onChange={(value) => onChange(control.id, value)}
        />
      ))}
    </AdjustSection>
  )
}

// Rhythm, borders, code blocks and the page frame: named options that set a
// few spacing and edge properties each.
function StyleOptionRow({
  id,
  label,
  options,
  value,
  fallback,
  onChange,
}: {
  id: keyof Customization
  label: string
  options: Array<{ id: string; label: string }>
  value: string
  fallback: string
  onChange: (value: string) => void
}) {
  return (
    <Row
      id={id}
      label={label}
      items={options.map((option) => ({ value: option.id, label: optionLabel(option) }))}
      value={value}
      fallback={fallback}
      onChange={onChange}
    />
  )
}

// Each typography card is drawn in the heading face it sets.
function FontCards({ value, fallback, onChange }: RowProps) {
  return (
    <Row
      id="fontId"
      label="Typography"
      kind="card"
      items={fontOptions.map((option) => ({
        value: option.id,
        label: optionLabel(option),
        face: (
          <span
            className="theme-adjust-sample"
            style={{ fontFamily: option.style["--folioh-heading-font-family"] }}
            aria-hidden="true"
          >
            {option.sample}
          </span>
        ),
      }))}
      value={value}
      fallback={fallback}
      onChange={onChange}
    />
  )
}

// Each corner pill is drawn with the radius it sets.
function CornerPills({ value, fallback, onChange }: RowProps) {
  return (
    <Row
      id="radiusIndex"
      label="Corners"
      items={radiusOptions.map((option, index) => ({
        value: String(index),
        label: option.label,
        style: { borderRadius: option.value },
      }))}
      value={value}
      fallback={fallback}
      onChange={onChange}
    />
  )
}

interface RowProps {
  value: string
  fallback: string
  onChange: (value: string) => void
}

const COLOR_FIELDS = [
  ["--background", "Background"], ["--foreground", "Text"], ["--primary", "Accent"],
] as const

type ColorToken = typeof COLOR_FIELDS[number][0]

// Let the browser convert the preset's OKLCH colors for native color inputs.
function colorHex(value: string) {
  const canvas = document.createElement("canvas")
  canvas.width = canvas.height = 1
  const context = canvas.getContext("2d")!
  context.fillStyle = value
  context.fillRect(0, 0, 1, 1)
  return "#" + [...context.getImageData(0, 0, 1, 1).data].slice(0, 3)
    .map((channel) => channel.toString(16).padStart(2, "0")).join("")
}

// The option that keeps the theme's own value is stored as "preset" and
// reads "Default".
function optionLabel(option: { id: string; label: string }) {
  return option.id === "preset" ? "Default" : option.label
}

// Manual colors and the preset's typography, shape and layout controls.
export function CustomizeSections({
  preset,
  draft,
  dark,
  onPresetOption,
  onCustomization,
  onRadius,
  onColor,
}: {
  preset: ThemePreset
  draft: ThemeConfig
  dark: boolean
  onPresetOption: (controlId: string, value: string) => void
  onCustomization: (patch: Partial<Customization>) => void
  onRadius: (radiusIndex: number) => void
  onColor: (mode: "light" | "dark", token: ColorToken, value: string) => void
}) {
  const defaults = themeDefaults(draft, preset)
  const value = draft.customization
  const fallback = defaults.customization
  const scheme = resolvePresetTheme(preset, presetOptions(draft, preset)).scheme
  const mode = scheme ?? (dark ? "dark" : "light")
  const vars = configVars(draft, dark)

  return (
    <>
      <section className="theme-adjust-colors" aria-label="Colors">
        {COLOR_FIELDS.map(([token, label]) => (
          <label key={token} data-picker-row={token}>{label}
            <input type="color" value={colorHex(vars[token] ?? "#000000")}
              onChange={(event) => onColor(mode, token, event.target.value)} />
          </label>
        ))}
      </section>
      <PresetOptionRows preset={preset} draft={draft} defaults={defaults} onChange={onPresetOption} />
      <AdjustSection title="Type">
        <FontCards
          value={value.fontId}
          fallback={fallback.fontId}
          onChange={(fontId) => onCustomization({ fontId })}
        />
        <StyleOptionRow
          id="rhythmId"
          label="Reading rhythm"
          options={rhythmOptions}
          value={value.rhythmId}
          fallback={fallback.rhythmId}
          onChange={(rhythmId) => onCustomization({ rhythmId })}
        />
      </AdjustSection>
      <AdjustSection title="Shape">
        <CornerPills
          value={String(draft.radiusIndex)}
          fallback={String(defaults.radiusIndex)}
          onChange={(index) => onRadius(Number(index))}
        />
        <StyleOptionRow
          id="borderId"
          label="Borders"
          options={borderOptions}
          value={value.borderId}
          fallback={fallback.borderId}
          onChange={(borderId) => onCustomization({ borderId })}
        />
        <StyleOptionRow
          id="codeTreatmentId"
          label="Code block frame"
          options={codeTreatmentOptions}
          value={value.codeTreatmentId}
          fallback={fallback.codeTreatmentId}
          onChange={(codeTreatmentId) => onCustomization({ codeTreatmentId })}
        />
      </AdjustSection>
      <AdjustSection title="Layout">
        <StyleOptionRow
          id="shellPaddingId"
          label="Page frame"
          options={shellPaddingOptions}
          value={value.shellPaddingId}
          fallback={fallback.shellPaddingId}
          onChange={(shellPaddingId) => onCustomization({ shellPaddingId })}
        />
      </AdjustSection>
    </>
  )
}
