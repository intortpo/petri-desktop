/** ReasoningBank-shaped store. Uses hashed embeddings locally; AgentDB if present. */

export type Pattern = {
  id: string;
  type: string;
  domain: string;
  pattern_data: string;
  confidence: number;
  usage_count: number;
  success_count: number;
  created_at: number;
  last_used: number;
};

export type MemoryHit = Pattern & { similarity: number; pattern?: Record<string, unknown> };

const KEY = "petri.rb.patterns";
const DIM = 64;

export function computeEmbedding(text: string, dim = DIM): number[] {
  const v = new Array(dim).fill(0);
  const s = (text || "").toLowerCase();
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    v[c % dim] += 1;
    v[(c * 7 + i) % dim] += 0.35;
  }
  let n = 0;
  for (const x of v) n += x * x;
  n = Math.sqrt(n) || 1;
  return v.map((x) => x / n);
}

export function cosine(a: number[], b: number[]): number {
  const n = Math.min(a.length, b.length);
  let dot = 0;
  for (let i = 0; i < n; i++) dot += a[i] * b[i];
  return dot;
}

function load(): Pattern[] {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) || "[]");
    return Array.isArray(raw) ? raw : [];
  } catch {
    return [];
  }
}

function save(rows: Pattern[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(rows.slice(-400)));
  } catch {}
}

function parsePattern(p: Pattern): Record<string, unknown> {
  try {
    const data = JSON.parse(p.pattern_data);
    return data.pattern || data;
  } catch {
    return {};
  }
}

export async function insertPattern(row: Partial<Pattern> & { pattern_data: string; domain: string }): Promise<Pattern> {
  const now = Date.now();
  const rec: Pattern = {
    id: row.id || `rb_${now.toString(36)}_${Math.random().toString(36).slice(2, 8)}`,
    type: row.type || "experience",
    domain: row.domain,
    pattern_data: row.pattern_data,
    confidence: row.confidence ?? 0.7,
    usage_count: row.usage_count ?? 1,
    success_count: row.success_count ?? 1,
    created_at: row.created_at ?? now,
    last_used: row.last_used ?? now,
  };
  const all = load();
  all.push(rec);
  save(all);
  return rec;
}

export async function retrieveWithReasoning(
  embedding: number[],
  opts: { domain?: string; k?: number; useMMR?: boolean; synthesizeContext?: boolean; minConfidence?: number } = {},
) {
  const k = opts.k ?? 5;
  let rows = load().filter((p) => (opts.domain ? p.domain === opts.domain || p.domain.startsWith(opts.domain + "/") : true));
  if (opts.minConfidence != null) rows = rows.filter((p) => p.confidence >= opts.minConfidence!);
  const scored: MemoryHit[] = rows.map((p) => {
    let emb: number[] = [];
    try {
      const data = JSON.parse(p.pattern_data);
      emb = Array.isArray(data.embedding) ? data.embedding : computeEmbedding(JSON.stringify(data.pattern || data));
    } catch {
      emb = computeEmbedding(p.pattern_data);
    }
    return { ...p, similarity: cosine(embedding, emb), pattern: parsePattern(p) };
  });
  scored.sort((a, b) => b.similarity - a.similarity);
  const memories = scored.slice(0, k);
  const context = opts.synthesizeContext
    ? memories
        .map((m) => {
          const q = String((m.pattern as { query?: string })?.query || "");
          const a = String((m.pattern as { approach?: string })?.approach || "");
          return [q, a].filter(Boolean).join(" → ");
        })
        .filter(Boolean)
        .join("\n")
    : "";
  return { memories, context, patterns: memories.map((m) => m.pattern) };
}

export function judgeTrajectory(
  similar: MemoryHit[],
): { verdict: "likely_success" | "needs_review"; confidence: number } {
  const good = similar.filter((m) => (m.pattern as { outcome?: string })?.outcome === "success" && m.similarity > 0.8);
  return {
    verdict: good.length > 2 ? "likely_success" : "needs_review",
    confidence: similar[0]?.similarity || 0,
  };
}

export async function rememberTurn(opts: {
  login: string;
  query: string;
  workspace?: string;
  window?: string;
  outcome?: string;
}) {
  const query = opts.query;
  const embedding = computeEmbedding(`${opts.login} ${query}`);
  await insertPattern({
    type: "experience",
    domain: `persona/${opts.login}`,
    pattern_data: JSON.stringify({
      embedding,
      pattern: {
        query,
        approach: "adversarial-question",
        outcome: opts.outcome || "recorded",
        workspace: opts.workspace,
        window: opts.window,
      },
    }),
    confidence: 0.8,
  });
}

export async function recallPersona(login: string, query: string): Promise<string> {
  const embedding = computeEmbedding(`${login} ${query}`);
  const result = await retrieveWithReasoning(embedding, {
    domain: `persona/${login}`,
    k: 8,
    synthesizeContext: true,
    useMMR: true,
  });
  if (!result.memories.length) return "";
  const verdict = judgeTrajectory(result.memories);
  return `[PERSONA ${login} · ${verdict.verdict}]\n${result.context}`.slice(0, 1800);
}
