import type { MDXComponents } from 'mdx/types'
import { Card } from '@/components/ui/card'
import { ContentPage, ContentSection, PageIntro } from '@/components/site-shell'
import {
  CodeBlock,
  CodeCard,
  QuickLinkCard,
  NextLink,
  TwoColumnGrid,
  ThreeColumnGrid,
  Divider,
  FeatureCard,
  InstallCard,
  InstallStep,
  PrerequisiteList,
  PrerequisiteItem,
  Step,
  GridLink,
  HelpCard,
  HelpLink,
  KeyConceptCard,
  NoteCard,
  FeaturePill,
  KeyConceptsCard,
  KeyConcept,
  NextSteps,
  AlertCard,
  TypeTable,
  TypeRow,
  RequirementsCard,
  RequirementsSection,
  RequirementItem,
  ModuleTable,
  ModuleRow,
  IntrinsicTable,
  IntrinsicRow,
  ContributionCard,
  ContributionGrid,
  DirectoryCard,
} from '@/components/mdx'
import { CodeHighlighter } from '@/components/mdx/code-highlighter'
import { Children, isValidElement, type ReactNode } from 'react'

// Helper to extract text content from React children
function extractTextContent(children: ReactNode): string {
  if (typeof children === 'string') return children
  if (typeof children === 'number') return String(children)
  if (Array.isArray(children)) return children.map(extractTextContent).join('')
  if (isValidElement(children)) {
    const props = children.props as { children?: ReactNode }
    if (props.children) {
      return extractTextContent(props.children)
    }
  }
  return ''
}

// Helper to extract language from className
function extractLanguage(className?: string): string {
  if (!className) return 'tea'
  const match = className.match(/language-(\w+)/)
  return match ? match[1] : 'tea'
}

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    // Override default HTML elements with styled versions
    h1: ({ children }) => (
      <h1 className="mb-4 font-display text-4xl font-semibold tracking-tight text-balance md:text-5xl">
        {children}
      </h1>
    ),
    h2: ({ children }) => (
      <h2 className="font-display text-3xl font-semibold tracking-tight text-foreground md:text-4xl">
        {children}
      </h2>
    ),
    h3: ({ children }) => (
      <h3 className="font-display text-2xl font-semibold tracking-tight text-primary">{children}</h3>
    ),
    p: ({ children }) => (
      <p className="text-[1.02rem] leading-8 text-muted-foreground md:text-[1.05rem]">{children}</p>
    ),
    code: ({ children, className }) => {
      // If it has a language class, it's a code block (handled by pre wrapper)
      if (className?.startsWith('language-')) {
        return <code className={className}>{children}</code>
      }
      // Inline code
      return (
        <code className="rounded-md border border-border/70 bg-background/70 px-1.5 py-0.5 text-[0.92em] text-foreground">
          {children}
        </code>
      )
    },
    pre: ({ children }) => {
      // Extract the code element - check for both string 'code' type and component with className
      const childArray = Children.toArray(children)

      // Find the code element - could be a native 'code' element or have language-* className
      const codeChild = childArray.find((child) => {
        if (!isValidElement(child)) return false
        // Check if it's a native code element
        if (child.type === 'code') return true
        // Check if props has a language className (for when code override is applied)
        const props = child.props as { className?: string }
        return props.className?.startsWith('language-')
      })

      if (isValidElement(codeChild)) {
        const props = codeChild.props as { className?: string; children?: ReactNode }
        const className = props.className
        const language = extractLanguage(className)
        const code = extractTextContent(props.children)

        // Only use CodeHighlighter if we have actual code content
        if (code.trim()) {
          return <CodeHighlighter code={code} language={language} />
        }
      }

      // Fallback for non-code content
      return (
        <pre className="surface-card overflow-x-auto rounded-[1.2rem] border border-border/70 p-5">
          <code className="font-mono text-sm">{children}</code>
        </pre>
      )
    },
    ul: ({ children }) => (
      <ul className="list-disc space-y-2 pl-5 text-muted-foreground">{children}</ul>
    ),
    ol: ({ children }) => (
      <ol className="list-decimal space-y-2 pl-5 text-muted-foreground">{children}</ol>
    ),
    li: ({ children }) => (
      <li className="leading-8">{children}</li>
    ),
    a: ({ href, children }) => (
      <a href={href} className="font-medium text-primary decoration-primary/35 underline-offset-4 hover:underline">
        {children}
      </a>
    ),
    blockquote: ({ children }) => (
      <blockquote className="surface-quiet rounded-r-2xl border-l-4 border-primary/55 px-5 py-4 italic text-muted-foreground">
        {children}
      </blockquote>
    ),
    // Custom components available in MDX
    Card,
    ContentPage,
    ContentSection,
    PageIntro,
    CodeBlock,
    CodeCard,
    QuickLinkCard,
    NextLink,
    TwoColumnGrid,
    ThreeColumnGrid,
    Divider,
    FeatureCard,
    InstallCard,
    InstallStep,
    PrerequisiteList,
    PrerequisiteItem,
    Step,
    GridLink,
    HelpCard,
    HelpLink,
    KeyConceptCard,
    NoteCard,
    FeaturePill,
    KeyConceptsCard,
    KeyConcept,
    NextSteps,
    AlertCard,
    TypeTable,
    TypeRow,
    RequirementsCard,
    RequirementsSection,
    RequirementItem,
    ModuleTable,
    ModuleRow,
    IntrinsicTable,
    IntrinsicRow,
    ContributionCard,
    ContributionGrid,
    DirectoryCard,
    ...components,
  }
}
