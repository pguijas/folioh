import { THEME_BOOTSTRAP_FILENAME, THEME_BOOTSTRAP_SCRIPT } from "@/theme/theme-config"

export const dynamic = "force-static"
export const dynamicParams = false

export function generateStaticParams() {
  return [{ file: THEME_BOOTSTRAP_FILENAME }]
}

export function GET() {
  return new Response(THEME_BOOTSTRAP_SCRIPT, {
    headers: {
      "Content-Type": "text/javascript; charset=utf-8",
      "Cache-Control": "public, max-age=31536000, immutable",
    },
  })
}
