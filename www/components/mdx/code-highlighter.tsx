'use client'

import { useEffect, useState } from 'react'
import { Check, Copy } from 'lucide-react'
import { createHighlighter, type Highlighter, type LanguageRegistration } from 'shiki'

// Import grammar statically for bundling
import teaGrammar from '@/lib/tea.tmLanguage.json'

interface CodeHighlighterProps {
  code: string
  language: string
}

const ACTIVE_THEME = 'rose-pine-dawn'

// Singleton highlighter promise
let highlighterPromise: Promise<Highlighter> | null = null

// Create a proper language registration for Tea
const teaLanguage: LanguageRegistration = {
  ...teaGrammar,
  name: 'tea',
} as LanguageRegistration

async function getHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = (async () => {
      const highlighter = await createHighlighter({
        themes: [ACTIVE_THEME],
        langs: [
          'javascript',
          'typescript',
          'json',
          'bash',
          'rust',
          'yaml',
          'toml',
          'text',
          teaLanguage,
        ],
      })

      return highlighter
    })()
  }
  return highlighterPromise
}

// Cache for highlighted code
const cache = new Map<string, string>()

export function CodeHighlighter({ code, language }: CodeHighlighterProps) {
  const [html, setHtml] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)
  const trimmedCode = code.trim()
  const cacheKey = `${ACTIVE_THEME}:${language}:${trimmedCode}`

  useEffect(() => {
    // Check cache first
    const cached = cache.get(cacheKey)
    if (cached) {
      setHtml(cached)
      return
    }

    // Map language aliases
    const langMap: Record<string, string> = {
      sh: 'bash',
      shell: 'bash',
      ts: 'typescript',
      js: 'javascript',
    }

    const effectiveLang = langMap[language] || language

    const highlight = async () => {
      try {
        const highlighter = await getHighlighter()
        const loadedLangs = highlighter.getLoadedLanguages()
        const langToUse = loadedLangs.includes(effectiveLang) ? effectiveLang : 'text'

        const result = highlighter.codeToHtml(trimmedCode, {
          lang: langToUse,
          theme: ACTIVE_THEME,
        })
        cache.set(cacheKey, result)
        setHtml(result)
      } catch (e) {
        console.error('Syntax highlighting failed:', e)
        setHtml(null)
      }
    }

    highlight()
  }, [trimmedCode, language, cacheKey])

  useEffect(() => {
    if (!copied) return

    const timeoutId = window.setTimeout(() => setCopied(false), 1800)
    return () => window.clearTimeout(timeoutId)
  }, [copied])

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(trimmedCode)
      setCopied(true)
    } catch (error) {
      console.error('Clipboard copy failed:', error)
    }
  }

  const copyButton = (
    <button
      type="button"
      onClick={handleCopy}
      className="absolute right-3 top-3 z-10 inline-flex items-center gap-1.5 rounded-full border border-border/70 bg-background/92 px-2.5 py-1.5 text-xs font-medium text-muted-foreground shadow-sm backdrop-blur transition hover:border-primary/35 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
      aria-label={copied ? 'Code copied' : 'Copy code to clipboard'}
    >
      {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
      <span>{copied ? 'Copied' : 'Copy'}</span>
    </button>
  )

  if (!html) {
    // Show unstyled code while loading or on error
    return (
      <div className="relative">
        {copyButton}
        <pre className="overflow-x-auto rounded-[1.4rem] bg-[var(--code-background)] p-4 pr-20 font-mono shadow-[inset_0_0_0_1px_var(--code-border),inset_0_1px_0_rgb(255_255_255_/_0.55),0_1px_2px_rgb(30_41_59_/_0.04)] md:p-5 md:pr-24">
          <code className="font-mono text-sm text-foreground">{trimmedCode}</code>
        </pre>
      </div>
    )
  }

  return (
    <div className="relative">
      {copyButton}
      <div
        className="[&>pre]:mb-0 [&>pre]:overflow-x-auto [&>pre]:rounded-[1.4rem] [&>pre]:bg-[var(--code-background)] [&>pre]:p-4 [&>pre]:pr-20 [&>pre]:font-mono [&>pre]:shadow-[inset_0_0_0_1px_var(--code-border),inset_0_1px_0_rgb(255_255_255_/_0.55),0_1px_2px_rgb(30_41_59_/_0.04)] md:[&>pre]:p-5 md:[&>pre]:pr-24 [&_code]:font-mono [&_code]:text-sm"
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </div>
  )
}
