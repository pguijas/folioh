"use client"

import { ArrowDown01Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import { Button } from "@/components/ui/button"
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover"

export function FolioProductSwitcher({ homeHref }: { homeHref: string }) {
  return (
    <div className="flex items-center gap-1">
      <span aria-hidden="true" className="text-muted-foreground">
        /
      </span>
      <Popover>
        <PopoverTrigger asChild>
          <Button
            variant="ghost"
            size="sm"
            aria-label="Choose a Folioh product: Docs"
          >
            Docs
            <HugeiconsIcon
              icon={ArrowDown01Icon}
              data-icon="inline-end"
              aria-hidden="true"
            />
          </Button>
        </PopoverTrigger>
        <PopoverContent
          align="start"
          collisionPadding={16}
          className="w-56 p-2"
          aria-label="Folioh products"
        >
          <nav aria-label="Folioh products" className="flex flex-col gap-1">
            <Button asChild variant="secondary" className="justify-start">
              <a href={homeHref} aria-current="true">
                Docs
              </a>
            </Button>
            <Button asChild variant="ghost" className="justify-start">
              <a href="https://pguijas.github.io/board-2/">Board</a>
            </Button>
          </nav>
        </PopoverContent>
      </Popover>
    </div>
  )
}
