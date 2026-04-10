'use client'

import { useEffect, useState } from 'react'
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

  if (!html) {
    // Show unstyled code while loading or on error
    return (
      <pre className="overflow-x-auto rounded-[1.4rem] bg-[var(--code-background)] p-4 font-mono shadow-[inset_0_0_0_1px_var(--code-border),inset_0_1px_0_rgb(255_255_255_/_0.55),0_1px_2px_rgb(30_41_59_/_0.04)] md:p-5">
        <code className="font-mono text-sm text-foreground">{trimmedCode}</code>
      </pre>
    )
  }

  return (
    <div
      className="[&>pre]:mb-0 [&>pre]:overflow-x-auto [&>pre]:rounded-[1.4rem] [&>pre]:bg-[var(--code-background)] [&>pre]:p-4 [&>pre]:font-mono [&>pre]:shadow-[inset_0_0_0_1px_var(--code-border),inset_0_1px_0_rgb(255_255_255_/_0.55),0_1px_2px_rgb(30_41_59_/_0.04)] md:[&>pre]:p-5 [&_code]:font-mono [&_code]:text-sm"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  )
}
