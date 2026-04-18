use crate::nice_display::NiceDisplay;
use crate::open_ai_key::OpenAiKey;
use reqwest::header::CONTENT_TYPE;

pub struct EmbeddingRequest {
    content: String,
}

pub enum EmbeddingError {
    Request(String),
    Response(String),
    ResponseJsonDecode(String),
}

impl NiceDisplay for EmbeddingError {
    fn message(&self) -> String {
        match self {
            EmbeddingError::Request(err) => {
                format!("I had trouble making a request to open ai\n{}", err)
            }
            EmbeddingError::Response(err) => {
                format!("I had trouble with the response from open ai\n{}", err)
            }
            EmbeddingError::ResponseJsonDecode(err) => {
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

        let response = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Content-Type", "application/json")
            .header("Authorization", open_ai_key.to_header())
            .json(&json_body)
            .send()
            .await
            .map_err(|err| EmbeddingError::Request(err.to_string()))?;

        let status = response.status();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string());

        let res = response
            .text()
            .await
            .map_err(|err| EmbeddingError::Response(err.to_string()))?;

        if !status.is_success() {
            let maybe_res_json: Result<serde_json::Value, serde_json::Error> =
                serde_json::from_str(&res);

            return match maybe_res_json {
                Ok(res_json) => Err(EmbeddingError::Response(format!(
                    "open ai returned HTTP {}: {}",
                    status, res_json
                ))),
                Err(err) => Err(EmbeddingError::Response(format!(
                    "open ai returned HTTP {} with a non-JSON body: {}",
                    status,
                    describe_json_decode_failure(
                        content_type.as_deref(),
                        res.as_str(),
                        err.to_string().as_str()
                    )
                ))),
            };
        }

        let res_json: serde_json::Value = serde_json::from_str(&res).map_err(|err| {
            EmbeddingError::ResponseJsonDecode(describe_json_decode_failure(
                content_type.as_deref(),
                res.as_str(),
                err.to_string().as_str(),
            ))
        })?;

        let vector = res_json
            .get("data")
            .ok_or_else(|| EmbeddingError::ResponseJsonDecode("Missing data field".to_string()))?
            .get(0)
            .ok_or_else(|| {
                EmbeddingError::ResponseJsonDecode("Missing first data element".to_string())
            })?
            .get("embedding")
            .ok_or_else(|| {
                EmbeddingError::ResponseJsonDecode("Missing embedding field".to_string())
            })?
            .as_array()
            .ok_or_else(|| EmbeddingError::ResponseJsonDecode("Embedding not array".to_string()))?
            .iter()
            .map(|v| {
                v.as_f64()
                    .ok_or_else(|| {
                        EmbeddingError::ResponseJsonDecode(
                            "Embedding value not a number".to_string(),
                        )
                    })
                    .map(|f| f as f32)
            })
            .collect::<Result<Vec<f32>, EmbeddingError>>()?;

        Ok(vector)
    }
}

fn describe_json_decode_failure(
    content_type: Option<&str>,
    response_body: &str,
    serde_error: &str,
) -> String {
    let content_type_text = match content_type {
        Some(value) => format!("content-type `{}`", value),
        None => "missing content-type".to_string(),
    };
    let body_preview = preview_response_body(response_body);

    format!(
        "{}; {}; body preview: {}",
        serde_error, content_type_text, body_preview
    )
}

fn preview_response_body(response_body: &str) -> String {
    let trimmed = response_body.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }

    let mut preview = String::new();
    let mut char_count = 0usize;
    for ch in trimmed.chars() {
        if char_count == 200 {
            preview.push_str("...");
            break;
        }

        match ch {
            '\n' | '\r' | '\t' => preview.push(' '),
            _ => preview.push(ch),
        }
        char_count += 1;
    }

    preview
}
