use super::{build_full_system, sanity_check, Correction, PolishError, Polisher};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const ENDPOINT: &str = "https://api.groq.com/openai/v1/chat/completions";
const MODEL: &str = "llama-3.3-70b-versatile";

pub struct GroqLlamaPolisher {
    api_key: String,
}

impl GroqLlamaPolisher {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: String,
}

#[derive(Deserialize)]
struct Response {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[async_trait]
impl Polisher for GroqLlamaPolisher {
    async fn polish(&self, raw: &str, recent: &[Correction]) -> Result<String, PolishError> {
        let system = build_full_system(recent);
        let req = Request {
            model: MODEL,
            messages: vec![
                Message {
                    role: "system",
                    content: system,
                },
                Message {
                    role: "user",
                    content: format!("Polish this transcript:\n\n{}", raw),
                },
            ],
            temperature: 0.2,
            max_tokens: (raw.chars().count() as u32 * 2 + 200).clamp(64, 2048),
        };

        let resp = reqwest::Client::new()
            .post(ENDPOINT)
            .bearer_auth(&self.api_key)
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
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        sanity_check(raw, &text).map_err(PolishError::OutputRejected)?;
        Ok(text.trim().to_string())
    }

    fn name(&self) -> &'static str {
        "groq_llama"
    }
}
