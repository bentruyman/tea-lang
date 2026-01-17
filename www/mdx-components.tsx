import type { MDXComponents } from 'mdx/types'
import { Card } from '@/components/ui/card'
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
  KeyConceptsCard,
  KeyConcept,
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
      <h1 className="text-4xl font-bold text-balance mb-4">{children}</h1>
    ),
    h2: ({ children }) => (
      <h2 className="text-3xl font-bold mt-12 mb-6">{children}</h2>
    ),
    h3: ({ children }) => (
      <h3 className="text-xl font-semibold text-accent mb-3">{children}</h3>
    ),
    p: ({ children }) => (
      <p className="text-muted-foreground leading-relaxed mb-4">{children}</p>
    ),
    code: ({ children, className }) => {
      // If it has a language class, it's a code block (handled by pre wrapper)
      if (className?.startsWith('language-')) {
        return <code className={className}>{children}</code>
      }
      // Inline code
      return <code className="text-accent bg-muted px-1.5 py-0.5 rounded text-sm">{children}</code>
    },
    pre: ({ children }) => {
      // Extract the code element's props
      const codeChild = Children.toArray(children).find(
        (child) => isValidElement(child) && child.type === 'code'
      )

      if (isValidElement(codeChild)) {
        const props = codeChild.props as { className?: string; children?: ReactNode }
        const className = props.className
        const language = extractLanguage(className)
        const code = extractTextContent(props.children)

        return <CodeHighlighter code={code} language={language} />
      }

      // Fallback for non-code content
      return (
        <pre className="bg-muted p-4 rounded-md overflow-x-auto mb-4">
          <code className="font-mono text-sm">{children}</code>
        </pre>
      )
    },
    ul: ({ children }) => (
      <ul className="list-disc list-inside space-y-2 text-muted-foreground mb-4">{children}</ul>
    ),
    ol: ({ children }) => (
      <ol className="list-decimal list-inside space-y-2 text-muted-foreground mb-4">{children}</ol>
    ),
    li: ({ children }) => (
      <li className="leading-relaxed">{children}</li>
    ),
    a: ({ href, children }) => (
      <a href={href} className="text-accent hover:underline">{children}</a>
    ),
    blockquote: ({ children }) => (
      <blockquote className="border-l-4 border-accent pl-4 italic text-muted-foreground my-4">
        {children}
      </blockquote>
    ),
    // Custom components available in MDX
    Card,
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
    KeyConceptsCard,
    KeyConcept,
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
