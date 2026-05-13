use super::{Correction, PolishError, Polisher};
use async_trait::async_trait;

pub struct NoOpPolisher;

#[async_trait]
impl Polisher for NoOpPolisher {
    async fn polish(&self, raw: &str, _recent: &[Correction]) -> Result<String, PolishError> {
        Ok(raw.to_string())
    }

    fn name(&self) -> &'static str {
        "none"
    }
}
