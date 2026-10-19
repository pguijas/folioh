"use client"

import { useLayoutEffect } from "react"
import { THEME_SCHEME_EVENT } from "@/components/theme-provider"
import { projectThemeDefaultConfig } from "@/theme/project-theme"
import { presets } from "@/theme/presets"
import { selectablePresets } from "@/theme/preset-catalog"
import { themeRadiusScale } from "@/theme/theme-contract.generated"
import {
  type PresetOptionValues,
  type ResolvedPresetTheme,
  type ThemePreset,
  type ThemeStyle,
  type ThemeVars,
  buildBootstrapPresets,
  getPresetOptionKey,
  normalizePresetOptions,
  resolvePresetTheme,
} from "@/theme/preset-types"

// Radius values come from the generated theme contract so the TypeScript
// scale can never drift from the Python one; only the labels live here.
const radiusLabels = ["None", "Sm", "Md", "Lg", "Full"]
export const radiusOptions: Array<{ label: string; value: string }> = themeRadiusScale.map(
  (value, index) => ({ label: radiusLabels[index] ?? value, value })
)

export const fontOptions = [
  {
    id: "folioh",
    label: "Editorial",
    description: "Serif headings, steady sans body",
    sample: "Aa",
    style: {
      "--folioh-heading-font-family": "Georgia, \"Times New Roman\", ui-serif, serif",
      "--folioh-body-font-family": "var(--font-sans), ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif",
      "--folioh-code-font-family": "var(--font-mono), ui-monospace, SFMono-Regular, \"SF Mono\", Menlo, Consolas, monospace",
    },
  },
  {
    id: "sans",
    label: "System sans",
    description: "Clean UI typography",
    sample: "Ag",
    style: {
      "--folioh-heading-font-family": "var(--font-sans), ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif",
      "--folioh-body-font-family": "var(--font-sans), ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif",
      "--folioh-code-font-family": "var(--font-mono), ui-monospace, SFMono-Regular, \"SF Mono\", Menlo, Consolas, monospace",
    },
  },
  {
    id: "geist",
    label: "Geist",
    description: "p2pfl web services typography",
    sample: "Gg",
    style: {
      "--folioh-heading-font-family": "var(--font-geist-sans), ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif",
      "--folioh-body-font-family": "var(--font-geist-sans), ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif",
      "--folioh-code-font-family": "var(--font-geist-mono), ui-monospace, SFMono-Regular, \"SF Mono\", Menlo, Consolas, monospace",
    },
  },
  {
    id: "serif",
    label: "Book serif",
    description: "Bookish long-form reading",
    sample: "St",
    style: {
      "--folioh-heading-font-family": "Georgia, \"Times New Roman\", ui-serif, serif",
      "--folioh-body-font-family": "Georgia, \"Times New Roman\", ui-serif, serif",
      "--folioh-code-font-family": "var(--font-mono), ui-monospace, SFMono-Regular, \"SF Mono\", Menlo, Consolas, monospace",
    },
  },
  {
    id: "mono",
    label: "Reference mono",
    description: "Monospaced API scanning",
    sample: "01",
    style: {
      "--folioh-heading-font-family": "var(--font-mono), ui-monospace, SFMono-Regular, \"SF Mono\", Menlo, Consolas, monospace",
      "--folioh-body-font-family": "var(--font-sans), ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif",
      "--folioh-code-font-family": "var(--font-mono), ui-monospace, SFMono-Regular, \"SF Mono\", Menlo, Consolas, monospace",
    },
  },
  {
    id: "terminal",
    label: "Terminal",
    description: "Monospaced body, Geist headings",
    sample: ">_",
    style: {
      "--folioh-heading-font-family": "var(--font-geist-sans), ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif",
      "--folioh-body-font-family": "var(--font-mono), ui-monospace, SFMono-Regular, \"SF Mono\", Menlo, Consolas, monospace",
      "--folioh-code-font-family": "var(--font-mono), ui-monospace, SFMono-Regular, \"SF Mono\", Menlo, Consolas, monospace",
    },
  },
  {
    id: "grotesque",
    label: "Grotesque",
    description: "Bricolage Grotesque headings, DM Sans body",
    sample: "Gq",
    style: {
      "--folioh-heading-font-family": "var(--font-bricolage), ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif",
      "--folioh-body-font-family": "var(--font-dm-sans), ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif",
      "--folioh-code-font-family": "var(--font-mono), ui-monospace, SFMono-Regular, \"SF Mono\", Menlo, Consolas, monospace",
    },
  },
] satisfies Array<{
  id: string
  label: string
  description: string
  sample: string
  style: Pick<ResolvedPresetTheme["style"], "--folioh-heading-font-family" | "--folioh-body-font-family" | "--folioh-code-font-family">
}>

export const colorOptions = [
  {
    id: "ink",
    label: "Theme ink",
    description: "Use the preset color",
    preview: { light: "oklch(0.155 0.007 82)", dark: "oklch(0.920 0.007 82)" },
    light: {},
    dark: {},
  },
  {
    id: "laurel",
    label: "Laurel",
    description: "Botanical green accent",
    preview: { light: "oklch(0.410 0.095 146)", dark: "oklch(0.770 0.105 146)" },
    light: {
      "--primary": "oklch(0.410 0.095 146)",
      "--primary-foreground": "oklch(0.980 0.006 120)",
      "--ring": "oklch(0.410 0.095 146)",
      "--accent": "oklch(0.900 0.030 146)",
      "--accent-foreground": "oklch(0.155 0.007 82)",
      "--sidebar-primary": "oklch(0.410 0.095 146)",
      "--sidebar-primary-foreground": "oklch(0.980 0.006 120)",
      "--sidebar-ring": "oklch(0.410 0.095 146)",
      "--chart-1": "oklch(0.410 0.095 146)",
    },
    dark: {
      "--primary": "oklch(0.770 0.105 146)",
      "--primary-foreground": "oklch(0.120 0.018 146)",
      "--ring": "oklch(0.770 0.105 146)",
      "--accent": "oklch(0.260 0.045 146)",
      "--accent-foreground": "oklch(0.930 0.008 146)",
      "--sidebar-primary": "oklch(0.770 0.105 146)",
      "--sidebar-primary-foreground": "oklch(0.120 0.018 146)",
      "--sidebar-ring": "oklch(0.770 0.105 146)",
      "--chart-1": "oklch(0.770 0.105 146)",
    },
  },
  {
    id: "indigo",
    label: "Indigo",
    description: "Cool technical accent",
    preview: { light: "oklch(0.420 0.095 268)", dark: "oklch(0.760 0.090 268)" },
    light: {
      "--primary": "oklch(0.420 0.095 268)",
      "--primary-foreground": "oklch(0.975 0.006 268)",
      "--ring": "oklch(0.420 0.095 268)",
      "--accent": "oklch(0.900 0.028 268)",
      "--accent-foreground": "oklch(0.150 0.010 268)",
      "--sidebar-primary": "oklch(0.420 0.095 268)",
      "--sidebar-primary-foreground": "oklch(0.975 0.006 268)",
      "--sidebar-ring": "oklch(0.420 0.095 268)",
      "--chart-1": "oklch(0.420 0.095 268)",
    },
    dark: {
      "--primary": "oklch(0.760 0.090 268)",
      "--primary-foreground": "oklch(0.115 0.014 268)",
      "--ring": "oklch(0.760 0.090 268)",
      "--accent": "oklch(0.260 0.045 268)",
      "--accent-foreground": "oklch(0.920 0.008 268)",
      "--sidebar-primary": "oklch(0.760 0.090 268)",
      "--sidebar-primary-foreground": "oklch(0.115 0.014 268)",
      "--sidebar-ring": "oklch(0.760 0.090 268)",
      "--chart-1": "oklch(0.760 0.090 268)",
    },
  },
  {
    id: "copper",
    label: "Copper",
    description: "Warm editorial accent",
    preview: { light: "oklch(0.500 0.105 54)", dark: "oklch(0.780 0.100 54)" },
    light: {
      "--primary": "oklch(0.500 0.105 54)",
      "--primary-foreground": "oklch(0.985 0.010 54)",
      "--ring": "oklch(0.500 0.105 54)",
      "--accent": "oklch(0.900 0.040 54)",
      "--accent-foreground": "oklch(0.170 0.012 54)",
      "--sidebar-primary": "oklch(0.500 0.105 54)",
      "--sidebar-primary-foreground": "oklch(0.985 0.010 54)",
      "--sidebar-ring": "oklch(0.500 0.105 54)",
      "--chart-1": "oklch(0.500 0.105 54)",
    },
    dark: {
      "--primary": "oklch(0.780 0.100 54)",
      "--primary-foreground": "oklch(0.125 0.014 54)",
      "--ring": "oklch(0.780 0.100 54)",
      "--accent": "oklch(0.270 0.050 54)",
      "--accent-foreground": "oklch(0.930 0.010 54)",
      "--sidebar-primary": "oklch(0.780 0.100 54)",
      "--sidebar-primary-foreground": "oklch(0.125 0.014 54)",
      "--sidebar-ring": "oklch(0.780 0.100 54)",
      "--chart-1": "oklch(0.780 0.100 54)",
    },
  },
] satisfies Array<{
  id: string
  label: string
  description: string
  preview: { light: string; dark: string }
  light: Partial<ThemeVars>
  dark: Partial<ThemeVars>
}>

type StyleOption = {
  id: string
  label: string
  style: Partial<ThemeStyle>
}

type SurfaceColorOption = {
  id: string
  label: string
  preview: { light: string; dark: string }
  light: Partial<ThemeVars>
  dark: Partial<ThemeVars>
}

export const surfaceColorOptions = [
  {
    id: "preset",
    label: "Preset",
    preview: { light: "oklch(0.155 0.007 82)", dark: "oklch(0.920 0.007 82)" },
    light: {},
    dark: {},
  },
  {
    id: "paper",
    label: "Paper",
    preview: { light: "oklch(0.966 0.008 82)", dark: "oklch(0.140 0.008 82)" },
    light: {
      "--background": "oklch(0.966 0.008 82)",
      "--foreground": "oklch(0.155 0.007 82)",
      "--card": "oklch(0.976 0.007 82)",
      "--popover": "oklch(0.982 0.006 82)",
      "--muted": "oklch(0.920 0.007 82)",
      "--muted-foreground": "oklch(0.420 0.007 82)",
      "--border": "oklch(0.740 0.007 82)",
      "--sidebar": "oklch(0.940 0.007 82)",
      "--sidebar-accent": "oklch(0.890 0.008 82)",
    },
    dark: {
      "--background": "oklch(0.140 0.008 82)",
      "--foreground": "oklch(0.925 0.007 82)",
      "--card": "oklch(0.170 0.008 82)",
      "--popover": "oklch(0.185 0.008 82)",
      "--muted": "oklch(0.225 0.008 82)",
      "--muted-foreground": "oklch(0.660 0.007 82)",
      "--border": "oklch(0.340 0.008 82)",
      "--sidebar": "oklch(0.115 0.008 82)",
      "--sidebar-accent": "oklch(0.205 0.010 82)",
    },
  },
  {
    id: "moss",
    label: "Moss",
    preview: { light: "oklch(0.958 0.014 112)", dark: "oklch(0.125 0.012 128)" },
    light: {
      "--background": "oklch(0.958 0.014 112)",
      "--foreground": "oklch(0.150 0.010 128)",
      "--card": "oklch(0.974 0.012 112)",
      "--popover": "oklch(0.982 0.010 112)",
      "--muted": "oklch(0.900 0.018 112)",
      "--muted-foreground": "oklch(0.410 0.020 128)",
      "--border": "oklch(0.720 0.018 112)",
      "--sidebar": "oklch(0.925 0.018 112)",
      "--sidebar-accent": "oklch(0.872 0.026 120)",
    },
    dark: {
      "--background": "oklch(0.125 0.012 128)",
      "--foreground": "oklch(0.910 0.010 112)",
      "--card": "oklch(0.158 0.012 128)",
      "--popover": "oklch(0.175 0.012 128)",
      "--muted": "oklch(0.210 0.014 128)",
      "--muted-foreground": "oklch(0.630 0.014 112)",
      "--border": "oklch(0.320 0.014 128)",
      "--sidebar": "oklch(0.102 0.012 128)",
      "--sidebar-accent": "oklch(0.195 0.014 128)",
    },
  },
  {
    id: "mist",
    label: "Mist",
    preview: { light: "oklch(0.977 0.006 150)", dark: "oklch(0.128 0.010 170)" },
    light: {
      "--background": "oklch(0.977 0.006 150)",
      "--foreground": "oklch(0.160 0.008 170)",
      "--card": "oklch(0.990 0.005 150)",
      "--popover": "oklch(0.996 0.004 150)",
      "--muted": "oklch(0.930 0.008 150)",
      "--muted-foreground": "oklch(0.440 0.012 170)",
      "--border": "oklch(0.790 0.008 150)",
      "--sidebar": "oklch(0.952 0.007 150)",
      "--sidebar-accent": "oklch(0.910 0.010 156)",
    },
    dark: {
      "--background": "oklch(0.128 0.010 170)",
      "--foreground": "oklch(0.920 0.006 150)",
      "--card": "oklch(0.160 0.010 170)",
      "--popover": "oklch(0.176 0.010 170)",
      "--muted": "oklch(0.215 0.010 170)",
      "--muted-foreground": "oklch(0.635 0.008 150)",
      "--border": "oklch(0.320 0.010 170)",
      "--sidebar": "oklch(0.104 0.010 170)",
      "--sidebar-accent": "oklch(0.195 0.010 170)",
    },
  },
] satisfies SurfaceColorOption[]

export const shellPaddingOptions = [
  { id: "preset", label: "Preset", style: {} },
  { id: "flush", label: "Flush", style: { "--folioh-workspace-shell-padding": "0px" } },
  { id: "frame", label: "Frame", style: { "--folioh-workspace-shell-padding": "18px" } },
  { id: "gallery", label: "Gallery", style: { "--folioh-workspace-shell-padding": "28px" } },
] satisfies StyleOption[]

const contentWidthOptions = [
  { id: "preset", label: "Preset", style: {} },
  { id: "focus", label: "Focus", style: { "--folioh-content-max-width": "54rem" } },
  { id: "docs", label: "Docs", style: { "--folioh-content-max-width": "62rem" } },
  { id: "wide", label: "Wide", style: { "--folioh-content-max-width": "74rem" } },
] satisfies StyleOption[]

export const rhythmOptions = [
  { id: "preset", label: "Preset", style: {} },
  {
    id: "compact",
    label: "Compact",
    style: {
      "--folioh-font-size-base": "0.95rem",
      "--folioh-body-line-height": "1.54",
      "--folioh-section-gap": "2.4rem",
      "--folioh-card-padding": "0.95rem",
    },
  },
  {
    id: "balanced",
    label: "Balanced",
    style: {
      "--folioh-font-size-base": "1rem",
      "--folioh-body-line-height": "1.62",
      "--folioh-section-gap": "3.2rem",
      "--folioh-card-padding": "1.2rem",
    },
  },
  {
    id: "roomy",
    label: "Roomy",
    style: {
      "--folioh-font-size-base": "1.03rem",
      "--folioh-body-line-height": "1.72",
      "--folioh-section-gap": "4.1rem",
      "--folioh-card-padding": "1.45rem",
    },
  },
] satisfies StyleOption[]

export const borderOptions = [
  { id: "preset", label: "Preset", style: {} },
  {
    id: "fine",
    label: "Fine",
    style: {
      "--folioh-card-border-width": "1px",
      "--folioh-card-shadow": "none",
      "--folioh-card-hover-shadow": "0 0 0 1px color-mix(in oklch, var(--border) 64%, var(--background))",
      "--folioh-workspace-shell-border": "1px solid color-mix(in oklch, var(--border) 70%, var(--background))",
      "--folioh-workspace-shell-shadow": "none",
    },
  },
  {
    id: "structured",
    label: "Structured",
    style: {
      "--folioh-card-border-width": "1px",
      "--folioh-card-shadow": "0 18px 54px -48px var(--foreground)",
      "--folioh-card-hover-shadow": "0 16px 42px -34px var(--foreground)",
      "--folioh-workspace-shell-border": "1px solid var(--border)",
      "--folioh-workspace-shell-shadow": "0 22px 70px -58px var(--foreground)",
    },
  },
  {
    id: "ruled",
    label: "Ruled",
    style: {
      "--folioh-card-border-width": "1.5px",
      "--folioh-card-shadow": "0 0 0 1px var(--border)",
      "--folioh-card-hover-shadow": "0 0 0 1.5px var(--border), 0 18px 48px -38px var(--foreground)",
      "--folioh-workspace-shell-border": "1.5px solid var(--border)",
      "--folioh-workspace-shell-shadow": "0 0 0 1px var(--border)",
    },
  },
] satisfies StyleOption[]

export const codeTreatmentOptions = [
  { id: "preset", label: "Preset", style: {} },
  {
    id: "soft",
    label: "Soft",
    style: {
      "--folioh-code-bg": "color-mix(in oklch, var(--muted) 86%, var(--background))",
      "--folioh-code-foreground": "inherit",
      "--folioh-code-border": "1px solid color-mix(in oklch, var(--border) 72%, var(--background))",
      "--folioh-code-border-radius": "0.5rem",
      "--folioh-code-shadow": "none",
    },
  },
  {
    id: "framed",
    label: "Framed",
    style: {
      "--folioh-code-bg": "var(--background)",
      "--folioh-code-foreground": "inherit",
      "--folioh-code-border": "1px solid var(--border)",
      "--folioh-code-border-radius": "0.35rem",
      "--folioh-code-shadow": "0 16px 48px -42px var(--foreground)",
    },
  },
  {
    id: "plate",
    label: "Plate",
    style: {
      "--folioh-code-bg": "color-mix(in oklch, var(--card) 72%, var(--muted))",
      "--folioh-code-foreground": "inherit",
      "--folioh-code-border": "1.5px solid var(--border)",
      "--folioh-code-border-radius": "0.15rem",
      "--folioh-code-shadow": "none",
    },
  },
  {
    id: "terminal",
    label: "Terminal",
    style: {
      "--folioh-code-bg": "color-mix(in oklch, var(--card) 84%, var(--background))",
      "--folioh-code-foreground": "inherit",
      "--folioh-code-border": "1px solid var(--border)",
      "--folioh-code-border-radius": "0.5rem",
      "--folioh-code-shadow": "var(--shadow-sm, none)",
    },
  },
] satisfies StyleOption[]

interface ThemeCustomization {
  fontId: string
  colorId: string
  surfaceColorId: string
  shellPaddingId: string
  contentWidthId: string
  rhythmId: string
  borderId: string
  codeTreatmentId: string
}

export type ThemeColorOverrides = Partial<
  Record<"light" | "dark", Partial<Record<"--background" | "--foreground" | "--primary", string>>>
>

interface ThemeConfig {
  presetId: string
  radiusIndex: number
  optionsByPreset: Record<string, PresetOptionValues>
  customization: ThemeCustomization
  colorOverrides?: ThemeColorOverrides
}

type LegacyThemeConfig = Partial<ThemeConfig> & {
  themeId?: string
  flavorId?: string
  optionsByFlavor?: Record<string, PresetOptionValues>
}

// A theme package's project-theme.ts may export an untyped literal that leaves
// keys out or sets the very customization keys defaulted below, so read it
// through the shape the configurator expects, every key optional.
const packageDefaults: {
  presetId?: string
  radiusIndex?: number
  optionsByPreset?: Record<string, PresetOptionValues>
  customization?: Partial<ThemeCustomization>
} = projectThemeDefaultConfig
const configuredDefaultPresetId = "pastel" // __FOLIOH_THEME_PRESET__
const DEFAULT_PRESET = presets.find((preset) => preset.id === (packageDefaults.presetId ?? configuredDefaultPresetId)) ?? presets[0]!
export const SELECTABLE_PRESETS = selectablePresets(DEFAULT_PRESET.id)
const DEFAULT_CUSTOMIZATION: ThemeCustomization = {
  fontId: "sans",
  colorId: "ink",
  surfaceColorId: "preset",
  shellPaddingId: "preset",
  contentWidthId: "preset",
  rhythmId: "preset",
  borderId: "preset",
  codeTreatmentId: "preset",
  ...DEFAULT_PRESET.defaultCustomization,
  ...packageDefaults.customization,
}
export const DEFAULT_CONFIG: ThemeConfig = {
  presetId: DEFAULT_PRESET.id,
  radiusIndex: packageDefaults.radiusIndex ?? DEFAULT_PRESET.defaultRadiusIndex ?? 2,
  optionsByPreset: {
    ...packageDefaults.optionsByPreset,
    [DEFAULT_PRESET.id]: normalizePresetOptions(DEFAULT_PRESET, packageDefaults.optionsByPreset?.[DEFAULT_PRESET.id]),
  },
  customization: DEFAULT_CUSTOMIZATION,
}
const STORAGE_KEY = `folioh-theme:${DEFAULT_CONFIG.presetId}`
// Readers of sites that used Folioh's former default keep their selection.
const PREVIOUS_STORAGE_KEY = DEFAULT_CONFIG.presetId === "pastel" ? "folioh-theme:organic-editorial" : undefined
// Pre-namespacing storage key; migrated to STORAGE_KEY on first read.
const LEGACY_STORAGE_KEY = "folioh-theme"
const SHELL_THEME_CSS = `
html {
  background: var(--folioh-workspace-shell-topbar);
}

body {
  min-height: 100vh;
  padding: var(--folioh-workspace-shell-padding);
  background: var(--folioh-workspace-shell-background);
}

body > .nextra-navbar {
  top: var(--folioh-workspace-shell-padding) !important;
  background: var(--folioh-workspace-shell-topbar) !important;
}

body > .nextra-navbar .nextra-navbar-blur {
  border: var(--folioh-workspace-shell-border);
  border-bottom: var(--folioh-workspace-shell-topbar-border);
  background: var(--folioh-workspace-shell-topbar) !important;
  -webkit-backdrop-filter: var(--folioh-workspace-shell-topbar-blur) !important;
  backdrop-filter: var(--folioh-workspace-shell-topbar-blur) !important;
}

.landing-shell {
  min-height: calc(100vh - (var(--folioh-workspace-shell-padding) * 2));
  overflow: hidden;
  border: var(--folioh-workspace-shell-border);
  background: var(--folioh-workspace-shell-surface);
  box-shadow: var(--folioh-workspace-shell-shadow);
}

.landing-navbar {
  top: var(--folioh-workspace-shell-padding) !important;
  right: var(--folioh-workspace-shell-padding);
  left: var(--folioh-workspace-shell-padding);
  width: calc(100% - (var(--folioh-workspace-shell-padding) * 2)) !important;
  border: var(--folioh-workspace-shell-border);
  border-bottom: var(--folioh-workspace-shell-topbar-border);
  background: var(--folioh-workspace-shell-topbar) !important;
  -webkit-backdrop-filter: var(--folioh-workspace-shell-topbar-blur) !important;
  backdrop-filter: var(--folioh-workspace-shell-topbar-blur) !important;
}

body > div:has(> .nextra-sidebar) {
  overflow: clip;
  border-right: var(--folioh-workspace-shell-border);
  border-bottom: var(--folioh-workspace-shell-border);
  border-left: var(--folioh-workspace-shell-border);
  background: var(--folioh-workspace-shell-surface);
  box-shadow: var(--folioh-workspace-shell-shadow);
}

.nextra-sidebar {
  top: calc(var(--nextra-navbar-height) + var(--folioh-workspace-shell-padding)) !important;
  height: calc(100dvh - var(--nextra-navbar-height) - (var(--folioh-workspace-shell-padding) * 2)) !important;
  z-index: 60 !important;
}

.nextra-toc > div {
  top: calc(var(--nextra-navbar-height) + var(--folioh-workspace-shell-padding)) !important;
  max-height: calc(100dvh - var(--nextra-navbar-height) - (var(--folioh-workspace-shell-padding) * 2)) !important;
}

@media (max-width: 767px) {
  body > .nextra-navbar {
    top: 0 !important;
    margin-right: calc(var(--folioh-workspace-shell-padding) * -1);
    margin-left: calc(var(--folioh-workspace-shell-padding) * -1);
    width: calc(100% + (var(--folioh-workspace-shell-padding) * 2)) !important;
  }

  .landing-navbar {
    top: 0 !important;
    right: 0;
    left: 0;
    width: 100% !important;
  }

  .nextra-sidebar,
  .nextra-toc > div {
    top: var(--nextra-navbar-height) !important;
  }

  .nextra-sidebar {
    height: calc(100dvh - var(--nextra-navbar-height) - var(--folioh-workspace-shell-padding)) !important;
  }

  .nextra-toc > div {
    max-height: calc(100dvh - var(--nextra-navbar-height) - var(--folioh-workspace-shell-padding)) !important;
  }
}
`
const LEGACY_PRESET_IDS: Record<string, string> = {
  "atelier": "atlas",
  "oxide": "proof",
  "signal": "ledger",
  "depth": "stacks",
  "flora": "draftline",
  "folioh": "atlas",
  "reference": "atlas",
  "promptix": "beacon",
  "openai": "aperture",
  "press": "proof",
  "archive": "stacks",
  "draft": "draftline",
  "ledger": "ledger",
  "carbon": "carbon",
}

function getRadius(index: number) {
  return radiusOptions[index] ?? radiusOptions[DEFAULT_CONFIG.radiusIndex]
}

function getRadiusIndex(value: string, fallback = DEFAULT_CONFIG.radiusIndex) {
  const index = radiusOptions.findIndex((option) => option.value === value)
  return index >= 0 ? index : fallback
}

function getFontOption(id: string | undefined) {
  return fontOptions.find((option) => option.id === id) ?? fontOptions[1]!
}

function getColorOption(id: string | undefined) {
  return colorOptions.find((option) => option.id === id) ?? colorOptions[0]!
}

function getSurfaceColorOption(id: string | undefined) {
  return surfaceColorOptions.find((option) => option.id === id) ?? surfaceColorOptions[0]!
}

function getShellPaddingOption(id: string | undefined) {
  return shellPaddingOptions.find((option) => option.id === id) ?? shellPaddingOptions[0]!
}

function getContentWidthOption(id: string | undefined) {
  return contentWidthOptions.find((option) => option.id === id) ?? contentWidthOptions[0]!
}

function getRhythmOption(id: string | undefined) {
  return rhythmOptions.find((option) => option.id === id) ?? rhythmOptions[0]!
}

function getBorderOption(id: string | undefined) {
  return borderOptions.find((option) => option.id === id) ?? borderOptions[0]!
}

function getCodeTreatmentOption(id: string | undefined) {
  return codeTreatmentOptions.find((option) => option.id === id) ?? codeTreatmentOptions[0]!
}

function normalizeCustomization(input: Partial<ThemeCustomization> = {}): ThemeCustomization {
  return {
    fontId: getFontOption(input.fontId).id,
    colorId: getColorOption(input.colorId).id,
    surfaceColorId: getSurfaceColorOption(input.surfaceColorId).id,
    shellPaddingId: getShellPaddingOption(input.shellPaddingId).id,
    contentWidthId: getContentWidthOption(input.contentWidthId).id,
    rhythmId: getRhythmOption(input.rhythmId).id,
    borderId: getBorderOption(input.borderId).id,
    codeTreatmentId: getCodeTreatmentOption(input.codeTreatmentId).id,
  }
}

function getCustomizationStyle(customization: ThemeCustomization): Partial<ThemeStyle> {
  return {
    ...getShellPaddingOption(customization.shellPaddingId).style,
    ...getContentWidthOption(customization.contentWidthId).style,
    ...getRhythmOption(customization.rhythmId).style,
    ...getBorderOption(customization.borderId).style,
    ...getCodeTreatmentOption(customization.codeTreatmentId).style,
  }
}

function normalizePresetId(id: string | undefined): string {
  if (!id || id === "custom") return DEFAULT_CONFIG.presetId
  return LEGACY_PRESET_IDS[id] ?? id
}

function getPreset(id: string | undefined): ThemePreset {
  const migratedId = normalizePresetId(id)
  return presets.find((preset) => preset.id === migratedId) ?? DEFAULT_PRESET
}

export function getPresetDefaults(preset: ThemePreset) {
  const radiusIndex = radiusOptions[preset.defaultRadiusIndex ?? DEFAULT_CONFIG.radiusIndex]
    ? preset.defaultRadiusIndex ?? DEFAULT_CONFIG.radiusIndex
    : DEFAULT_CONFIG.radiusIndex

  return {
    radiusIndex,
    customization: normalizeCustomization(preset.defaultCustomization ?? DEFAULT_CUSTOMIZATION),
  }
}

function getPresetOptions(config: ThemeConfig, presetId: string): PresetOptionValues {
  const preset = getPreset(presetId)
  return normalizePresetOptions(preset, config.optionsByPreset[preset.id])
}

function getRequestedPresetId(input: LegacyThemeConfig): string | undefined {
  if (input.presetId && input.presetId !== "custom") return input.presetId
  return input.flavorId ?? input.themeId ?? input.presetId
}

function getMigratedOptions(input: LegacyThemeConfig): Record<string, PresetOptionValues> {
  const optionsByPreset =
    input.optionsByPreset && typeof input.optionsByPreset === "object"
      ? { ...input.optionsByPreset }
      : {}
  const legacyOptions =
    input.optionsByFlavor && typeof input.optionsByFlavor === "object"
      ? input.optionsByFlavor
      : {}

  for (const [legacyId, options] of Object.entries(legacyOptions)) {
    const preset = getPreset(legacyId)
    if (!optionsByPreset[preset.id]) {
      optionsByPreset[preset.id] = options
    }
  }

  return optionsByPreset
}

function normalizeColorOverrides(input: unknown): ThemeColorOverrides | undefined {
  if (!input || typeof input !== "object") return undefined
  const result: ThemeColorOverrides = {}
  for (const mode of ["light", "dark"] as const) {
    const values = (input as Record<string, unknown>)[mode]
    if (!values || typeof values !== "object") continue
    for (const token of ["--background", "--foreground", "--primary"] as const) {
      const value = (values as Record<string, unknown>)[token]
      if (typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value)) {
        const colors = (result[mode] ??= {})
        colors[token] = value
      }
    }
  }
  return Object.keys(result).length ? result : undefined
}

function normalizeConfig(input: LegacyThemeConfig = {}): ThemeConfig {
  const requestedId = getRequestedPresetId(input)
  const preset = getPreset(requestedId === "custom" ? input.flavorId ?? input.themeId : requestedId)
  const defaults = getPresetDefaults(preset)
  const optionsByPreset = getMigratedOptions(input)
  if (requestedId === "pastel" && !optionsByPreset.pastel?.palette) {
    optionsByPreset.pastel = { ...optionsByPreset.pastel, palette: "jade" }
  }
  const requestedRadiusIndex = Number.isInteger(input.radiusIndex)
    ? Number(input.radiusIndex)
    : defaults.radiusIndex

  if (!optionsByPreset[preset.id]) {
    optionsByPreset[preset.id] = normalizePresetOptions(preset)
  }

  return {
    presetId: preset.id,
    radiusIndex: radiusOptions[requestedRadiusIndex] ? requestedRadiusIndex : defaults.radiusIndex,
    optionsByPreset,
    customization: normalizeCustomization(input.customization ?? defaults.customization),
    colorOverrides: normalizeColorOverrides(input.colorOverrides),
  }
}

function readableConfig(input: LegacyThemeConfig): ThemeConfig {
  const config = normalizeConfig(input)
  return SELECTABLE_PRESETS.some((preset) => preset.id === config.presetId) ? config : DEFAULT_CONFIG
}

function loadConfig(): ThemeConfig {
  if (typeof window === "undefined") return DEFAULT_CONFIG

  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored) {
      return readableConfig(JSON.parse(stored))
    }
    const previous = PREVIOUS_STORAGE_KEY && localStorage.getItem(PREVIOUS_STORAGE_KEY)
    const legacy = previous || localStorage.getItem(LEGACY_STORAGE_KEY)
    if (legacy) {
      const migrated = readableConfig(JSON.parse(legacy))
      localStorage.setItem(STORAGE_KEY, JSON.stringify(migrated))
      if (!previous) localStorage.removeItem(LEGACY_STORAGE_KEY)
      return migrated
    }
  } catch {}

  return DEFAULT_CONFIG
}

// A theme that fixes its scheme sets every colour itself: the reader's accent
// and surface stay stored but add nothing on top of it.
const NO_COLOR_LAYER = { light: {}, dark: {} }

function colorLayers(theme: ResolvedPresetTheme, customization: ThemeCustomization) {
  if (theme.scheme) return { color: NO_COLOR_LAYER, surface: NO_COLOR_LAYER }
  return {
    color: getColorOption(customization.colorId),
    surface: getSurfaceColorOption(customization.surfaceColorId),
  }
}

function themeToCss(
  theme: ResolvedPresetTheme,
  radius: string,
  customization: ThemeCustomization,
  colorOverrides: ThemeColorOverrides = {}
) {
  const font = getFontOption(customization.fontId)
  const { color, surface } = colorLayers(theme, customization)
  const style = getCustomizationStyle(customization)
  const toVars = (vars: object) =>
    Object.entries(vars)
      .map(([key, value]) => `  ${key}: ${String(value)};`)
      .join("\n")

  // Keep light-only overrides out of the dark cascade.
  return `
    :root {
${toVars(theme.light)}
${toVars(surface.light)}
${toVars(color.light)}
${toVars(theme.style)}
${toVars(style)}
${toVars(font.style)}
      --radius: ${radius};
    }
    .dark {
${toVars(theme.dark)}
${toVars(surface.dark)}
${toVars(color.dark)}
    }
    :root:not(.dark) {
${toVars(colorOverrides[theme.scheme ?? "light"] ?? {})}
    }
    .dark {
${toVars(colorOverrides[theme.scheme ?? "dark"] ?? {})}
    }
${SHELL_THEME_CSS}
  `
}

function configToCss(config: ThemeConfig) {
  const normalized = normalizeConfig(config)
  const preset = getPreset(normalized.presetId)
  const options = getPresetOptions(normalized, preset.id)
  const theme = resolvePresetTheme(preset, options)
  const radius = getRadius(normalized.radiusIndex)

  return themeToCss(theme, radius.value, normalized.customization, normalized.colorOverrides)
}

const DEFAULT_THEME_CSS = configToCss(DEFAULT_CONFIG)

// Marks <html> with the applied preset, and with its scheme when the theme is
// only light or only dark. For a fixed scheme it sets the class itself, so the
// page is right before the theme provider catches up with the event. The
// bootstrap script carries a copy of this.
function markScheme(presetId: string, scheme: ResolvedPresetTheme["scheme"]) {
  const root = document.documentElement
  root.dataset.foliohPreset = presetId
  if (scheme) {
    root.dataset.foliohScheme = scheme
    root.classList.remove("light", "dark")
    root.classList.add(scheme)
    root.style.colorScheme = scheme
  } else {
    delete root.dataset.foliohScheme
  }
  window.dispatchEvent(new Event(THEME_SCHEME_EVENT))
}

function applyConfig(config: ThemeConfig) {
  // The root layout mounts the style element; a theme package's page may
  // still mount a second one. Every copy gets the same CSS, so a later copy
  // never paints the default theme over the reader's.
  const styles = Array.from(document.querySelectorAll("style#theme-configurator-style"))
  if (styles.length === 0) {
    const el = document.createElement("style")
    el.id = "theme-configurator-style"
    document.head.appendChild(el)
    styles.push(el)
  }

  const css = configToCss(config)
  styles.forEach((style) => {
    style.textContent = css
  })
  const normalized = normalizeConfig(config)
  const preset = getPreset(normalized.presetId)
  markScheme(preset.id, resolvePresetTheme(preset, getPresetOptions(normalized, preset.id)).scheme)
}

// The gallery (theme-gallery.tsx) saves the theme a reader applies and fires
// THEME_CONFIG_EVENT, so every mounted reader of the config stays in step.
export const THEME_CONFIG_EVENT = "folioh:theme-config"

export type { ThemeConfig }

export function readThemeConfig(): ThemeConfig {
  return loadConfig()
}

let themeTransition: ViewTransition | undefined
let themeUpdate = 0

export function saveThemeConfig(config: ThemeConfig) {
  const previous = readThemeConfig()
  const next = readableConfig(config)
  const update = ++themeUpdate
  themeTransition?.skipTransition()
  document.documentElement.classList.remove("folioh-theme-transition")
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
  } catch {}
  const apply = () => {
    if (update !== themeUpdate) return
    applyConfig(next)
    window.dispatchEvent(new Event(THEME_CONFIG_EVENT))
  }
  const changingPalette = next.presetId === "omarchy" && (
    previous.presetId !== "omarchy" || previous.optionsByPreset.omarchy?.palette !== next.optionsByPreset.omarchy?.palette
  )
  if (changingPalette && document.startViewTransition && !document.hidden && !window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    document.documentElement.classList.add("folioh-theme-transition")
    try {
      const transition = document.startViewTransition(apply)
      themeTransition = transition
      void transition.ready.catch(() => {})
      void transition.finished.finally(() => {
        if (themeTransition === transition) {
          themeTransition = undefined
          document.documentElement.classList.remove("folioh-theme-transition")
        }
      }).catch(() => {})
    } catch {
      document.documentElement.classList.remove("folioh-theme-transition")
      apply()
    }
  } else {
    apply()
  }
}

// Restyles the page with a config without saving it or telling anyone: the
// picker previews a draft this way and restores the saved config after.
export function previewThemeConfig(config: ThemeConfig) {
  ++themeUpdate
  themeTransition?.skipTransition()
  document.documentElement.classList.remove("folioh-theme-transition")
  applyConfig(readableConfig(config))
}

// The config a reader gets by picking a preset with the given options, or with
// the options they last stored for it. On the preset they already use it
// changes the options and keeps their customization; the corners snap only
// when the options change the preset's own radius. Another preset starts from
// its own defaults; the site preset starts from the site's defaults.
export function configForPreset(
  config: ThemeConfig,
  presetId: string,
  options?: PresetOptionValues
): ThemeConfig {
  const preset = getPreset(presetId)
  const isSitePreset = preset.id === DEFAULT_CONFIG.presetId
  const nextOptions = normalizePresetOptions(
    preset,
    options ??
      config.optionsByPreset[preset.id] ??
      (isSitePreset ? DEFAULT_CONFIG.optionsByPreset[preset.id] : undefined)
  )
  const optionsByPreset = { ...config.optionsByPreset, [preset.id]: nextOptions }

  if (preset.id === getPreset(config.presetId).id) {
    const currentOptions = getPresetOptions(config, preset.id)
    if (getPresetOptionKey(nextOptions) === getPresetOptionKey(currentOptions)) {
      return normalizeConfig(config)
    }
    const radius = resolvePresetTheme(preset, nextOptions).radius
    const radiusIndex =
      radius === resolvePresetTheme(preset, currentOptions).radius
        ? config.radiusIndex
        : getRadiusIndex(radius, config.radiusIndex)
    return normalizeConfig({ ...config, radiusIndex, optionsByPreset })
  }

  const defaults = isSitePreset
    ? { radiusIndex: DEFAULT_CONFIG.radiusIndex, customization: DEFAULT_CONFIG.customization }
    : getPresetDefaults(preset)
  return normalizeConfig({
    ...config,
    presetId: preset.id,
    radiusIndex: defaults.radiusIndex,
    optionsByPreset,
    customization: defaults.customization,
    colorOverrides: undefined,
  })
}

// The custom properties a config puts on the page in one scheme, in the order
// themeToCss writes them, so a preview can scope them to its own element.
export function configVars(config: ThemeConfig, dark: boolean): Record<string, string | undefined> {
  const normalized = normalizeConfig(config)
  const preset = getPreset(normalized.presetId)
  const theme = resolvePresetTheme(preset, getPresetOptions(normalized, preset.id))
  const customization = normalized.customization
  const { color, surface } = colorLayers(theme, customization)
  const isDark = theme.scheme ? theme.scheme === "dark" : dark
  const vars: Record<string, string | undefined> = {
    ...theme.light,
    ...surface.light,
    ...color.light,
    ...theme.style,
    ...getCustomizationStyle(customization),
    ...getFontOption(customization.fontId).style,
    "--radius": getRadius(normalized.radiusIndex).value,
  }

  const resolved = isDark ? { ...vars, ...theme.dark, ...surface.dark, ...color.dark } : vars
  return { ...resolved, ...normalized.colorOverrides?.[isDark ? "dark" : "light"] }
}

// Combination-capped bootstrap payload; buildBootstrapPresets (preset-types.ts)
// embeds only the default resolution for presets whose option-combination
// count exceeds MAX_BOOTSTRAP_COMBINATIONS so the inline script stays small.
const BOOTSTRAP_PRESETS = buildBootstrapPresets(SELECTABLE_PRESETS)

const BOOTSTRAP_FONT_OPTIONS = fontOptions.map((option) => ({
  id: option.id,
  style: option.style,
}))

const BOOTSTRAP_COLOR_OPTIONS = colorOptions.map((option) => ({
  id: option.id,
  light: option.light,
  dark: option.dark,
}))

const BOOTSTRAP_SURFACE_COLOR_OPTIONS = surfaceColorOptions.map((option) => ({
  id: option.id,
  light: option.light,
  dark: option.dark,
}))

const BOOTSTRAP_SHELL_PADDING_OPTIONS = shellPaddingOptions.map((option) => ({
  id: option.id,
  style: option.style,
}))

const BOOTSTRAP_CONTENT_WIDTH_OPTIONS = contentWidthOptions.map((option) => ({
  id: option.id,
  style: option.style,
}))

const BOOTSTRAP_RHYTHM_OPTIONS = rhythmOptions.map((option) => ({
  id: option.id,
  style: option.style,
}))

const BOOTSTRAP_BORDER_OPTIONS = borderOptions.map((option) => ({
  id: option.id,
  style: option.style,
}))

const BOOTSTRAP_CODE_TREATMENT_OPTIONS = codeTreatmentOptions.map((option) => ({
  id: option.id,
  style: option.style,
}))

const THEME_BOOTSTRAP_SCRIPT = `
(() => {
  try {
    const presets = ${JSON.stringify(BOOTSTRAP_PRESETS)};
    const radiusOptions = ${JSON.stringify(radiusOptions)};
    const fontOptions = ${JSON.stringify(BOOTSTRAP_FONT_OPTIONS)};
    const colorOptions = ${JSON.stringify(BOOTSTRAP_COLOR_OPTIONS)};
    const surfaceColorOptions = ${JSON.stringify(BOOTSTRAP_SURFACE_COLOR_OPTIONS)};
    const shellPaddingOptions = ${JSON.stringify(BOOTSTRAP_SHELL_PADDING_OPTIONS)};
    const contentWidthOptions = ${JSON.stringify(BOOTSTRAP_CONTENT_WIDTH_OPTIONS)};
    const rhythmOptions = ${JSON.stringify(BOOTSTRAP_RHYTHM_OPTIONS)};
    const borderOptions = ${JSON.stringify(BOOTSTRAP_BORDER_OPTIONS)};
    const codeTreatmentOptions = ${JSON.stringify(BOOTSTRAP_CODE_TREATMENT_OPTIONS)};
    const defaultConfig = ${JSON.stringify(DEFAULT_CONFIG)};
    const legacyPresetIds = ${JSON.stringify(LEGACY_PRESET_IDS)};
    const shellThemeCss = ${JSON.stringify(SHELL_THEME_CSS)};
    const toVars = (vars) => Object.entries(vars).map(([key, value]) => "  " + key + ": " + value + ";").join("\\n");
    const normalizePresetId = (id) => !id || id === "custom" ? defaultConfig.presetId : legacyPresetIds[id] || id;
    const getPreset = (id) => {
      const migratedId = normalizePresetId(id);
      return presets.find((item) => item.id === migratedId) || presets.find((item) => item.id === defaultConfig.presetId) || presets[0];
    };
    const getRadius = (index) => radiusOptions[index] || radiusOptions[defaultConfig.radiusIndex];
    const getFontOption = (id) => fontOptions.find((item) => item.id === id) || fontOptions.find((item) => item.id === defaultConfig.customization.fontId) || fontOptions[0];
    const getColorOption = (id) => colorOptions.find((item) => item.id === id) || colorOptions.find((item) => item.id === defaultConfig.customization.colorId) || colorOptions[0];
    const getSurfaceColorOption = (id) => surfaceColorOptions.find((item) => item.id === id) || surfaceColorOptions[0];
    const getShellPaddingOption = (id) => shellPaddingOptions.find((item) => item.id === id) || shellPaddingOptions[0];
    const getContentWidthOption = (id) => contentWidthOptions.find((item) => item.id === id) || contentWidthOptions[0];
    const getRhythmOption = (id) => rhythmOptions.find((item) => item.id === id) || rhythmOptions[0];
    const getBorderOption = (id) => borderOptions.find((item) => item.id === id) || borderOptions[0];
    const getCodeTreatmentOption = (id) => codeTreatmentOptions.find((item) => item.id === id) || codeTreatmentOptions[0];
    const getCustomizationStyle = (customization) => ({
      ...getShellPaddingOption(customization.shellPaddingId).style,
      ...getContentWidthOption(customization.contentWidthId).style,
      ...getRhythmOption(customization.rhythmId).style,
      ...getBorderOption(customization.borderId).style,
      ...getCodeTreatmentOption(customization.codeTreatmentId).style,
    });
    const normalizeCustomization = (raw = {}) => {
      const current = raw && typeof raw === "object" ? raw : {};
      return {
        fontId: getFontOption(current.fontId).id,
        colorId: getColorOption(current.colorId).id,
        surfaceColorId: getSurfaceColorOption(current.surfaceColorId).id,
        shellPaddingId: getShellPaddingOption(current.shellPaddingId).id,
        contentWidthId: getContentWidthOption(current.contentWidthId).id,
        rhythmId: getRhythmOption(current.rhythmId).id,
        borderId: getBorderOption(current.borderId).id,
        codeTreatmentId: getCodeTreatmentOption(current.codeTreatmentId).id,
      };
    };
    const getPresetDefaults = (preset) => {
      const radiusIndex = radiusOptions[preset.defaultRadiusIndex ?? defaultConfig.radiusIndex]
        ? preset.defaultRadiusIndex ?? defaultConfig.radiusIndex
        : defaultConfig.radiusIndex;
      return {
        radiusIndex,
        customization: normalizeCustomization(preset.defaultCustomization || defaultConfig.customization),
      };
    };
    const getRequestedPresetId = (raw = {}) => {
      if (raw.presetId && raw.presetId !== "custom") return raw.presetId;
      return raw.flavorId || raw.themeId || raw.presetId;
    };
    const getOptionKey = (options) => Object.keys(options).sort().map((key) => key + ":" + options[key]).join("|");
    const normalizeOptions = (preset, rawOptions) => {
      const normalized = { ...preset.defaultOptions };
      const current = rawOptions && typeof rawOptions === "object" ? rawOptions : {};
      preset.controls.forEach((control) => {
        const requested = current[control.id];
        normalized[control.id] = control.values.includes(requested) ? requested : normalized[control.id];
      });
      return normalized;
    };
    const getMigratedOptions = (raw = {}) => {
      const optionsByPreset = raw.optionsByPreset && typeof raw.optionsByPreset === "object" ? { ...raw.optionsByPreset } : {};
      const legacyOptions = raw.optionsByFlavor && typeof raw.optionsByFlavor === "object" ? raw.optionsByFlavor : {};
      Object.entries(legacyOptions).forEach(([legacyId, options]) => {
        const preset = getPreset(legacyId);
        if (!optionsByPreset[preset.id]) {
          optionsByPreset[preset.id] = options;
        }
      });
      return optionsByPreset;
    };
    const normalizeColorOverrides = (raw) => {
      if (!raw || typeof raw !== "object") return undefined;
      const result = {};
      for (const mode of ["light", "dark"]) {
        const values = raw[mode];
        if (!values || typeof values !== "object") continue;
        for (const token of ["--background", "--foreground", "--primary"]) {
          const value = values[token];
          if (typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value)) {
            const colors = (result[mode] ??= {});
            colors[token] = value;
          }
        }
      }
      return Object.keys(result).length ? result : undefined;
    };
    const normalizeConfig = (raw = {}) => {
      const requestedId = getRequestedPresetId(raw);
      const preset = getPreset(requestedId === "custom" ? raw.flavorId || raw.themeId : requestedId);
      const defaults = getPresetDefaults(preset);
      const optionsByPreset = getMigratedOptions(raw);
      if (requestedId === "pastel" && !optionsByPreset.pastel?.palette) {
        optionsByPreset.pastel = { ...optionsByPreset.pastel, palette: "jade" };
      }
      const requestedRadiusIndex = Number.isInteger(raw.radiusIndex) ? raw.radiusIndex : defaults.radiusIndex;
      if (!optionsByPreset[preset.id]) {
        optionsByPreset[preset.id] = normalizeOptions(preset);
      }
      return {
        presetId: preset.id,
        radiusIndex: radiusOptions[requestedRadiusIndex] ? requestedRadiusIndex : defaults.radiusIndex,
        optionsByPreset,
        customization: normalizeCustomization(raw.customization || defaults.customization),
        colorOverrides: normalizeColorOverrides(raw.colorOverrides),
      };
    };
    const readableConfig = (raw) => {
      const id = normalizePresetId(getRequestedPresetId(raw));
      return presets.some((preset) => preset.id === id) ? normalizeConfig(raw) : normalizeConfig(defaultConfig);
    };
    const readConfig = () => {
      try {
        const stored = localStorage.getItem("${STORAGE_KEY}");
        if (stored) {
          return readableConfig(JSON.parse(stored));
        }
        const previousKey = ${JSON.stringify(PREVIOUS_STORAGE_KEY ?? null)};
        const previous = previousKey && localStorage.getItem(previousKey);
        const legacy = previous || localStorage.getItem("${LEGACY_STORAGE_KEY}");
        if (legacy) {
          const migrated = readableConfig(JSON.parse(legacy));
          localStorage.setItem("${STORAGE_KEY}", JSON.stringify(migrated));
          if (!previous) localStorage.removeItem("${LEGACY_STORAGE_KEY}");
          return migrated;
        }
        return normalizeConfig(defaultConfig);
      } catch {
        return normalizeConfig(defaultConfig);
      }
    };
    const getTheme = (preset, options) => preset.themes[getOptionKey(options)] || preset.themes[preset.defaultKey];
    const markScheme = (presetId, scheme) => {
      const root = document.documentElement;
      root.dataset.foliohPreset = presetId;
      if (scheme) {
        root.dataset.foliohScheme = scheme;
        root.classList.remove("light", "dark");
        root.classList.add(scheme);
        root.style.colorScheme = scheme;
      } else {
        delete root.dataset.foliohScheme;
      }
      window.dispatchEvent(new Event("${THEME_SCHEME_EVENT}"));
    };
    const getStyleElements = () => {
      const elements = Array.from(document.querySelectorAll("style#theme-configurator-style"));
      if (elements.length === 0) {
        const element = document.createElement("style");
        element.id = "theme-configurator-style";
        document.head.appendChild(element);
        elements.push(element);
      }
      return elements;
    };
    const apply = (rawConfig) => {
      const config = normalizeConfig(rawConfig);
      const preset = getPreset(config.presetId);
      const options = normalizeOptions(preset, config.optionsByPreset[preset.id]);
      const theme = getTheme(preset, options);
      const radius = getRadius(config.radiusIndex);
      const font = getFontOption(config.customization.fontId);
      // A theme that fixes its scheme sets every colour itself, as in
      // colorLayers().
      const noLayer = { light: {}, dark: {} };
      const color = theme.scheme ? noLayer : getColorOption(config.customization.colorId);
      const surface = theme.scheme ? noLayer : getSurfaceColorOption(config.customization.surfaceColorId);
      const style = getCustomizationStyle(config.customization);
      const overrides = config.colorOverrides || {};
      // Keep this format byte-identical to themeToCss(): the bootstrap runs
      // before hydration and rewrites the server-rendered style element, so a
      // format drift would make every page load a hydration mismatch even for
      // the default config.
      const css = "\\n    :root {\\n" + toVars(theme.light) + "\\n" + toVars(surface.light) + "\\n" + toVars(color.light) + "\\n" + toVars(theme.style) + "\\n" + toVars(style) + "\\n" + toVars(font.style) + "\\n      --radius: " + radius.value + ";\\n    }\\n    .dark {\\n" + toVars(theme.dark) + "\\n" + toVars(surface.dark) + "\\n" + toVars(color.dark) + "\\n    }\\n    :root:not(.dark) {\\n" + toVars(overrides[theme.scheme || "light"] || {}) + "\\n    }\\n    .dark {\\n" + toVars(overrides[theme.scheme || "dark"] || {}) + "\\n    }\\n" + shellThemeCss + "\\n  ";
      getStyleElements().forEach((element) => {
        element.textContent = css;
      });
      markScheme(preset.id, theme.scheme);
    };
    apply(readConfig());
  } catch {}
})();
`

// Every theme control lives in the theme picker (theme-gallery.tsx). This
// renders nothing and stays exported for one release, so a theme package
// layout that still mounts it keeps building.
export function ThemeConfigurator() {
  return null
}

/**
 * The saved-reader-theme bootstrap: the default theme CSS plus the
 * pre-hydration script that rewrites it to whatever the reader stored. The
 * root layout mounts it once, so every route shows the same theme. React does
 * not run an inline script it inserts on the client, so a copy mounted by a
 * client-side navigation (a theme package's page may still carry one) applies
 * the stored config in a layout effect, before paint. Idempotent: every copy
 * re-applies the same stored config.
 */
export function ThemeStyleBootstrap() {
  useLayoutEffect(() => {
    applyConfig(loadConfig())
  }, [])

  return (
    <>
      {/* The bootstrap script rewrites this element's text before hydration
          (to the reader's saved theme), so the hydrated innerHTML legitimately
          differs from DEFAULT_THEME_CSS whenever a non-default config is
          stored. Suppress the mismatch so React never "corrects" the element
          back to the default CSS mid-load. */}
      <style
        id="theme-configurator-style"
        suppressHydrationWarning
        dangerouslySetInnerHTML={{ __html: DEFAULT_THEME_CSS }}
      />
      <script
        id="theme-configurator-boot"
        dangerouslySetInnerHTML={{ __html: THEME_BOOTSTRAP_SCRIPT }}
      />
    </>
  )
}
