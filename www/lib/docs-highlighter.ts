import {
  createHighlighter,
  type Highlighter,
  type LanguageRegistration,
} from "shiki";

import teaGrammar from "@/lib/tea.tmLanguage.json";

export const DOCS_HIGHLIGHT_THEME_LIGHT = "rose-pine-dawn";
export const DOCS_HIGHLIGHT_THEME_DARK = "rose-pine-moon";

const teaLanguage: LanguageRegistration = {
  ...teaGrammar,
  name: "tea",
} as LanguageRegistration;

let highlighterPromise: Promise<Highlighter> | null = null;

export function getDocsHighlightTheme(theme: "light" | "dark" = "light") {
  return theme === "dark"
    ? DOCS_HIGHLIGHT_THEME_DARK
    : DOCS_HIGHLIGHT_THEME_LIGHT;
}

export function normalizeHighlightedLanguage(language: string) {
  const langMap: Record<string, string> = {
    js: "javascript",
    sh: "bash",
    shell: "bash",
    ts: "typescript",
  };

  return langMap[language] || language;
}

export async function getDocsHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = createHighlighter({
      themes: [DOCS_HIGHLIGHT_THEME_LIGHT, DOCS_HIGHLIGHT_THEME_DARK],
      langs: [
        "javascript",
        "typescript",
        "json",
        "bash",
        "rust",
        "yaml",
        "toml",
        "text",
        teaLanguage,
      ],
    });
  }

  return highlighterPromise;
}
