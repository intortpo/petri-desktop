/**
 * EverOS client & memory unification layer for Petri.
 * Coordinates with the Python EverOS sidecar via Tauri IPC.
 */

export type EverosHealth = {
  status: string;
  engine: string;
  has_everos_pkg: boolean;
  storage_root: string;
  memory_count: number;
  capabilities: {
    markdown_native: boolean;
    local_storage: boolean;
    fts_search: boolean;
    vector_search: boolean;
  };
};

export type EverosMemoryHit = {
  id: string;
  type: string;
  source_path: string;
  session_id: string | null;
  content: string;
  created_at: number;
  similarity: number;
};

export type EverosFlushResult = {
  status: string;
  flushed: boolean;
  session_id?: string;
  episode_path?: string;
  turns_written?: number;
  facts_extracted?: number;
  reason?: string;
};

// Fallback in-memory state for non-Tauri / test environments
let mockMemories: EverosMemoryHit[] = [];

async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)) {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      return await invoke<T>(cmd, args);
    } catch {
      // Fallback
    }
  }
  throw new Error("Tauri IPC not available in this environment");
}

export async function checkEverosHealth(): Promise<EverosHealth> {
  try {
    return await invokeTauri<EverosHealth>("everos_status");
  } catch {
    return {
      status: "ok",
      engine: "everos-mock-fallback",
      has_everos_pkg: false,
      storage_root: "~/.mesh/everos",
      memory_count: mockMemories.length,
      capabilities: {
        markdown_native: true,
        local_storage: true,
        fts_search: true,
        vector_search: false,
      },
    };
  }
}

export async function addEverosTurn(
  sessionId: string,
  role: string,
  content: string,
): Promise<void> {
  try {
    await invokeTauri("everos_add_turn", { sessionId, role, content });
  } catch {
    // Mock record for fallback
    mockMemories.push({
      id: `mock_${Date.now()}`,
      type: "turn",
      source_path: "mock://episodes",
      session_id: sessionId,
      content: `[${role}] ${content}`,
      created_at: Date.now(),
      similarity: 1.0,
    });
  }
}

export async function rememberEveros(
  lesson: string,
  domain: string = "general",
): Promise<void> {
  try {
    await invokeTauri("everos_remember", { lesson, domain });
  } catch {
    mockMemories.push({
      id: `fact_${Date.now()}`,
      type: "fact",
      source_path: "mock://profile/facts.md",
      session_id: null,
      content: `[${domain}] ${lesson}`,
      created_at: Date.now(),
      similarity: 1.0,
    });
  }
}

export async function flushEveros(sessionId: string): Promise<EverosFlushResult> {
  try {
    return await invokeTauri<EverosFlushResult>("everos_flush", { sessionId });
  } catch {
    return {
      status: "ok",
      flushed: true,
      session_id: sessionId,
      episode_path: `~/.mesh/everos/episodes/${sessionId}.md`,
      turns_written: 1,
      facts_extracted: 1,
    };
  }
}

export async function searchEveros(
  query: string,
  topK: number = 5,
): Promise<EverosMemoryHit[]> {
  try {
    return await invokeTauri<EverosMemoryHit[]>("everos_search", { query, topK });
  } catch {
    const q = query.toLowerCase();
    return mockMemories
      .filter((m) => m.content.toLowerCase().includes(q))
      .slice(0, topK);
  }
}

export async function getEverosProfile(): Promise<string> {
  try {
    return await invokeTauri<string>("everos_get_profile");
  } catch {
    return "# EverOS User Profile\n\n- [mock] Running in local fallback environment\n";
  }
}

export function formatEverosRecall(hits: EverosMemoryHit[]): string {
  if (!hits.length) return "No matching EverOS memories found.";
  return hits
    .map((h) => `• [${h.type.toUpperCase()}] ${h.content.trim()} (score: ${h.similarity})`)
    .join("\n");
}
