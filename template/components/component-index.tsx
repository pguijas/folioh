import { docsRoute } from "@/lib/docs-routes"
import { ComponentGalleryPreview } from "./component-gallery-preview"
import { ComponentIndexPreview } from "./component-index-previews"

type ComponentEntry = {
  title: string
  description: string
  href: string
  role: string
}

type ComponentGroup = {
  title: string
  summary: string
  accent: string
  entries: ComponentEntry[]
}

const componentGroups: ComponentGroup[] = [
  {
    title: "Workflow Components",
    summary:
      "Build setup guides, CLI references, migration notes, and plugin docs that stay easy to scan.",
    accent: "var(--primary)",
    entries: [
      {
        title: "TerminalSession",
        description:
          "Command and output blocks with prompt, status, and working directory context.",
        href: docsRoute("components/terminal-session"),
        role: "Commands",
      },
      {
        title: "ConfigPanel",
        description:
          "Annotated configuration snippets for docs.yaml examples.",
        href: docsRoute("components/config-panel"),
        role: "Config",
      },
      {
        title: "BuildArtifact",
        description:
          "Compact summaries of generated files, folders, and routes.",
        href: docsRoute("components/build-artifact"),
        role: "Output",
      },
      {
        title: "CommandGrid",
        description: "Scannable CLI command cards with common flags.",
        href: docsRoute("components/command-grid"),
        role: "CLI",
      },
      {
        title: "BeforeAfter",
        description:
          "Side-by-side migration, config, and output comparisons.",
        href: docsRoute("components/before-after"),
        role: "Compare",
      },
      {
        title: "Checklist",
        description:
          "Readiness and deployment checklists with explicit states.",
        href: docsRoute("components/checklist"),
        role: "State",
      },
      {
        title: "HookMap",
        description:
          "Plugin lifecycle stages with hook names and descriptions.",
        href: docsRoute("components/hook-map"),
        role: "Plugins",
      },
    ],
  },
  {
    title: "Interactive Components",
    summary:
      "Add controlled disclosure, alternate examples, and sequence structure without custom page logic.",
    accent: "var(--chart-5)",
    entries: [
      {
        title: "Accordion",
        description:
          "Collapsible content sections for FAQs and grouped information.",
        href: docsRoute("components/accordion"),
        role: "Reveal",
      },
      {
        title: "Tabs",
        description: "Tabbed panels for organizing alternative content views.",
        href: docsRoute("components/tabs"),
        role: "Views",
      },
      {
        title: "Steps",
        description:
          "Numbered step-by-step instructions with a vertical timeline.",
        href: docsRoute("components/steps"),
        role: "Sequence",
      },
      {
        title: "Timeline",
        description: "Vertical timeline for changelogs and version history.",
        href: docsRoute("components/timeline"),
        role: "History",
      },
      {
        title: "CodeGroup",
        description:
          "Tabbed code blocks for multi-language or multi-variant examples.",
        href: docsRoute("components/code-group"),
        role: "Code",
      },
      {
        title: "PreviewCode",
        description:
          "Paired rendered previews and source snippets for component examples.",
        href: docsRoute("components/preview-code"),
        role: "Examples",
      },
      {
        title: "ExampleTabs",
        description:
          "Tabbed code panels for basic vs. advanced usage patterns.",
        href: docsRoute("components/example-tabs"),
        role: "Examples",
      },
    ],
  },
  {
    title: "Display Components",
    summary:
      "Present diagrams, file structures, notes, feature summaries, comparisons, key numbers, and rendered technical notation.",
    accent: "var(--chart-4)",
    entries: [
      {
        title: "Callout",
        description:
          "Highlighted message blocks for notes, warnings, tips, and alerts.",
        href: docsRoute("components/callout"),
        role: "Message",
      },
      {
        title: "FeatureCards",
        description:
          "Cards for feature overviews, landing pages, and link grids.",
        href: docsRoute("components/feature-cards"),
        role: "Summary",
      },
      {
        title: "FileTree",
        description: "Visual file and folder tree for project structures.",
        href: docsRoute("components/file-tree"),
        role: "Files",
      },
      {
        title: "DocPreview",
        description:
          "Responsive iframe frames for showing generated docs pages inside a guide.",
        href: docsRoute("components/doc-preview"),
        role: "Preview",
      },
      {
        title: "BrowserFrame",
        description:
          "Browser window chrome with a mono URL bar for framing embedded examples.",
        href: docsRoute("components/browser-frame"),
        role: "Frame",
      },
      {
        title: "Mermaid",
        description: "Diagrams rendered as SVG for flows and sequences.",
        href: docsRoute("components/mermaid"),
        role: "Diagram",
      },
      {
        title: "Math",
        description: "LaTeX math rendering with KaTeX.",
        href: docsRoute("components/math"),
        role: "Formula",
      },
      {
        title: "Code Blocks",
        description:
          "Syntax highlighting, line numbers, filenames, and word highlighting.",
        href: docsRoute("components/code-blocks"),
        role: "Code",
      },
      {
        title: "StatStrip",
        description: "A row of big numbers with labels that makes scale legible.",
        href: docsRoute("components/stat-strip"),
        role: "Numbers",
      },
      {
        title: "PullQuote",
        description:
          "A high-emphasis block for the one sentence the page exists to deliver.",
        href: docsRoute("components/pull-quote"),
        role: "Emphasis",
      },
      {
        title: "CompareMatrix",
        description:
          "Comparison grid with check, dash, and partial cells and a highlighted column.",
        href: docsRoute("components/compare-matrix"),
        role: "Compare",
      },
      {
        title: "Swot",
        description:
          "Four-quadrant grid of strengths, weaknesses, opportunities, and threats.",
        href: docsRoute("components/swot"),
        role: "Assess",
      },
    ],
  },
  {
    title: "Prose Components",
    summary:
      "Mark keys, define terms, and label the parts of a long guide without leaving the sentence.",
    accent: "var(--chart-1)",
    entries: [
      {
        title: "Kbd",
        description: "Keycaps for shortcuts and key names inside a sentence.",
        href: docsRoute("components/kbd"),
        role: "Keys",
      },
      {
        title: "Marker",
        description: "A labelled dashed divider between the parts of a long guide.",
        href: docsRoute("components/marker"),
        role: "Divider",
      },
      {
        title: "Term",
        description:
          "An inline term whose definition opens on hover, focus, or tap.",
        href: docsRoute("components/term"),
        role: "Glossary",
      },
    ],
  },
  {
    title: "API Reference Components",
    summary:
      "These appear automatically when Folioh turns Python source into reference pages.",
    accent: "var(--chart-3)",
    entries: [
      {
        title: "ClassOverview",
        description:
          "Card displaying a Python class with bases, decorators, and description.",
        href: docsRoute("components/class-overview"),
        role: "Class",
      },
      {
        title: "MethodAccordion",
        description:
          "Expandable panels for documenting class methods and signatures.",
        href: docsRoute("components/method-accordion"),
        role: "Method",
      },
      {
        title: "ParamTable",
        description: "Structured table for function or method parameters.",
        href: docsRoute("components/param-table"),
        role: "Params",
      },
      {
        title: "TypeBadge",
        description:
          "Inline badge for displaying type annotations with optional links.",
        href: docsRoute("components/type-badge"),
        role: "Types",
      },
      {
        title: "DeprecationNotice",
        description:
          "Banner indicating deprecated classes, functions, or modules.",
        href: docsRoute("components/deprecation-notice"),
        role: "Lifecycle",
      },
    ],
  },
  {
    title: "Theme Components",
    summary:
      "Small pieces of site chrome used by the generated Nextra theme.",
    accent: "var(--chart-2)",
    entries: [
      {
        title: "Theme picker",
        description:
          "Gallery of themes and their colours, with a Customize step for each.",
        href: docsRoute("components/theme-configurator"),
        role: "Theme",
      },
      {
        title: "Page Actions",
        description:
          "Header controls for copying a page or opening it with AI context.",
        href: docsRoute("components/copy-page-button"),
        role: "Actions",
      },
    ],
  },
]

function groupId(title: string) {
  return title.toLowerCase().replace(" components", "").replaceAll(" ", "-")
}

export function ComponentIndex() {
  return (
    <div className="component-gallery not-prose">
      <nav className="component-gallery-nav" aria-label="Component families">
        {componentGroups.map((group) => (
          <a key={group.title} href={`#${groupId(group.title)}`}>
            {group.title.replace(" Components", "")}
          </a>
        ))}
      </nav>
      {componentGroups.map((group) => (
        <section
          className="component-gallery-family"
          key={group.title}
          aria-labelledby={groupId(group.title)}
        >
          <h2 id={groupId(group.title)}>
            {group.title.replace(" Components", "")}
          </h2>
          <div className="component-gallery-grid">
            {group.entries.map((entry) => (
              <article className="component-gallery-card" key={entry.href}>
                <ComponentGalleryPreview
                  title={entry.title}
                  compact={
                    group.title === "Theme Components" ||
                    ["Kbd", "Term", "TypeBadge"].includes(entry.title)
                  }
                >
                  {group.title === "Theme Components" ? (
                    <span>Available in the site header</span>
                  ) : (
                    <ComponentIndexPreview title={entry.title} />
                  )}
                </ComponentGalleryPreview>
                <a className="component-gallery-link" href={entry.href}>
                  {entry.title}
                  <span aria-hidden="true">↗</span>
                </a>
              </article>
            ))}
          </div>
        </section>
      ))}
    </div>
  )
}
