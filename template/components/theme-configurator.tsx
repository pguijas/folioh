"use client"

import { useLayoutEffect } from "react"
import { applyConfig, DEFAULT_THEME_CSS, loadConfig, THEME_BOOTSTRAP_SRC } from "@/theme/theme-config"

// Keep the existing import surface for controls and theme packages.
export {
  radiusOptions,
  fontOptions,
  colorOptions,
  surfaceColorOptions,
  shellPaddingOptions,
  rhythmOptions,
  borderOptions,
  codeTreatmentOptions,
  SELECTABLE_PRESETS,
  DEFAULT_CONFIG,
  getPresetDefaults,
  THEME_CONFIG_EVENT,
  readThemeConfig,
  saveThemeConfig,
  previewThemeConfig,
  configForPreset,
  configVars,
} from "@/theme/theme-config"
export type { ThemeConfig, ThemeColorOverrides } from "@/theme/theme-config"

// Every theme control lives in the theme picker (theme-gallery.tsx). This
// renders nothing and stays exported for one release, so a theme package
// layout that still mounts it keeps building.
export function ThemeConfigurator() {
  return null
}

/**
 * The saved-reader-theme bootstrap: the default theme CSS plus the
 * shared pre-hydration script that applies whatever the reader stored. The
 * root layout mounts it once, so every route shows the same theme. React does
 * not run a script it inserts on the client, so a copy mounted by a
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
      {/* Apply saved reader preferences before content or hydration paints. */}
      {/* eslint-disable-next-line @next/next/no-sync-scripts */}
      <script
        id="theme-configurator-boot"
        suppressHydrationWarning
        src={THEME_BOOTSTRAP_SRC}
      />
    </>
  )
}
