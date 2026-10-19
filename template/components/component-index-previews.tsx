import { Callout } from "./callout"
import { TerminalSession } from "./terminal-session"
import { ConfigPanel } from "./config-panel"
import { BuildArtifact } from "./build-artifact"
import { CommandGrid, CommandCard } from "./command-grid"
import { BeforeAfter } from "./before-after"
import { Checklist } from "./checklist"
import { HookMap } from "./hook-map"
import { Accordion, AccordionItem } from "./accordion"
import { Tabs, TabItem } from "./tabs"
import { Steps, Step } from "./steps"
import { Timeline, TimelineItem } from "./timeline"
import { CodeGroup } from "./code-group"
import { PreviewCode } from "./preview-code"
import { ExampleTabs } from "./example-tabs"
import { FeatureCard } from "./feature-card"
import { FileTree } from "./file-tree"
import { DocPreview } from "./doc-preview"
import { BrowserFrame } from "./browser-frame"
import { Mermaid } from "./mermaid"
import { CodeBlockPre } from "./code-block"
import { Code as NextraCode } from "nextra/components"
import { compileMdx } from "nextra/compile"
import { evaluate } from "nextra/evaluate"
import { StatStrip } from "./stat-strip"
import { PullQuote } from "./pull-quote"
import { CompareMatrix } from "./compare-matrix"
import { Swot } from "./swot"
import { Kbd } from "./kbd"
import { Marker } from "./marker"
import { Term } from "./term"
import { ClassOverview } from "./class-overview"
import { MethodAccordion } from "./method-accordion"
import { ParamTable } from "./param-table"
import { TypeBadge } from "./type-badge"
import { DeprecationNotice } from "./deprecation-notice"
import katex from "katex"

// Gallery fixtures go through the same server-side highlighter as fenced MDX.
async function Code({
  children,
  language,
  filename,
}: {
  children: string
  language: "python" | "rust" | "bash" | "yaml" | "mdx"
  filename?: string
}) {
  const meta = filename ? ` filename=${JSON.stringify(filename)}` : ""
  const compiled = await compileMdx(
    `\`\`\`${language}${meta}\n${children}\n\`\`\``,
    {
      defaultShowCopyCode: true,
    }
  )
  const { default: Content } = evaluate(compiled, {
    pre: CodeBlockPre,
    code: NextraCode,
  })
  return (
    <div data-language={language}>
      <Content />
    </div>
  )
}

export function ComponentIndexPreview({ title }: { title: string }) {
  switch (title) {
    case "TerminalSession":
      return (
        <TerminalSession
          title="Build your docs"
          command="folio build"
          output={["✓ Pages generated", "✓ Search indexed", "Ready in _site/"]}
          status="success"
        />
      )
    case "ConfigPanel":
      return (
        <ConfigPanel
          title="docs.yaml"
          fields={[
            {
              name: "output",
              description: "Where your site is written.",
              default: "_site",
            },
          ]}
        >
          <Code language="yaml">
            {"project:\n  name: Atlas\noutput: _site"}
          </Code>
        </ConfigPanel>
      )
    case "BuildArtifact":
      return (
        <BuildArtifact
          title="Your published site"
          items={[
            { path: "_site/", kind: "HTML", description: "Static pages" },
            {
              path: "llms.txt",
              kind: "Text",
              description: "Context for agents",
            },
          ]}
        />
      )
    case "CommandGrid":
      return (
        <CommandGrid>
          <CommandCard
            command="folio serve"
            title="Preview locally"
            description="Rebuild as you write."
            flags={["--port", "--clean"]}
          />
        </CommandGrid>
      )
    case "BeforeAfter":
      return (
        <BeforeAfter
          before={"docs/\n  index.md"}
          after={"_site/\n  index.html"}
          beforeLabel="Source"
          afterLabel="Published"
        />
      )
    case "Checklist":
      return (
        <Checklist
          items={[
            { label: "Write your first guide", state: "done" },
            { label: "Preview the site", state: "done" },
            { label: "Publish when ready", state: "todo" },
          ]}
        />
      )
    case "HookMap":
      return (
        <HookMap
          title="Build lifecycle"
          hooks={[
            {
              stage: "Configure",
              hook: "configure",
              description: "Read project settings",
            },
            {
              stage: "Collect",
              hook: "collect",
              description: "Gather documents",
            },
          ]}
        />
      )
    case "Accordion":
      return (
        <Accordion>
          <AccordionItem title="Can I use my own components?" defaultOpen>
            Yes. Register a React component in docs.yaml.
          </AccordionItem>
          <AccordionItem title="Where can I publish?">
            Any host that serves static files.
          </AccordionItem>
        </Accordion>
      )
    case "Tabs":
      return (
        <Tabs aria-label="Gallery install examples">
          <TabItem label="Install">
            <Code language="bash">
              curl -fsSL
              https://raw.githubusercontent.com/pguijas/folio/main/install.sh |
              sh
            </Code>
          </TabItem>
          <TabItem label="Build">
            <Code language="bash">folio build</Code>
          </TabItem>
        </Tabs>
      )
    case "Steps":
      return (
        <Steps>
          <Step title="Initialize">
            <p>Start with your source files.</p>
          </Step>
          <Step title="Build">
            <p>Turn them into a documentation site.</p>
          </Step>
        </Steps>
      )
    case "Timeline":
      return (
        <Timeline>
          <TimelineItem date="Today" title="First release" badge="New">
            Your documentation is live.
          </TimelineItem>
          <TimelineItem date="Yesterday" title="First build">
            A guide and an API reference.
          </TimelineItem>
        </Timeline>
      )
    case "CodeGroup":
      return (
        <CodeGroup labels={["Python", "Rust"]}>
          <Code language="python">
            {'def greet(name):\n    return f"Hello, {name}"'}
          </Code>
          <Code language="rust">
            {'fn greet(name: &str) {\n    println!("Hello, {name}");\n}'}
          </Code>
        </CodeGroup>
      )
    case "PreviewCode":
      return (
        <PreviewCode>
          <Callout type="tip" title="Make it yours">
            Use components inside any MDX page.
          </Callout>
          <Code language="mdx">
            {
              '<Callout type="tip" title="Make it yours">\n  Use components inside any MDX page.\n</Callout>'
            }
          </Code>
        </PreviewCode>
      )
    case "ExampleTabs":
      return (
        <ExampleTabs
          examples={[
            { label: "Basic", code: "folio build", language: "bash" },
            { label: "Clean", code: "folio build --clean", language: "bash" },
          ]}
        />
      )
    case "Callout":
      return (
        <div>
          <Callout type="tip" title="A little guidance">
            Keep the useful detail close to the example.
          </Callout>
          <Callout type="warning" title="Before you publish">
            Check your internal links.
          </Callout>
        </div>
      )
    case "FeatureCards":
      return (
        <FeatureCard
          title="Built to be read"
          description="Clear guides, code examples and API references."
          icon="book"
        />
      )
    case "FileTree":
      return (
        <FileTree
          tree={
            "atlas/\n├── docs/\n│   ├── index.md\n│   └── quickstart.md\n└── docs.yaml"
          }
        />
      )
    case "DocPreview":
      return (
        <DocPreview
          example="generated-site"
          title="Generated documentation"
          height={210}
        />
      )
    case "BrowserFrame":
      return (
        <BrowserFrame url="atlas.dev/docs">
          <Callout type="info" title="Welcome to Atlas">
            Your documentation, in context.
          </Callout>
        </BrowserFrame>
      )
    case "Mermaid":
      return (
        <Mermaid
          chart={"flowchart LR\n  A[Source] --> B[Build]\n  B --> C[Your site]"}
        />
      )
    case "Math":
      return (
        <div
          dangerouslySetInnerHTML={{
            __html: katex.renderToString(
              "f(x) = \\int_{0}^{x} t^2\\,dt = \\frac{x^3}{3}",
              { displayMode: true, throwOnError: false }
            ),
          }}
        />
      )
    case "Code Blocks":
      return (
        <Code language="python" filename="greet.py">
          {
            'def greet(name: str) -> str:\n    """Say hello."""\n    return f"Hello, {name}"'
          }
        </Code>
      )
    case "StatStrip":
      return (
        <StatStrip
          stats={[
            { value: "12", label: "Guides" },
            { value: "3", label: "Languages" },
          ]}
        />
      )
    case "PullQuote":
      return <PullQuote>Write once. Read everywhere.</PullQuote>
    case "CompareMatrix":
      return (
        <CompareMatrix
          tools={["Basic", "Extended"]}
          highlight={1}
          rows={[
            { feature: "Guides", values: [true, true] },
            { feature: "API reference", values: [false, true] },
          ]}
        />
      )
    case "Swot":
      return (
        <Swot
          strengths={["Static output"]}
          weaknesses={["Build step"]}
          opportunities={["New readers"]}
          threats={["Stale guides"]}
        />
      )
    case "Kbd":
      return (
        <p>
          Find anything with <Kbd>⌘</Kbd> <Kbd>K</Kbd>
        </p>
      )
    case "Marker":
      return (
        <div>
          <p>Your first guide is ready.</p>
          <Marker>Next chapter</Marker>
          <p>Now add an API reference.</p>
        </div>
      )
    case "Term":
      return (
        <p>
          Publish a{" "}
          <Term def="A site made of HTML, CSS and JavaScript files, served without a backend.">
            static site
          </Term>{" "}
          anywhere.
        </p>
      )
    case "ClassOverview":
      return (
        <ClassOverview
          name="Document"
          bases={["BaseModel"]}
          description="A page and its metadata."
        />
      )
    case "MethodAccordion":
      return (
        <MethodAccordion
          methods={[
            {
              name: "build",
              signature: "build(clean: bool = False)",
              description: "Generate a fresh documentation site.",
            },
          ]}
        />
      )
    case "ParamTable":
      return (
        <ParamTable
          args={[
            { name: "path", type: "str", description: "Source directory" },
          ]}
        />
      )
    case "TypeBadge":
      return (
        <p>
          Returns <TypeBadge type="str" /> or <TypeBadge type="None" />
        </p>
      )
    case "DeprecationNotice":
      return (
        <DeprecationNotice
          since="2.0"
          alternative="build()"
          message="Use the unified build command."
        />
      )
    default:
      return null
  }
}
