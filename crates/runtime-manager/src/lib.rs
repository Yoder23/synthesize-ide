use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeKind {
    LlamaCpp,
    Vllm,
    Mlx,
    Transformers,
    Ollama,
    LmStudio,
    OpenAiCompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeHealth {
    pub status: RuntimeStatus,
    pub message: Option<String>,
    pub pid: Option<u32>,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeStatus {
    Starting,
    Ready,
    Busy,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub runtime: RuntimeKind,
    pub local_path: Option<String>,
    pub endpoint: Option<String>,
    pub context_window: u32,
    pub supports_json_mode: bool,
    pub supports_embeddings: bool,
}

pub trait RuntimeAdapter {
    fn id(&self) -> &str;
    fn health(&self) -> RuntimeHealth;
}

pub struct LlamaCppRuntime {
    pub server_path: String,
    pub endpoint: String,
}

impl RuntimeAdapter for LlamaCppRuntime {
    fn id(&self) -> &str { "llamacpp" }
    fn health(&self) -> RuntimeHealth {
        RuntimeHealth { status: RuntimeStatus::Stopped, message: Some("supervisor not implemented yet".into()), pid: None, endpoint: Some(self.endpoint.clone()) }
    }
}
