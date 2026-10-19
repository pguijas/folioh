"use client"

import { useConfig, useThemeConfig } from "nextra-theme-docs"
import { Comment01Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import { Button } from "@/components/ui/button"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"

type FeedbackConfig = ReturnType<typeof useThemeConfig>["feedback"]

export function feedbackHref(
  feedback: FeedbackConfig,
  repository: string,
  pageTitle?: string,
): string | null {
  if (!feedback.content) return null
  if (feedback.link != null) return feedback.link

  const { origin, pathname } = new URL(repository)
  const [, owner, name] = pathname.split("/", 3)
  const title = encodeURIComponent(`Feedback for “${pageTitle}”`)
  const { labels } = feedback
  if (origin.includes("gitlab")) {
    const description = labels ? `&issue[description]=/label${encodeURIComponent(` ~${labels}\n`)}` : ""
    return `${origin}/${owner}/${name}/-/issues/new?issue[title]=${title}${description}`
  }
  if (origin.includes("github")) {
    return `${origin}/${owner}/${name}/issues/new?title=${title}&labels=${labels || ""}`
  }
  return null
}

export function FloatingFeedback() {
  const { feedback, docsRepositoryBase } = useThemeConfig()
  const { activeMetadata } = useConfig().normalizePagesResult
  const href = feedbackHref(feedback, docsRepositoryBase, activeMetadata?.title)
  if (href === null) return null

  return (
    <TooltipProvider delayDuration={150}>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button asChild variant="secondary" size="icon-lg" className="folioh-feedback rounded-full">
            <a
              href={href}
              target="_blank"
              rel="noreferrer"
              aria-label={`Give feedback about ${activeMetadata?.title || "this page"} (opens in a new tab)`}
            >
              <HugeiconsIcon icon={Comment01Icon} data-icon="inline-start" aria-hidden="true" />
            </a>
          </Button>
        </TooltipTrigger>
        <TooltipContent side="left" sideOffset={8} className="folioh-feedback-label">
          Feedback
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  )
}
