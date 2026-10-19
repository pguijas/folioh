import { projectIconSvg } from "@/theme/project-icon"
import { ImageResponse } from "next/og"
import { isPastelSocialImage, PastelSocialImage } from "@/theme/social-image"

export const dynamic = "force-static"
export const size = { width: 1200, height: 630 }
export const contentType = "image/png"

function formatTitle(mdxPath?: string[]): string {
  if (!mdxPath || mdxPath.length === 0) return "Documentation"

  const last = mdxPath[mdxPath.length - 1]
  return last
    .replace(/[-_]/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase())
}

function formatBreadcrumb(mdxPath?: string[]): string {
  if (!mdxPath || mdxPath.length <= 1) return ""
  return mdxPath
    .slice(0, -1)
    .map((seg) =>
      seg
        .replace(/[-_]/g, " ")
        .replace(/\b\w/g, (c) => c.toUpperCase())
    )
    .join(" / ")
}

export default async function OGImage({
  params,
}: {
  params?: Promise<{ mdxPath?: string[] }>
}) {
  const resolvedParams = params ? await params : {}
  const mdxPath = (resolvedParams as Record<string, unknown>)?.mdxPath as string[] | undefined
  const title = formatTitle(mdxPath)
  const breadcrumb = formatBreadcrumb(mdxPath)

  if (isPastelSocialImage) {
    return new ImageResponse(
      <PastelSocialImage projectName="__PROJECT_NAME__" title={title} breadcrumb={breadcrumb} />,
      size
    )
  }

  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          padding: "72px 80px",
          background: "rgb(247, 245, 239)",
          color: "rgb(35, 34, 31)",
          fontFamily: "system-ui, sans-serif",
        }}
      >
        {/* Top: breadcrumb */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "16px",
          }}
        >
          {breadcrumb && (
            <div
              style={{
                fontSize: 22,
                color: "rgb(100, 96, 86)",
                letterSpacing: 0,
              }}
            >
              {breadcrumb}
            </div>
          )}
          {/* Title */}
          <div
            style={{
              fontSize: title.length > 40 ? 48 : 60,
              fontWeight: 700,
              color: "rgb(35, 34, 31)",
              lineHeight: 1.2,
              letterSpacing: 0,
              maxWidth: "900px",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            {title}
          </div>
        </div>

        {/* Bottom: branding bar */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "14px",
            }}
          >
            {/* Monogram box */}
            {projectIconSvg && /<svg\b[^>]*\bdata-folioh-icon="pastel"/.test(projectIconSvg) ? (
              <img width={40} height={40} alt="" src={"data:image/svg+xml," + encodeURIComponent(projectIconSvg)} />
            ) : (
              <div
                style={{
                  width: "40px",
                  height: "40px",
                  borderRadius: "8px",
                  border: "2px solid rgb(35, 34, 31)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontSize: 16,
                  fontWeight: 700,
                  color: "rgb(35, 34, 31)",
                  letterSpacing: 0,
                }}
              >
                __PROJECT_MONOGRAM__
              </div>
            )}
            <div
              style={{
                fontSize: 26,
                fontWeight: 600,
                color: "rgb(35, 34, 31)",
                letterSpacing: 0,
              }}
            >
              __PROJECT_NAME__
            </div>
          </div>
          <div
            style={{
              fontSize: 18,
              color: "rgb(100, 96, 86)",
              letterSpacing: 0,
              textTransform: "uppercase" as const,
            }}
          >
            Documentation
          </div>
        </div>
      </div>
    ),
    { ...size }
  )
}
