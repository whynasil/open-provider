use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_db_path")]
    pub database_path: String,

    #[serde(default = "default_check_interval_minutes")]
    pub check_interval_minutes: u64,

    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    pub models_endpoint: String,
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub free_rules: FreeRules,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FreeRules {
    /// Check if model ID ends with ":free" suffix
    #[serde(default)]
    pub id_suffix_colon_free: bool,
    /// Check if model ID contains "-free" pattern
    #[serde(default)]
    pub id_contains_dash_free: bool,
    /// Check if model ID contains "free" anywhere (case insensitive)
    #[serde(default)]
    pub id_contains_free: bool,
    /// Check if isFree field is true in raw data
    #[serde(default)]
    pub field_is_free: bool,
    /// Check if all pricing values are zero
    #[serde(default)]
    pub zero_pricing: bool,
}

fn default_db_path() -> String {
    "open-provider.db".into()
}

fn default_check_interval_minutes() -> u64 {
    15
}

impl Config {
    pub fn example() -> Self {
        Self {
            database_path: default_db_path(),
            check_interval_minutes: 15,
            providers: vec![
                ProviderConfig {
                    name: "OpenAI".into(),
                    base_url: "https://api.openai.com".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("OPENAI_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Anthropic".into(),
                    base_url: "https://api.anthropic.com".into(),
                    models_endpoint: "/v1/models?limit=1000".into(),
                    api_key_env: Some("ANTHROPIC_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Groq".into(),
                    base_url: "https://api.groq.com".into(),
                    models_endpoint: "/openai/v1/models".into(),
                    api_key_env: Some("GROQ_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "OpenRouter".into(),
                    base_url: "https://openrouter.ai".into(),
                    models_endpoint: "/api/v1/models".into(),
                    api_key_env: None,
                    headers: Default::default(),
                    disabled: false,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Together".into(),
                    base_url: "https://api.together.xyz".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("TOGETHER_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Google".into(),
                    base_url: "https://generativelanguage.googleapis.com".into(),
                    models_endpoint: "/v1beta/models?key={API_KEY}".into(),
                    api_key_env: Some("GOOGLE_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Mistral".into(),
                    base_url: "https://api.mistral.ai".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("MISTRAL_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "DeepSeek".into(),
                    base_url: "https://api.deepseek.com".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("DEEPSEEK_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "xAI".into(),
                    base_url: "https://api.x.ai".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("XAI_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Cohere".into(),
                    base_url: "https://api.cohere.ai".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("COHERE_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "OpenCode Zen".into(),
                    base_url: "https://opencode.ai".into(),
                    models_endpoint: "/zen/v1/models".into(),
                    api_key_env: None,
                    headers: [("Accept".into(), "application/json".into())].into(),
                    disabled: false,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "OpenCode Zen Go".into(),
                    base_url: "https://opencode.ai".into(),
                    models_endpoint: "/zen/go/v1/models".into(),
                    api_key_env: None,
                    headers: [("Accept".into(), "application/json".into())].into(),
                    disabled: false,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "CommandCode".into(),
                    base_url: "https://api.commandcode.ai".into(),
                    models_endpoint: "/provider/v1/models".into(),
                    api_key_env: None,
                    headers: [("Accept".into(), "application/json".into())].into(),
                    disabled: false,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Vercel AI Gateway".into(),
                    base_url: "https://ai-gateway.vercel.sh".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: None,
                    headers: Default::default(),
                    disabled: false,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "NVIDIA NIM".into(),
                    base_url: "https://integrate.api.nvidia.com".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: None,
                    headers: Default::default(),
                    disabled: false,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Chutes".into(),
                    base_url: "https://llm.chutes.ai".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: None,
                    headers: Default::default(),
                    disabled: false,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Ollama".into(),
                    base_url: "https://ollama.com".into(),
                    models_endpoint: "/api/tags".into(),
                    api_key_env: None,
                    headers: Default::default(),
                    disabled: false,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "BluesMinds".into(),
                    base_url: "https://api.bluesminds.com".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("BLUESMINDS_API_KEY".into()),
                    headers: Default::default(),
                    disabled: false,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Kilo AI".into(),
                    base_url: "https://api.kilo.ai".into(),
                    models_endpoint: "/api/gateway/models".into(),
                    api_key_env: None,
                    headers: Default::default(),
                    disabled: false,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Antigravity".into(),
                    base_url: "https://daily-cloudcode-pa.sandbox.googleapis.com".into(),
                    models_endpoint: "/v1internal:models".into(),
                    api_key_env: Some("ANTIGRAVITY_TOKEN".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "GitHub Copilot".into(),
                    base_url: "https://api.githubcopilot.com".into(),
                    models_endpoint: "/models".into(),
                    api_key_env: Some("GITHUB_COPILOT_TOKEN".into()),
                    headers: [
                        ("editor-version".into(), "vscode/1.107.1".into()),
                        ("Copilot-Integration-Id".into(), "vscode-chat".into()),
                        ("editor-plugin-version".into(), "copilot-chat/0.26.7".into()),
                    ].into(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Qwen Portal".into(),
                    base_url: "https://portal.qwen.ai".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("QWEN_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "SiliconFlow".into(),
                    base_url: "https://api.siliconflow.cn".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("SILICONFLOW_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Hyperbolic".into(),
                    base_url: "https://api.hyperbolic.xyz".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("HYPERBOLIC_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Nebius".into(),
                    base_url: "https://api.studio.nebius.ai".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("NEBIUS_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Cerebras".into(),
                    base_url: "https://api.cerebras.ai".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("CEREBRAS_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Fireworks".into(),
                    base_url: "https://api.fireworks.ai".into(),
                    models_endpoint: "/inference/v1/models".into(),
                    api_key_env: Some("FIREWORKS_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Perplexity".into(),
                    base_url: "https://api.perplexity.ai".into(),
                    models_endpoint: "/models".into(),
                    api_key_env: Some("PERPLEXITY_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "NanoBanana".into(),
                    base_url: "https://api.nanobananaapi.ai".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("NANOBANANA_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "AssemblyAI".into(),
                    base_url: "https://api.assemblyai.com".into(),
                    models_endpoint: "/v1/models".into(),
                    api_key_env: Some("ASSEMBLYAI_API_KEY".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
                ProviderConfig {
                    name: "Codex".into(),
                    base_url: "https://chatgpt.com".into(),
                    models_endpoint: "/backend-api/codex/models?client_version=1.0.0".into(),
                    api_key_env: Some("CODEX_TOKEN".into()),
                    headers: Default::default(),
                    disabled: true,
                    free_rules: Default::default(),
                },
            ],
        }
    }
}

pub fn load(path: &str) -> anyhow::Result<Config> {
    if Path::new(path).exists() {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    } else {
        // Write default config and use it
        let cfg = Config::example();
        std::fs::write(path, toml::to_string_pretty(&cfg)?)?;
        tracing::info!("Created default config at {path}");
        Ok(cfg)
    }
}
