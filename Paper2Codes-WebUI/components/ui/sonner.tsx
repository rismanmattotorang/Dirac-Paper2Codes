'use client'

import * as React from 'react'
import { useTheme } from '@/components/theme-provider'
import { Toaster as Sonner, ToasterProps } from 'sonner'

const Toaster = ({ ...props }: ToasterProps) => {
  let themeValue: ToasterProps['theme'] = 'system'
  
  try {
    const { theme } = useTheme()
    themeValue = theme === 'dark' ? 'dark' : theme === 'light' ? 'light' : 'system'
  } catch {
    // Theme context not available (SSR), use default
    themeValue = 'system'
  }

  return (
    <Sonner
      theme={themeValue}
      className="toaster group"
      style={
        {
          '--normal-bg': 'var(--popover)',
          '--normal-text': 'var(--popover-foreground)',
          '--normal-border': 'var(--border)',
        } as React.CSSProperties
      }
      {...props}
    />
  )
}

export { Toaster }
