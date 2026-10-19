import { projectIconSvg } from "@/theme/project-icon"
import { ImageResponse } from "next/og"
import { isPastelSocialImage, PastelSocialImage } from "@/theme/social-image"

export const dynamic = "force-static"
export const size = { width: 1200, height: 630 }
export const contentType = "image/png"

export default function OGImage() {
  if (isPastelSocialImage) {
    return new ImageResponse(
      <PastelSocialImage projectName="__PROJECT_NAME__" title="__PROJECT_DESCRIPTION__" />,
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
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "18px",
            fontSize: 30,
            fontWeight: 700,
            letterSpacing: 0,
          }}
        >
          {projectIconSvg && /<svg\b[^>]*\bdata-folioh-icon="pastel"/.test(projectIconSvg) ? (
            <img width={54} height={54} alt="" src={"data:image/svg+xml," + encodeURIComponent(projectIconSvg)} />
          ) : (
            <div
              style={{
                width: "54px",
                height: "54px",
                borderRadius: "10px",
                border: "2px solid rgb(35, 34, 31)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: 18,
                fontWeight: 800,
              }}
            >
              __PROJECT_MONOGRAM__
            </div>
          )}
          __PROJECT_NAME__
        </div>

        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "24px",
          }}
        >
          <div
            style={{
              maxWidth: "860px",
              fontSize: 68,
              lineHeight: 1.05,
              fontWeight: 750,
              letterSpacing: 0,
            }}
          >
            __PROJECT_DESCRIPTION__
          </div>
        </div>
      </div>
    ),
    size
  )
}
