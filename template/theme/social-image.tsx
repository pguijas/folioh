/* eslint-disable @next/next/no-img-element -- ImageResponse embeds the SVG directly in its PNG. */
import { projectIconSvg } from "./project-icon"
import { projectThemeDefaultConfig } from "./project-theme"
import { pastelPreset } from "./presets"
import { pastelShapes } from "./pastel-shapes"

export const isPastelSocialImage = Boolean(
  projectIconSvg &&
  /<svg\b[^>]*\bdata-folioh-icon="pastel"/.test(projectIconSvg)
)

// Theme packages may export a minimal, untyped defaults object.
const packageDefaults: {
  presetId?: string
  radiusIndex?: number
  optionsByPreset?: Record<string, Record<string, string>>
  customization?: Record<string, string>
} = projectThemeDefaultConfig
const colors = pastelPreset.resolve(
  packageDefaults.optionsByPreset?.pastel ?? pastelPreset.defaultOptions
).light

export function PastelSocialImage({
  projectName,
  title,
  breadcrumb,
}: {
  projectName: string
  title: string
  breadcrumb?: string
}) {
  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        position: "relative",
        flexDirection: "column",
        justifyContent: "space-between",
        padding: "72px 80px",
        background: colors["--background"],
        color: colors["--foreground"],
        fontFamily: "system-ui, sans-serif",
      }}
    >
      {pastelShapes("social").map((shape, index) => (
        <svg
          key={index}
          viewBox="0 0 100 100"
          width={[310, 250, 180][index]}
          height={[310, 250, 180][index]}
          style={{
            position: "absolute",
            left: [805, 785, 990][index],
            top: [120, 310, 290][index],
            color: colors[`--chart-${index + 1}`],
          }}
        >
          <path d={shape.from} fill="currentColor" />
        </svg>
      ))}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 18,
          fontSize: 34,
          fontWeight: 700,
        }}
      >
        <img
          width={64}
          height={64}
          alt=""
          src={"data:image/svg+xml," + encodeURIComponent(projectIconSvg!)}
        />
        {projectName}
      </div>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 22,
          maxWidth: 700,
        }}
      >
        {breadcrumb ? (
          <div style={{ fontSize: 24, color: colors["--muted-foreground"] }}>
            {breadcrumb}
          </div>
        ) : null}
        <div
          style={{
            fontSize: title.length > 48 ? 52 : 72,
            fontWeight: 500,
            lineHeight: 1.1,
            letterSpacing: "-0.02em",
          }}
        >
          {title}
        </div>
      </div>
    </div>
  )
}
