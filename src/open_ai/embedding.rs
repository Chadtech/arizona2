use crate::nice_display::NiceDisplay;
use crate::open_ai_key::OpenAiKey;

pub struct EmbeddingRequest {
    content: String,
}

pub enum EmbeddingError {
    RequestError(String),
    ResponseError(String),
    ResponseJsonDecodeError(String),
}

impl NiceDisplay for EmbeddingError {
    fn message(&self) -> String {
        match self {
            EmbeddingError::RequestError(err) => {
                format!("I had trouble making a request to open ai\n{}", err)
            }
            EmbeddingError::ResponseError(err) => {
                format!("I had trouble with the response from open ai\n{}", err)
            }
            EmbeddingError::ResponseJsonDecodeError(err) => {
                format!("I had trouble decoding the response from open ai\n{}", err)
            }
        }
    }
}

impl EmbeddingRequest {
    pub fn new(content: String) -> Self {
        Self { content }
    }

    pub async fn create(
        &self,
        open_ai_key: OpenAiKey,
        client: reqwest::Client,
    ) -> Result<Vec<f32>, EmbeddingError> {
        let json_body = serde_json::json!({
            "input": self.content,
            "model": "text-embedding-3-small"
        });

        let res = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Content-Type", "application/json")
            .header("Authorization", open_ai_key.to_header())
            .json(&json_body)
            .send()
            .await
            .map_err(|err| EmbeddingError::RequestError(err.to_string()))?
            .text()
            .await
            .map_err(|err| EmbeddingError::ResponseError(err.to_string()))?;

        let res_json: serde_json::Value = serde_json::from_str(&res)
            .map_err(|err| EmbeddingError::ResponseJsonDecodeError(err.to_string()))?;

        let vector = res_json
            .get("data")
            .ok_or_else(|| {
                EmbeddingError::ResponseJsonDecodeError("Missing data field".to_string())
            })?
            .get(0)
            .ok_or_else(|| {
                EmbeddingError::ResponseJsonDecodeError("Missing first data element".to_string())
            })?
            .get("embedding")
            .ok_or_else(|| {
                EmbeddingError::ResponseJsonDecodeError("Missing embedding field".to_string())
            })?
            .as_array()
            .ok_or_else(|| {
                EmbeddingError::ResponseJsonDecodeError("Embedding not array".to_string())
            })?
            .iter()
            .map(|v| {
                v.as_f64()
                    .ok_or_else(|| {
                        EmbeddingError::ResponseJsonDecodeError(
                            "Embedding value not a number".to_string(),
                        )
                    })
                    .map(|f| f as f32)
            })
            .collect::<Result<Vec<f32>, EmbeddingError>>()?;

        Ok(vector)
    }
}
