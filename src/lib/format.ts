/** Escape first, then lift a small markdown subset into inert HTML. */

export function formatMessage(content: string): string {
  if (content == null) return "";
  const raw = typeof content === "string" ? content : String(content);
  if (raw.trim() === "") return "";

  const escaped = escapeHtml(raw);
  const { text, fences } = extractFences(escaped);
  let html = renderBlocks(text);
  html = restoreFences(html, fences);
  return html;
}

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

type Fence = { lang: string; body: string };

function extractFences(text: string): { text: string; fences: Fence[] } {
  const fences: Fence[] = [];
  const out = text.replace(/```([^\n`]*)\n([\s\S]*?)```/g, (_m, lang: string, body: string) => {
    const i = fences.length;
    fences.push({ lang: lang.trim(), body: body.replace(/\n$/, "") });
    return `\u0000FENCE${i}\u0000`;
  });
  return { text: out, fences };
}

function restoreFences(html: string, fences: Fence[]): string {
  return html.replace(/\u0000FENCE(\d+)\u0000/g, (_m, n: string) => {
    const f = fences[Number(n)];
    if (!f) return "";
    const lang = f.lang ? ` data-lang="${f.lang}"` : "";
    return `<pre class="md-pre"><code${lang}>${f.body}</code></pre>`;
  });
}

function renderBlocks(text: string): string {
  const lines = text.split("\n");
  const parts: string[] = [];
  let i = 0;

  const flushPara = (buf: string[]) => {
    if (!buf.length) return;
    parts.push(`<p class="md-p">${buf.map(inline).join("<br>")}</p>`);
    buf.length = 0;
  };

  while (i < lines.length) {
    const line = lines[i];
    const heading = line.match(/^(#{1,6})\s+(.+)$/);
    if (heading) {
      const level = heading[1].length;
      parts.push(`<h${level} class="md-h">${inline(heading[2])}</h${level}>`);
      i += 1;
      continue;
    }

    const ul = line.match(/^\s*[-*+]\s+(.+)$/);
    if (ul) {
      const items: string[] = [];
      while (i < lines.length) {
        const m = lines[i].match(/^\s*[-*+]\s+(.+)$/);
        if (!m) break;
        items.push(`<li>${inline(m[1])}</li>`);
        i += 1;
      }
      parts.push(`<ul class="md-ul">${items.join("")}</ul>`);
      continue;
    }

    const ol = line.match(/^\s*\d+\.\s+(.+)$/);
    if (ol) {
      const items: string[] = [];
      while (i < lines.length) {
        const m = lines[i].match(/^\s*\d+\.\s+(.+)$/);
        if (!m) break;
        items.push(`<li>${inline(m[1])}</li>`);
        i += 1;
      }
      parts.push(`<ol class="md-ol">${items.join("")}</ol>`);
      continue;
    }

    if (line.trim() === "") {
      i += 1;
      continue;
    }

    const para: string[] = [];
    while (i < lines.length) {
      const l = lines[i];
      if (l.trim() === "") break;
      if (/^(#{1,6})\s+/.test(l)) break;
      if (/^\s*[-*+]\s+/.test(l)) break;
      if (/^\s*\d+\.\s+/.test(l)) break;
      para.push(l);
      i += 1;
    }
    flushPara(para);
  }

  return parts.join("");
}

function inline(s: string): string {
  let t = s;
  t = t.replace(/`([^`]+)`/g, '<code class="md-code">$1</code>');
  t = t.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_m, label: string, href: string) => {
    const safe = safeHref(href);
    return `<a class="md-a" href="${safe}" rel="noopener noreferrer">${label}</a>`;
  });
  t = t.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  t = t.replace(/__([^_]+)__/g, '<strong>$1</strong>');
  t = t.replace(/\*([^*]+)\*/g, '<em>$1</em>');
  t = t.replace(/_([^_]+)_/g, '<em>$1</em>');
  return t;
}

function safeHref(href: string): string {
  const raw = href.replace(/&amp;/g, "&").trim();
  const lower = raw.toLowerCase();
  if (lower.startsWith("https:") || lower.startsWith("http:") || lower.startsWith("mailto:")) {
    return escapeHtml(raw);
  }
  return "#";
}
