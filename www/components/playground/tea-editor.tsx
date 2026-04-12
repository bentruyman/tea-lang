"use client";

import {
  Fragment,
  useDeferredValue,
  useEffect,
  useId,
  useRef,
  useState,
  type ChangeEvent,
  type CSSProperties,
  type KeyboardEvent,
} from "react";
import type { ThemedToken } from "shiki";

import {
  getDocsHighlighter,
  getDocsHighlightTheme,
} from "@/lib/docs-highlighter";
import { cn } from "@/lib/utils";

type TeaEditorProps = {
  value: string;
  onChange: (value: string) => void;
  className?: string;
};

const FONT_STYLE_ITALIC = 1;
const FONT_STYLE_BOLD = 2;
const FONT_STYLE_UNDERLINE = 4;
const FONT_STYLE_STRIKETHROUGH = 8;
const EDITOR_LINE_HEIGHT = "1.35";
const EDITOR_TEXT_CLASS = "px-4 py-3.5 font-mono text-[0.95rem] [tab-size:2]";

const highlightCache = new Map<string, ThemedToken[][]>();
const EDITOR_HIGHLIGHT_THEME = getDocsHighlightTheme("dark");

function fallbackTokens(code: string) {
  return code
    .split("\n")
    .map((line) => (line ? [{ content: line, offset: 0 }] : []));
}

function getTokenStyle(token: ThemedToken): CSSProperties {
  if (token.htmlStyle) {
    return token.htmlStyle;
  }

  const style: CSSProperties = {};
  const fontStyle = token.fontStyle ?? 0;
  const decorations: string[] = [];

  if (token.color) {
    style.color = token.color;
  }

  if (fontStyle & FONT_STYLE_ITALIC) {
    style.fontStyle = "italic";
  }

  if (fontStyle & FONT_STYLE_BOLD) {
    style.fontWeight = 700;
  }

  if (fontStyle & FONT_STYLE_UNDERLINE) {
    decorations.push("underline");
  }

  if (fontStyle & FONT_STYLE_STRIKETHROUGH) {
    decorations.push("line-through");
  }

  if (decorations.length > 0) {
    style.textDecoration = decorations.join(" ");
  }

  return style;
}

export function TeaEditor({ value, onChange, className }: TeaEditorProps) {
  const editorId = useId();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const highlightRef = useRef<HTMLPreElement>(null);
  const deferredValue = useDeferredValue(value);
  const [lines, setLines] = useState<ThemedToken[][]>(fallbackTokens(value));

  useEffect(() => {
    const cacheKey = `${EDITOR_HIGHLIGHT_THEME}:${deferredValue}`;
    const cached = highlightCache.get(cacheKey);

    if (cached) {
      setLines(cached);
      return;
    }

    let cancelled = false;

    const highlight = async () => {
      try {
        const highlighter = await getDocsHighlighter();
        const nextLines = highlighter.codeToTokensBase(deferredValue, {
          lang: "tea" as Parameters<
            typeof highlighter.codeToTokensBase
          >[1]["lang"],
          theme: EDITOR_HIGHLIGHT_THEME,
        });

        if (cancelled) {
          return;
        }

        highlightCache.set(cacheKey, nextLines);
        setLines(nextLines);
      } catch (error) {
        console.error("Tea editor highlighting failed:", error);

        if (!cancelled) {
          setLines(fallbackTokens(deferredValue));
        }
      }
    };

    highlight();

    return () => {
      cancelled = true;
    };
  }, [deferredValue]);

  function syncHighlightScroll(target: HTMLTextAreaElement) {
    if (!highlightRef.current) {
      return;
    }

    highlightRef.current.style.transform = `translate3d(${-target.scrollLeft}px, ${-target.scrollTop}px, 0)`;
  }

  function handleChange(event: ChangeEvent<HTMLTextAreaElement>) {
    onChange(event.target.value);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Tab") {
      return;
    }

    event.preventDefault();

    const textarea = event.currentTarget;
    const selectionStart = textarea.selectionStart;
    const selectionEnd = textarea.selectionEnd;
    const nextValue = `${value.slice(0, selectionStart)}  ${value.slice(selectionEnd)}`;
    const nextCaret = selectionStart + 2;

    onChange(nextValue);

    requestAnimationFrame(() => {
      if (!textareaRef.current) {
        return;
      }

      textareaRef.current.selectionStart = nextCaret;
      textareaRef.current.selectionEnd = nextCaret;
      syncHighlightScroll(textareaRef.current);
    });
  }

  return (
    <div className={cn("relative", className)}>
      <label htmlFor={editorId} className="sr-only">
        Tea playground source
      </label>
      <div
        className="relative min-h-[32rem] overflow-hidden rounded-[1.4rem] border transition focus-within:border-primary/35 focus-within:ring-2 focus-within:ring-primary/15"
        style={{
          backgroundColor: "var(--code-background)",
          borderColor: "var(--code-border)",
          boxShadow:
            "inset 0 0 0 1px var(--code-border), inset 0 1px 0 rgb(255 255 255 / 0.6), 0 18px 36px -30px rgb(70 47 30 / 0.28)",
        }}
      >
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 overflow-hidden"
        >
          <pre
            ref={highlightRef}
            className={cn(
              EDITOR_TEXT_CLASS,
              "m-0 min-h-[32rem] whitespace-pre-wrap break-words will-change-transform",
            )}
            style={{
              color: "var(--code-foreground)",
              lineHeight: EDITOR_LINE_HEIGHT,
            }}
          >
            {lines.map((line, lineIndex) => (
              <Fragment key={`line-${lineIndex}`}>
                {line.length > 0
                  ? line.map((token, tokenIndex) => (
                      <span
                        key={`token-${lineIndex}-${tokenIndex}-${token.offset}`}
                        style={getTokenStyle(token)}
                      >
                        {token.content}
                      </span>
                    ))
                  : " "}
                {lineIndex < lines.length - 1 ? "\n" : null}
              </Fragment>
            ))}
          </pre>
        </div>

        <textarea
          id={editorId}
          ref={textareaRef}
          value={value}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          onScroll={(event) => syncHighlightScroll(event.currentTarget)}
          spellCheck={false}
          className={cn(
            EDITOR_TEXT_CLASS,
            "min-h-[32rem] w-full resize-none border-0 bg-transparent text-transparent outline-none selection:bg-emerald-500/16",
          )}
          style={{
            caretColor: "var(--code-foreground)",
            lineHeight: EDITOR_LINE_HEIGHT,
          }}
        />
      </div>
    </div>
  );
}
