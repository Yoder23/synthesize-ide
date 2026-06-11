import { OperationListSchema } from './schema';
import type { AgentOperation } from '@synthesize/shared-types';

export type ParseResult =
  | { ok: true; operations: AgentOperation[]; source: 'json' | 'fenced-json' }
  | { ok: false; error: string; raw: string };

export function parseAgentOperations(raw: string): ParseResult {
  const extracted = extractJsonPayload(raw);
  if (!extracted) return { ok: false, error: 'No strict JSON payload found in model output', raw };
  try {
    const parsed = JSON.parse(extracted.text);
    const result = OperationListSchema.safeParse(parsed);
    if (!result.success) return { ok: false, error: result.error.message, raw };
    return { ok: true, operations: result.data.operations as AgentOperation[], source: extracted.source };
  } catch (error) {
    return { ok: false, error: String(error), raw };
  }
}

function extractJsonPayload(raw: string): { text: string; source: 'json' | 'fenced-json' } | null {
  const trimmed = raw.trim();
  if (looksLikeSingleJsonObject(trimmed)) {
    return { text: trimmed, source: 'json' };
  }

  const fenced = extractFencedJson(trimmed);
  if (fenced && looksLikeSingleJsonObject(fenced)) {
    return { text: fenced, source: 'fenced-json' };
  }

  return null;
}

function extractFencedJson(raw: string): string | null {
  const match = raw.match(/```(?:json)?\s*([\s\S]*?)\s*```/i);
  return match?.[1]?.trim() ?? null;
}

function looksLikeSingleJsonObject(text: string): boolean {
  if (!text.startsWith('{') || !text.endsWith('}')) return false;
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let i = 0; i < text.length; i++) {
    const char = text[i];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (char === '\\') {
      escaped = true;
      continue;
    }
    if (char === '"') {
      inString = !inString;
      continue;
    }
    if (inString) continue;
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0 && i < text.length - 1) return false;
    if (depth < 0) return false;
  }
  return depth === 0 && !inString;
}

export { extractJsonPayload as _extractJsonPayloadForTests };
