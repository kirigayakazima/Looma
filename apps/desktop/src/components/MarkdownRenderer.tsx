import React from 'react';

interface MarkdownRendererProps {
  content: string;
  className?: string;
}

export const MarkdownRenderer: React.FC<MarkdownRendererProps> = ({ content, className = '' }) => {
  if (!content) return null;

  // Split lines
  const lines = content.split('\n');
  const elements: React.ReactNode[] = [];

  let inCodeBlock = false;
  let codeBlockLines: string[] = [];

  const renderInline = (text: string): React.ReactNode => {
    // Process inline code, bold, italic, links
    const parts: React.ReactNode[] = [];
    let remaining = text;
    let keyIdx = 0;

    // Regex for inline patterns: `code`, **bold**, *italic*, [link](url)
    const pattern = /(`[^`]+`|\*\*[^*]+\*\*|\*[^*]+\*|\[[^\]]+\]\([^)]+\))/g;
    let lastIndex = 0;
    let match: RegExpExecArray | null;

    while ((match = pattern.exec(remaining)) !== null) {
      if (match.index > lastIndex) {
        parts.push(remaining.substring(lastIndex, match.index));
      }

      const matchedStr = match[0];
      if (matchedStr.startsWith('`') && matchedStr.endsWith('`')) {
        parts.push(
          <code
            key={keyIdx++}
            className="px-1.5 py-0.5 rounded bg-neutral-800 text-amber-300 font-mono text-[11px] border border-neutral-700/50"
          >
            {matchedStr.slice(1, -1)}
          </code>
        );
      } else if (matchedStr.startsWith('**') && matchedStr.endsWith('**')) {
        parts.push(
          <strong key={keyIdx++} className="font-semibold text-neutral-100">
            {matchedStr.slice(2, -2)}
          </strong>
        );
      } else if (matchedStr.startsWith('*') && matchedStr.endsWith('*')) {
        parts.push(
          <em key={keyIdx++} className="italic text-neutral-200">
            {matchedStr.slice(1, -1)}
          </em>
        );
      } else if (matchedStr.startsWith('[') && matchedStr.includes('](')) {
        const linkText = matchedStr.substring(1, matchedStr.indexOf(']('));
        const linkUrl = matchedStr.substring(matchedStr.indexOf('](') + 2, matchedStr.length - 1);
        parts.push(
          <a
            key={keyIdx++}
            href={linkUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="text-amber-400 hover:text-amber-300 underline underline-offset-2"
          >
            {linkText}
          </a>
        );
      }

      lastIndex = pattern.lastIndex;
    }

    if (lastIndex < remaining.length) {
      parts.push(remaining.substring(lastIndex));
    }

    return parts;
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Handle code blocks
    if (line.trim().startsWith('```')) {
      if (inCodeBlock) {
        // Close code block
        elements.push(
          <pre
            key={`code-${i}`}
            className="p-3 my-2 rounded-lg bg-neutral-950 border border-neutral-800 font-mono text-xs text-neutral-200 overflow-x-auto"
          >
            <code>{codeBlockLines.join('\n')}</code>
          </pre>
        );
        codeBlockLines = [];
        inCodeBlock = false;
      } else {
        inCodeBlock = true;
      }
      continue;
    }

    if (inCodeBlock) {
      codeBlockLines.push(line);
      continue;
    }

    // Horizontal rule
    if (line.trim() === '---' || line.trim() === '***') {
      elements.push(<hr key={`hr-${i}`} className="my-3 border-neutral-800" />);
      continue;
    }

    // Headings
    if (line.startsWith('# ')) {
      elements.push(
        <h1 key={`h1-${i}`} className="text-base font-bold text-neutral-100 mt-3 mb-1">
          {renderInline(line.slice(2))}
        </h1>
      );
      continue;
    }
    if (line.startsWith('## ')) {
      elements.push(
        <h2 key={`h2-${i}`} className="text-sm font-semibold text-neutral-100 mt-2.5 mb-1">
          {renderInline(line.slice(3))}
        </h2>
      );
      continue;
    }
    if (line.startsWith('### ')) {
      elements.push(
        <h3 key={`h3-${i}`} className="text-xs font-semibold text-amber-300/90 mt-2 mb-0.5">
          {renderInline(line.slice(4))}
        </h3>
      );
      continue;
    }

    // Blockquote
    if (line.startsWith('> ')) {
      elements.push(
        <blockquote
          key={`quote-${i}`}
          className="border-l-2 border-amber-500/60 pl-3 my-1 text-xs text-neutral-400 italic"
        >
          {renderInline(line.slice(2))}
        </blockquote>
      );
      continue;
    }

    // Bullet list
    if (line.trim().startsWith('- ') || line.trim().startsWith('* ')) {
      elements.push(
        <li key={`li-${i}`} className="ml-4 list-disc text-xs text-neutral-300/90 my-0.5">
          {renderInline(line.trim().slice(2))}
        </li>
      );
      continue;
    }

    // Numbered list
    const numMatch = line.trim().match(/^(\d+)\.\s+(.*)/);
    if (numMatch) {
      elements.push(
        <li key={`ol-${i}`} className="ml-4 list-decimal text-xs text-neutral-300/90 my-0.5">
          {renderInline(numMatch[2])}
        </li>
      );
      continue;
    }

    // Empty line
    if (!line.trim()) {
      elements.push(<div key={`empty-${i}`} className="h-2" />);
      continue;
    }

    // Normal paragraph
    elements.push(
      <p key={`p-${i}`} className="text-xs text-neutral-300/90 leading-relaxed my-0.5">
        {renderInline(line)}
      </p>
    );
  }

  // Close unclosed code block if file ends
  if (inCodeBlock && codeBlockLines.length > 0) {
    elements.push(
      <pre
        key="code-unclosed"
        className="p-3 my-2 rounded-lg bg-neutral-950 border border-neutral-800 font-mono text-xs text-neutral-200 overflow-x-auto"
      >
        <code>{codeBlockLines.join('\n')}</code>
      </pre>
    );
  }

  return <div className={`markdown-body space-y-0.5 ${className}`}>{elements}</div>;
};
