import type React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "@/lib/utils"

const statusVariants = cva(
  "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold transition-colors",
  {
    variants: {
      variant: {
        success: "bg-accent/10 text-accent dark:bg-accent/20",
        warning: "bg-yellow-500/10 text-yellow-700 dark:text-yellow-300",
        error: "bg-destructive/10 text-destructive",
        pending: "bg-blue-500/10 text-blue-700 dark:text-blue-300",
        info: "bg-primary/10 text-primary",
      },
    },
    defaultVariants: {
      variant: "info",
    },
  },
)

export interface StatusBadgeProps extends VariantProps<typeof statusVariants> {
  children: React.ReactNode
}

export function StatusBadge({ variant, children }: StatusBadgeProps) {
  return <span className={cn(statusVariants({ variant }))}>{children}</span>
}
