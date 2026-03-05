//! LLM client abstraction for parity failure analysis.
//!
//! Supports Anthropic Messages API and OpenAI Chat Completions API via reqwest.
//! Backend is selected by environment variable: `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`.

use serde::{Deserialize, Serialize};

/// Parsed LLM analysis of a failure cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAnalysis {
    pub mechanic: String,
    pub root_cause: String,
    pub files_to_check: Vec<String>,
    pub repro_command: String,
    pub severity: String,
}

/// Which LLM backend to use.
enum Backend {
    Anthropic { api_key: String },
    OpenAi { api_key: String, base_url: String, model: String },
    ClaudeCode { binary: String },
}

/// LLM client that calls either Anthropic, OpenAI, or Claude Code CLI for analysis.
pub struct LlmClient {
    backend: Backend,
    client: reqwest::Client,
}

/// Contextual information about a failure cluster for the prompt.
pub struct ClusterContext {
    pub count: usize,
    pub divergence_field: String,
    pub rust_value: String,
    pub java_value: String,
    pub deck_pairs: String,
    pub covered_cards: String,
    pub sample_seeds: String,
}

impl LlmClient {
    /// Create a new LLM client from environment variables.
    ///
    /// Priority: `ANTHROPIC_API_KEY` > `OPENAI_API_KEY` > `claude` CLI (if found on PATH).
    pub fn from_env() -> Option<Self> {
        let client = reqwest::Client::new();

        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                return Some(Self {
                    backend: Backend::Anthropic { api_key: key },
                    client,
                });
            }
        }
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            if !key.is_empty() {
                let base_url = std::env::var("OPENAI_API_BASE")
                    .unwrap_or_else(|_| "https://api.openai.com".to_string());
                let model = std::env::var("OPENAI_MODEL")
                    .unwrap_or_else(|_| "gpt-4o-mini".to_string());
                return Some(Self {
                    backend: Backend::OpenAi { api_key: key, base_url, model },
                    client,
                });
            }
        }
        // Fall back to Claude Code CLI if available
        if let Ok(output) = std::process::Command::new("claude").arg("--version").output() {
            if output.status.success() {
                return Some(Self {
                    backend: Backend::ClaudeCode {
                        binary: "claude".to_string(),
                    },
                    client,
                });
            }
        }
        None
    }

    /// Analyze a failure cluster and return structured analysis.
    pub async fn analyze_cluster(&self, ctx: &ClusterContext) -> Result<LlmAnalysis, String> {
        let prompt = build_prompt(ctx);
        let raw = match &self.backend {
            Backend::Anthropic { api_key } => {
                self.call_anthropic(api_key, &prompt).await?
            }
            Backend::OpenAi { api_key, base_url, model } => {
                self.call_openai(api_key, base_url, model, &prompt).await?
            }
            Backend::ClaudeCode { binary } => {
                self.call_claude_code(binary, &prompt).await?
            }
        };

        // Extract JSON from the response (may be wrapped in markdown code fences)
        let json_str = extract_json(&raw);
        serde_json::from_str::<LlmAnalysis>(json_str)
            .map_err(|e| format!("Failed to parse LLM JSON: {e}\nRaw: {raw}"))
    }

    async fn call_anthropic(&self, api_key: &str, prompt: &str) -> Result<String, String> {
        let body = serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Anthropic request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Anthropic API error {status}: {text}"));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Anthropic response parse error: {e}"))?;

        json["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Unexpected Anthropic response shape: {json}"))
    }

    async fn call_claude_code(&self, binary: &str, prompt: &str) -> Result<String, String> {
        let binary = binary.to_string();
        let prompt = prompt.to_string();
        tokio::task::spawn_blocking(move || {
            let output = std::process::Command::new(&binary)
                .args(["-p", &prompt, "--output-format", "text"])
                .env_remove("CLAUDECODE")
                .output()
                .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("claude CLI error: {stderr}"));
            }

            String::from_utf8(output.stdout)
                .map_err(|e| format!("claude CLI output not UTF-8: {e}"))
        })
        .await
        .map_err(|e| format!("spawn_blocking failed: {e}"))?
    }

    async fn call_openai(&self, api_key: &str, base_url: &str, model: &str, prompt: &str) -> Result<String, String> {
        let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": model,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "max_tokens": 1024
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("OpenAI request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("OpenAI API error {status}: {text}"));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("OpenAI response parse error: {e}"))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Unexpected OpenAI response shape: {json}"))
    }
}

fn build_prompt(ctx: &ClusterContext) -> String {
    format!(
        r#"You are a game engine parity analyst. The Rust Forge engine is being compared against the Java Forge reference implementation for the card game Magic: The Gathering.

A cluster of {count} failures was detected:

Divergence field: {field}
Rust value: {rust}
Java value: {java}
Affected deck pairs: {decks}
Cards involved: {cards}
Sample seeds (reproducible): {seeds}

Analyze this failure pattern:
1. What game mechanic is likely involved?
2. What is the probable root cause (Rust engine bug)?
3. Which Rust source files should be investigated?
4. Suggested reproduction command.

Respond in JSON only (no markdown fences): {{"mechanic": "...", "root_cause": "...", "files_to_check": [...], "repro_command": "...", "severity": "high|medium|low"}}"#,
        count = ctx.count,
        field = ctx.divergence_field,
        rust = ctx.rust_value,
        java = ctx.java_value,
        decks = ctx.deck_pairs,
        cards = ctx.covered_cards,
        seeds = ctx.sample_seeds,
    )
}

/// Extract JSON from a string that may contain markdown code fences.
fn extract_json(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return &trimmed[start..=end];
        }
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_from_fenced() {
        let input = "```json\n{\"mechanic\": \"combat\"}\n```";
        assert_eq!(extract_json(input), r#"{"mechanic": "combat"}"#);
    }

    #[test]
    fn extract_json_plain() {
        let input = r#"{"mechanic": "combat", "root_cause": "test"}"#;
        assert_eq!(extract_json(input), input);
    }

    #[test]
    fn parse_analysis() {
        let json = r#"{"mechanic":"damage","root_cause":"off-by-one","files_to_check":["action.rs"],"repro_command":"cargo run","severity":"high"}"#;
        let analysis: LlmAnalysis = serde_json::from_str(json).unwrap();
        assert_eq!(analysis.mechanic, "damage");
        assert_eq!(analysis.severity, "high");
    }
}
