import type {
  EmbedRequest,
  GenerateRequest,
  InstallModelRequest,
  InstallProgress,
  LoadModelRequest,
  LocalModelRuntime,
  ModelBenchmark,
  ModelHandle,
  ModelInfo,
  RuntimeHealth,
  TokenEvent,
  ImportModelRequest
} from '@synthesize/shared-types';

export type EndpointClassification = 'local' | 'private-lan' | 'remote';

export function classifyEndpoint(endpointUrl: string): EndpointClassification {
  try {
    const url = new URL(endpointUrl);
    const host = url.hostname.toLowerCase();
    if (host === 'localhost' || host === '127.0.0.1' || host === '::1' || host === '[::1]') return 'local';
    if (host.startsWith('10.') || host.startsWith('192.168.') || host.endsWith('.local')) return 'private-lan';
    if (host.startsWith('172.')) {
      const second = Number(host.split('.')[1]);
      if (second >= 16 && second <= 31) return 'private-lan';
    }
    return 'remote';
  } catch {
    return 'remote';
  }
}

export class LocalOpenAICompatibleRuntimeAdapter implements LocalModelRuntime {
  id = 'local-model-server-openai-compatible';
  label = 'Local Model Server (OpenAI-compatible local HTTP)';

  constructor(private readonly config: { endpointUrl: string; model: string; timeoutMs?: number }) {}

  private baseUrl(): string {
    return this.config.endpointUrl.replace(/\/$/, '');
  }

  async listInstalledModels(): Promise<ModelInfo[]> {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.config.timeoutMs ?? 10000);
    try {
      const response = await fetch(`${this.baseUrl()}/models`, { signal: controller.signal });
      if (!response.ok) throw new Error(`model list failed: HTTP ${response.status}`);
      const json = await response.json() as { data?: Array<{ id: string }> };
      return (json.data ?? []).map((m) => ({
        id: m.id,
        name: m.id,
        runtime: 'openai-compatible',
        format: 'remote-compatible',
        endpoint: this.baseUrl(),
        contextWindow: 32768,
        supportsJsonMode: true,
        supportsToolCalling: false,
        supportsEmbeddings: false,
        family: 'custom'
      }));
    } finally {
      clearTimeout(timeout);
    }
  }

  async importModel(_req: ImportModelRequest): Promise<ModelInfo> {
    return {
      id: this.config.model,
      name: this.config.model,
      runtime: 'openai-compatible',
      format: 'remote-compatible',
      endpoint: this.baseUrl(),
      contextWindow: 32768,
      supportsJsonMode: true,
      supportsToolCalling: false,
      supportsEmbeddings: false,
      family: 'custom'
    };
  }

  async *installModel(_req: InstallModelRequest): AsyncIterable<InstallProgress> {
    yield { phase: 'done', message: 'endpoint runtime does not download models' };
  }

  async loadModel(req: LoadModelRequest): Promise<ModelHandle> {
    return { modelId: req.modelId, runtimeId: this.id, loadedAt: new Date().toISOString() };
  }

  async unloadModel(_modelId: string): Promise<void> {}

  async *generate(req: GenerateRequest): AsyncIterable<TokenEvent> {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.config.timeoutMs ?? 120000);
    try {
      const body: Record<string, unknown> = {
        model: this.config.model || req.modelId,
        messages: req.messages,
        temperature: req.temperature,
        max_tokens: req.maxTokens,
        stream: false
      };
      if (req.responseFormat?.type === 'json_schema') {
        body.response_format = { type: 'json_object' };
      }
      const response = await fetch(`${this.baseUrl()}/chat/completions`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        signal: controller.signal
      });
      if (!response.ok) {
        const text = await response.text().catch(() => '');
        yield { type: 'error', message: `endpoint generation failed: HTTP ${response.status} ${text.slice(0, 500)}` };
        return;
      }
      const json = await response.json() as { choices?: Array<{ message?: { content?: string } }>; usage?: { prompt_tokens?: number; completion_tokens?: number } };
      const content = json.choices?.[0]?.message?.content ?? '';
      yield { type: 'token', text: content };
      yield { type: 'usage', inputTokens: json.usage?.prompt_tokens ?? 0, outputTokens: json.usage?.completion_tokens ?? content.length };
      yield { type: 'done' };
    } catch (error) {
      yield { type: 'error', message: error instanceof Error ? error.message : String(error) };
    } finally {
      clearTimeout(timeout);
    }
  }

  async embed(_req: EmbedRequest): Promise<number[][]> {
    return [];
  }

  async health(): Promise<RuntimeHealth> {
    try {
      const models = await this.listInstalledModels();
      return { status: 'ready', endpoint: this.baseUrl(), message: `endpoint reachable; ${models.length} model(s) listed` };
    } catch (error) {
      return { status: 'failed', endpoint: this.baseUrl(), message: error instanceof Error ? error.message : String(error) };
    }
  }

  async benchmark(modelId: string): Promise<ModelBenchmark> {
    return { modelId, promptTokensPerSecond: 0, generationTokensPerSecond: 0 };
  }
}

export class FakeRuntimeAdapter implements LocalModelRuntime {
  id = 'fake-runtime';
  label = 'Fake Runtime Adapter';

  private models: ModelInfo[] = [{
    id: 'fixture-patcher',
    name: 'Fixture Patcher',
    runtime: 'openai-compatible',
    format: 'remote-compatible',
    endpoint: 'memory://fixture-patcher',
    contextWindow: 8192,
    supportsJsonMode: true,
    supportsToolCalling: false,
    supportsEmbeddings: false,
    family: 'custom',
    installedAt: new Date(0).toISOString()
  }];

  async listInstalledModels(): Promise<ModelInfo[]> { return this.models; }

  async importModel(req: ImportModelRequest): Promise<ModelInfo> {
    const model: ModelInfo = {
      id: req.metadata?.id ?? `imported-${Date.now()}`,
      name: req.metadata?.name ?? 'Imported Model',
      runtime: req.runtime,
      format: req.metadata?.format ?? 'gguf',
      localPath: req.path,
      contextWindow: req.metadata?.contextWindow ?? 8192,
      supportsJsonMode: req.metadata?.supportsJsonMode ?? false,
      supportsToolCalling: req.metadata?.supportsToolCalling ?? false,
      supportsEmbeddings: req.metadata?.supportsEmbeddings ?? false,
      ...req.metadata
    };
    this.models.push(model);
    return model;
  }

  async *installModel(_req: InstallModelRequest): AsyncIterable<InstallProgress> {
    yield { phase: 'metadata', message: 'fake runtime: metadata loaded' };
    yield { phase: 'done', message: 'fake runtime: no download required' };
  }

  async loadModel(req: LoadModelRequest): Promise<ModelHandle> {
    return { modelId: req.modelId, runtimeId: this.id, loadedAt: new Date().toISOString() };
  }

  async unloadModel(_modelId: string): Promise<void> {}

  async *generate(req: GenerateRequest): AsyncIterable<TokenEvent> {
    const prompt = req.messages.map((m) => m.content).join('\n');
    const beforeSha256 = prompt.match(/beforeSha256=([^\s]+)/)?.[1] ?? 'fixture-before-sha256';
    const currentCommit = prompt.match(/currentCommit=([^\s]*)/)?.[1] || undefined;
    const currentFile = prompt.match(/currentFile=([^\s]+)/)?.[1] ?? 'src/auth/refresh.ts';
    const proposalId = `fixture-${Date.now()}`;
    const payload = JSON.stringify({
      operations: [{
        type: 'propose_patch',
        proposalId,
        summary: 'Replace throwing refreshToken stub with a deterministic return value.',
        baseCommit: 'fixture-base',
        currentCommit,
        files: [{ id: `${proposalId}-file-001`, path: currentFile, beforeSha256, patch: makeFixturePatch(currentFile) }],
        riskNotes: ['Fixture patch only. Backend validates file hash, path, lifecycle, approval, checkpoint, and rollback.'],
        suggestedCommands: [{
          type: 'run_command',
          argv: ['pnpm', 'test', 'auth'],
          cwd: '.',
          reason: 'Verify auth refresh behavior.',
          expectedOutcome: 'Auth tests pass.',
          requiresNetwork: false,
          mayModifyFiles: false
        }]
      }]
    });
    yield { type: 'token', text: payload };
    yield { type: 'usage', inputTokens: 0, outputTokens: payload.length };
    yield { type: 'done' };
  }

  async embed(_req: EmbedRequest): Promise<number[][]> { return []; }
  async health(): Promise<RuntimeHealth> { return { status: 'ready', message: 'fake runtime is deterministic and in-memory' }; }
  async benchmark(modelId: string): Promise<ModelBenchmark> { return { modelId, promptTokensPerSecond: 0, generationTokensPerSecond: 0 }; }
}

function makeFixturePatch(path: string): string {
  return `diff --git a/${path} b/${path}\n--- a/${path}\n+++ b/${path}\n@@ -1,3 +1,3 @@\n export function refreshToken() {\n-  throw new Error("not implemented");\n+  return "refreshed";\n }\n`;
}

// ---------------------------------------------------------------------------
// Cloud Runtime Adapters
// API keys are read from environment variables only — never hardcoded.
// Endpoints are cloud-only (remote classification) and require explicit
// user approval via the backend approval gate before repo context is sent.
// ---------------------------------------------------------------------------

export type CloudProviderConfig = {
  /** API key loaded from env var at runtime — e.g. process.env.OPENAI_API_KEY */
  apiKey: string;
  model: string;
  timeoutMs?: number;
};

/**
 * CloudOpenAIAdapter routes requests to the OpenAI API using the
 * OpenAI-compatible chat completions endpoint. Reserved for heavy-lift tasks
 * that local Qwen3 skill agents hand off due to complexity or context size.
 */
export class CloudOpenAIAdapter implements LocalModelRuntime {
  id = 'cloud-openai';
  label = 'OpenAI Cloud (heavy-lift frontier)';
  private readonly baseUrl = 'https://api.openai.com/v1';

  constructor(private readonly config: CloudProviderConfig) {}

  async listInstalledModels(): Promise<ModelInfo[]> {
    return [
      { id: 'gpt-4o', name: 'GPT-4o', runtime: 'cloud-openai', format: 'remote-compatible', endpoint: this.baseUrl, contextWindow: 128000, supportsJsonMode: true, supportsToolCalling: true, supportsEmbeddings: false, family: 'gpt', skillTier: 'cloud-heavy' },
      { id: 'o3', name: 'o3', runtime: 'cloud-openai', format: 'remote-compatible', endpoint: this.baseUrl, contextWindow: 200000, supportsJsonMode: true, supportsToolCalling: true, supportsEmbeddings: false, family: 'gpt', skillTier: 'cloud-reasoning' }
    ];
  }

  async importModel(_req: ImportModelRequest): Promise<ModelInfo> {
    return { id: this.config.model, name: this.config.model, runtime: 'cloud-openai', format: 'remote-compatible', endpoint: this.baseUrl, contextWindow: 128000, supportsJsonMode: true, supportsToolCalling: true, supportsEmbeddings: false, family: 'gpt' };
  }

  async *installModel(_req: InstallModelRequest): AsyncIterable<InstallProgress> {
    yield { phase: 'done', message: 'cloud provider: no local download required' };
  }

  async loadModel(req: LoadModelRequest): Promise<ModelHandle> {
    return { modelId: req.modelId, runtimeId: this.id, loadedAt: new Date().toISOString() };
  }

  async unloadModel(_modelId: string): Promise<void> {}

  async *generate(req: GenerateRequest): AsyncIterable<TokenEvent> {
    if (!this.config.apiKey) {
      yield { type: 'error', message: 'OPENAI_API_KEY is not set. Configure it in Settings or as an environment variable.' };
      return;
    }
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.config.timeoutMs ?? 180000);
    try {
      const body: Record<string, unknown> = {
        model: this.config.model,
        messages: req.messages,
        temperature: req.temperature,
        max_tokens: req.maxTokens,
        stream: false
      };
      if (req.responseFormat?.type === 'json_schema') {
        body.response_format = { type: 'json_object' };
      }
      const response = await fetch(`${this.baseUrl}/chat/completions`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${this.config.apiKey}` },
        body: JSON.stringify(body),
        signal: controller.signal
      });
      if (!response.ok) {
        const text = await response.text().catch(() => '');
        yield { type: 'error', message: `OpenAI API error: HTTP ${response.status} ${text.slice(0, 500)}` };
        return;
      }
      const json = await response.json() as { choices?: Array<{ message?: { content?: string } }>; usage?: { prompt_tokens?: number; completion_tokens?: number } };
      const content = json.choices?.[0]?.message?.content ?? '';
      yield { type: 'token', text: content };
      yield { type: 'usage', inputTokens: json.usage?.prompt_tokens ?? 0, outputTokens: json.usage?.completion_tokens ?? content.length };
      yield { type: 'done' };
    } catch (error) {
      yield { type: 'error', message: error instanceof Error ? error.message : String(error) };
    } finally {
      clearTimeout(timeout);
    }
  }

  async embed(_req: EmbedRequest): Promise<number[][]> { return []; }

  async health(): Promise<RuntimeHealth> {
    if (!this.config.apiKey) return { status: 'failed', message: 'OPENAI_API_KEY not configured' };
    return { status: 'ready', endpoint: this.baseUrl, message: 'OpenAI cloud endpoint (remote; requires explicit approval before repo context is sent)' };
  }

  async benchmark(modelId: string): Promise<ModelBenchmark> {
    return { modelId, promptTokensPerSecond: 0, generationTokensPerSecond: 0 };
  }
}

/**
 * CloudAnthropicAdapter routes requests to the Anthropic Messages API.
 * Anthropic does not expose an OpenAI-compatible endpoint natively,
 * so this adapter translates the request format.
 * Reserved for heavy-lift tasks handed off by local Qwen3 skill agents.
 */
export class CloudAnthropicAdapter implements LocalModelRuntime {
  id = 'cloud-anthropic';
  label = 'Anthropic Cloud (heavy-lift frontier)';
  private readonly baseUrl = 'https://api.anthropic.com/v1';

  constructor(private readonly config: CloudProviderConfig) {}

  async listInstalledModels(): Promise<ModelInfo[]> {
    return [
      { id: 'claude-sonnet-4-5', name: 'Claude Sonnet (Anthropic)', runtime: 'cloud-anthropic', format: 'remote-compatible', endpoint: this.baseUrl, contextWindow: 200000, supportsJsonMode: true, supportsToolCalling: true, supportsEmbeddings: false, family: 'claude', skillTier: 'cloud-heavy' }
    ];
  }

  async importModel(_req: ImportModelRequest): Promise<ModelInfo> {
    return { id: this.config.model, name: this.config.model, runtime: 'cloud-anthropic', format: 'remote-compatible', endpoint: this.baseUrl, contextWindow: 200000, supportsJsonMode: true, supportsToolCalling: true, supportsEmbeddings: false, family: 'claude' };
  }

  async *installModel(_req: InstallModelRequest): AsyncIterable<InstallProgress> {
    yield { phase: 'done', message: 'cloud provider: no local download required' };
  }

  async loadModel(req: LoadModelRequest): Promise<ModelHandle> {
    return { modelId: req.modelId, runtimeId: this.id, loadedAt: new Date().toISOString() };
  }

  async unloadModel(_modelId: string): Promise<void> {}

  async *generate(req: GenerateRequest): AsyncIterable<TokenEvent> {
    if (!this.config.apiKey) {
      yield { type: 'error', message: 'ANTHROPIC_API_KEY is not set. Configure it in Settings or as an environment variable.' };
      return;
    }
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.config.timeoutMs ?? 180000);
    try {
      const systemMessages = req.messages.filter((m) => m.role === 'system').map((m) => m.content).join('\n');
      const userMessages = req.messages.filter((m) => m.role !== 'system').map((m) => ({ role: m.role === 'assistant' ? 'assistant' : 'user', content: m.content }));
      const body: Record<string, unknown> = {
        model: this.config.model,
        max_tokens: req.maxTokens,
        messages: userMessages
      };
      if (systemMessages) body.system = systemMessages;
      const response = await fetch(`${this.baseUrl}/messages`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': this.config.apiKey,
          'anthropic-version': '2023-06-01'
        },
        body: JSON.stringify(body),
        signal: controller.signal
      });
      if (!response.ok) {
        const text = await response.text().catch(() => '');
        yield { type: 'error', message: `Anthropic API error: HTTP ${response.status} ${text.slice(0, 500)}` };
        return;
      }
      const json = await response.json() as { content?: Array<{ type: string; text?: string }>; usage?: { input_tokens?: number; output_tokens?: number } };
      const content = json.content?.find((b) => b.type === 'text')?.text ?? '';
      yield { type: 'token', text: content };
      yield { type: 'usage', inputTokens: json.usage?.input_tokens ?? 0, outputTokens: json.usage?.output_tokens ?? content.length };
      yield { type: 'done' };
    } catch (error) {
      yield { type: 'error', message: error instanceof Error ? error.message : String(error) };
    } finally {
      clearTimeout(timeout);
    }
  }

  async embed(_req: EmbedRequest): Promise<number[][]> { return []; }

  async health(): Promise<RuntimeHealth> {
    if (!this.config.apiKey) return { status: 'failed', message: 'ANTHROPIC_API_KEY not configured' };
    return { status: 'ready', endpoint: this.baseUrl, message: 'Anthropic cloud endpoint (remote; requires explicit approval before repo context is sent)' };
  }

  async benchmark(modelId: string): Promise<ModelBenchmark> {
    return { modelId, promptTokensPerSecond: 0, generationTokensPerSecond: 0 };
  }
}
