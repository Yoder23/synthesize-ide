export type RuntimeKind =
  | 'llamacpp'
  | 'vllm'
  | 'mlx'
  | 'transformers'
  | 'ollama'
  | 'lmstudio'
  | 'openai-compatible'
  | 'cloud-openai'
  | 'cloud-anthropic';

export type SkillTier = 'fast' | 'balanced' | 'powerful' | 'frontier-local' | 'cloud-heavy' | 'cloud-reasoning';

export type ModelFormat = 'gguf' | 'safetensors' | 'mlx' | 'remote-compatible';

export type ModelInfo = {
  id: string;
  name: string;
  runtime: RuntimeKind;
  format: ModelFormat;
  localPath?: string;
  endpoint?: string;
  family?: 'qwen' | 'deepseek' | 'llama' | 'starcoder' | 'codestral' | 'gpt' | 'claude' | 'custom';
  skillTier?: SkillTier;
  apiKeyEnvVar?: string;
  contextWindow: number;
  supportsJsonMode: boolean;
  supportsToolCalling: boolean;
  supportsEmbeddings: boolean;
  quantization?: string;
  license?: string;
  sha256?: string;
  diskSizeBytes?: number;
  recommendedRamGb?: number;
  recommendedVramGb?: number;
  installedAt?: string;
  lastUsedAt?: string;
};

export type ChatMessage = {
  role: 'system' | 'user' | 'assistant' | 'tool';
  content: string;
  name?: string;
};

export type GenerateRequest = {
  modelId: string;
  messages: ChatMessage[];
  temperature: number;
  maxTokens: number;
  stop?: string[];
  responseFormat?: { type: 'text' | 'json_schema'; schema?: unknown };
  sessionId: string;
  agentId: string;
  cancellationToken: string;
};

export type TokenEvent =
  | { type: 'token'; text: string }
  | { type: 'usage'; inputTokens: number; outputTokens: number }
  | { type: 'done' }
  | { type: 'error'; message: string };

export interface LocalModelRuntime {
  id: string;
  label: string;
  listInstalledModels(): Promise<ModelInfo[]>;
  importModel(req: ImportModelRequest): Promise<ModelInfo>;
  installModel(req: InstallModelRequest): AsyncIterable<InstallProgress>;
  loadModel(req: LoadModelRequest): Promise<ModelHandle>;
  unloadModel(modelId: string): Promise<void>;
  generate(req: GenerateRequest): AsyncIterable<TokenEvent>;
  embed?(req: EmbedRequest): Promise<number[][]>;
  health(): Promise<RuntimeHealth>;
  benchmark?(modelId: string): Promise<ModelBenchmark>;
}

export type ImportModelRequest = { path: string; runtime: RuntimeKind; metadata?: Partial<ModelInfo> };
export type InstallModelRequest = { registryId: string; quantization?: string };
export type InstallProgress = { phase: 'metadata' | 'download' | 'verify' | 'register' | 'done' | 'error'; bytesDone?: number; bytesTotal?: number; message?: string };
export type LoadModelRequest = { modelId: string; contextWindow?: number; gpuLayers?: number };
export type ModelHandle = { modelId: string; runtimeId: string; loadedAt: string };
export type EmbedRequest = { modelId: string; texts: string[] };
export type RuntimeHealth = { status: 'starting' | 'ready' | 'busy' | 'failed' | 'stopped'; message?: string; pid?: number; endpoint?: string };
export type ModelBenchmark = { modelId: string; promptTokensPerSecond: number; generationTokensPerSecond: number; ramBytes?: number; vramBytes?: number };
