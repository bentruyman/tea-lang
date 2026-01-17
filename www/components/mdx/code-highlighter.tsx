'use client'

import { useEffect, useState } from 'react'
import { createHighlighter, type Highlighter } from 'shiki'

// Import grammar statically for bundling
import teaLanguage from '@/lib/tea.tmLanguage.json'

interface CodeHighlighterProps {
  code: string
  language: string
}

// Singleton highlighter promise
let highlighterPromise: Promise<Highlighter> | null = null

async function getHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = (async () => {
      const highlighter = await createHighlighter({
        themes: ['github-dark'],
        langs: [
          'javascript',
          'typescript',
          'json',
          'bash',
          'rust',
          'yaml',
          'toml',
          'text',
        ],
      })

      // Load custom Tea language after initialization
      await highlighter.loadLanguage(teaLanguage as any)

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
  const cacheKey = `${language}:${trimmedCode}`

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
          theme: 'github-dark',
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
      <pre className="bg-[#24292e] p-4 rounded-md overflow-x-auto mb-4">
        <code className="font-mono text-sm text-[#e1e4e8]">{trimmedCode}</code>
      </pre>
    )
  }

  return (
    <div
      className="[&>pre]:p-4 [&>pre]:rounded-md [&>pre]:overflow-x-auto [&>pre]:mb-4 [&_code]:font-mono [&_code]:text-sm"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  )
}
