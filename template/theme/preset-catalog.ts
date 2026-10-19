import { presets } from "./presets"
import { builtinGalleryVariants } from "./preset-registry"

export const extraPresetsEnabled: boolean = false // __FOLIOH_EXTRA_PRESETS__

// Keep every recipe resolvable; the reader's catalog is a separate choice.
export function selectablePresets(sitePresetId: string) {
  return presets.filter((preset) =>
    extraPresetsEnabled || preset.id === "pastel" || preset.id === sitePresetId ||
    !builtinGalleryVariants.has(preset)
  )
}
