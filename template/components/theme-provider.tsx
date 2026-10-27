"use client"

import * as React from "react"
import { ThemeProvider as NextThemesProvider, useTheme } from "next-themes"

import { THEME_SCHEME_EVENT } from "@/lib/theme-events"
export { THEME_SCHEME_EVENT } from "@/lib/theme-events"

import { switchScheme } from "@/lib/scheme-transition"

// `theme.dark_mode: false` in docs.yaml sets this to false: the site stays
// light unless the reader applies a dark-only palette, and the mode controls,
// the toggles and the `d` shortcut go away.
export const darkModeEnabled: boolean = true // __FOLIOH_DARK_MODE__

// A theme that is only light or only dark (each Omarchy palette) marks <html>
// with data-folioh-scheme when it is applied, and fires this event whenever the
// applied theme changes. The provider forces the marked scheme on next-themes,
// which hands the reader's own light/dark choice back once the mark is gone.


type FixedScheme = "light" | "dark" | undefined

function readFixedScheme(): FixedScheme {
  const scheme = document.documentElement.dataset.foliohScheme
  return scheme === "light" || scheme === "dark" ? scheme : undefined
}

function isSchemeFixed() {
  return readFixedScheme() !== undefined
}

// Undefined on the server and in the first client render, so hydration sees
// what the server rendered; the layout effect picks up the mark before paint.
function useFixedScheme(): FixedScheme {
  const [scheme, setScheme] = React.useState<FixedScheme>(undefined)

  React.useLayoutEffect(() => {
    const sync = () => setScheme(readFixedScheme())
    sync()
    window.addEventListener(THEME_SCHEME_EVENT, sync)
    return () => window.removeEventListener(THEME_SCHEME_EVENT, sync)
  }, [])

  return scheme
}

function ThemeProvider({
  children,
  ...props
}: React.ComponentProps<typeof NextThemesProvider>) {
  const fixedScheme = useFixedScheme()

  if (!darkModeEnabled) {
    // A storage key of its own, so a "dark" saved while dark mode was on
    // cannot leak into resolvedTheme. A dark palette the reader picked still
    // wins: its colours are dark whatever the class says, and code blocks and
    // `dark:` styles follow the class. Both schemes stay in `themes` so the
    // class a dark palette set is removed when the reader leaves it.
    return (
      <NextThemesProvider
        attribute="class"
        disableTransitionOnChange
        {...props}
        forcedTheme={fixedScheme ?? "light"}
        defaultTheme="light"
        enableSystem={false}
        themes={["light", "dark"]}
        storageKey="folioh-theme-light"
      >
        {children}
      </NextThemesProvider>
    )
  }
  return (
    <NextThemesProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
      {...props}
      forcedTheme={fixedScheme ?? props.forcedTheme}
    >
      <ThemeHotkey />
      {children}
    </NextThemesProvider>
  )
}

// Inputs a reader does not type into: a letter key pressed on one of them is
// still a shortcut, so the theme picker's radios keep `d` working.
const NON_TEXT_INPUTS = new Set(["button", "checkbox", "radio", "range", "reset", "submit"])

function isTypingTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false
  }

  if (target instanceof HTMLInputElement) {
    return !NON_TEXT_INPUTS.has(target.type)
  }

  return (
    target.isContentEditable ||
    target.tagName === "TEXTAREA" ||
    target.tagName === "SELECT"
  )
}

function ThemeHotkey() {
  const { resolvedTheme, setTheme } = useTheme()

  React.useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.defaultPrevented || event.repeat) {
        return
      }

      if (event.metaKey || event.ctrlKey || event.altKey) {
        return
      }

      if (event.key.toLowerCase() !== "d") {
        return
      }

      if (isTypingTarget(event.target) || isSchemeFixed()) {
        return
      }

      switchScheme(setTheme, resolvedTheme === "dark" ? "light" : "dark")
    }

    window.addEventListener("keydown", onKeyDown)

    return () => {
      window.removeEventListener("keydown", onKeyDown)
    }
  }, [resolvedTheme, setTheme])

  return null
}

export { ThemeProvider, isTypingTarget }
