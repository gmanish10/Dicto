use super::{build_full_system, sanity_check, Correction, PolishError, Polisher};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-haiku-4-5-20251001";
const VERSION: &str = "2023-06-01";

pub struct ClaudePolisher {
    api_key: String,
}

impl ClaudePolisher {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    max_tokens: u32,
    system: String,
    messages: Vec<Message<'a>>,
    temperature: f32,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: String,
}

#[derive(Deserialize)]
struct Response {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[async_trait]
impl Polisher for ClaudePolisher {
    async fn polish(&self, raw: &str, recent: &[Correction]) -> Result<String, PolishError> {
        let system = build_full_system(recent);
        let req = Request {
            model: MODEL,
            // Polish should never balloon — cap roughly proportional to input.
            max_tokens: (raw.chars().count() as u32 * 2 + 200).clamp(64, 2048),
            system,
            messages: vec![Message {
                role: "user",
                content: format!("Polish this transcript:\n\n{}", raw),
            }],
            temperature: 0.2,
        };

        let resp = reqwest::Client::new()
            .post(ENDPOINT)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", VERSION)
            .json(&req)
            .send()
            .await
            .map_err(|e| PolishError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(PolishError::Api(format!("{status}: {body}")));
        }

        let parsed: Response = resp
            .json()
            .await
            .map_err(|e| PolishError::Api(e.to_string()))?;
        let text = parsed
            .content
            .into_iter()
            .filter(|b| b.block_type == "text")
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        sanity_check(raw, &text).map_err(PolishError::OutputRejected)?;
        Ok(text.trim().to_string())
    }

    fn name(&self) -> &'static str {
        "claude"
    }
}
