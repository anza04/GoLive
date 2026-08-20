//! The one `AiService` implementation that exists today (TASK-016's
//! "test connection" call, and TASK-017's real generation call) —
//! everything OpenAI-specific lives in this one file, so nothing above
//! `ai::AiService` ever needs to know these are the calls being made.

use crate::ai::{AiService, ProcessDraft, ProcessDraftRequest, ProcessDraftStep};
use crate::errors::AppError;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const CHAT_COMPLETIONS_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";
const MODELS_ENDPOINT: &str = "https://api.openai.com/v1/models";

/// A model snapshot confirmed (see DECISIONS.md, TASK-017) to support
/// both vision input and strict JSON-schema structured output on the
/// Chat Completions API as of when this was written. Kept as one named
/// constant, not scattered across call sites, so bumping it later (a
/// near-certainty — OpenAI's model lineup moves fast) is a one-line
/// change.
const MODEL: &str = "gpt-5.6";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub struct OpenAiService;

impl AiService for OpenAiService {
    fn generate_process_draft(&self, api_key: &str, request: &ProcessDraftRequest) -> Result<ProcessDraft, AppError> {
        let body = build_request(request);
        let client = http_client()?;

        let response = client
            .post(CHAT_COMPLETIONS_ENDPOINT)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .map_err(|err| {
                eprintln!("[golive] OpenAI process-draft request failed to reach the API: {err}");
                AppError::Network("Could not reach OpenAI. Check your internet connection.".to_string())
            })?;

        let status = response.status();
        let text = response.text().map_err(|err| {
            eprintln!("[golive] failed to read OpenAI's response body: {err}");
            AppError::Network("OpenAI's response could not be read.".to_string())
        })?;

        if !status.is_success() {
            eprintln!("[golive] OpenAI process-draft request got status {status}: {text}");
            return Err(describe_openai_error(status, &text));
        }

        let parsed: ChatCompletionsResponse = serde_json::from_str(&text).map_err(|err| {
            eprintln!("[golive] could not parse OpenAI's response envelope: {err}\nbody: {text}");
            AppError::Ai("OpenAI's response was not in the expected format.".to_string())
        })?;

        let content = parsed
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_deref())
            .ok_or_else(|| {
                eprintln!("[golive] OpenAI response had no message content: {text}");
                AppError::Ai("OpenAI did not return a usable result.".to_string())
            })?;

        let raw: RawProcessDraft = serde_json::from_str(content).map_err(|err| {
            eprintln!("[golive] OpenAI's structured output did not match the schema: {err}\ncontent: {content}");
            AppError::Ai("OpenAI's result was not in the expected structure.".to_string())
        })?;

        Ok(finalize_draft(raw, request))
    }
}

fn http_client() -> Result<reqwest::blocking::Client, AppError> {
    reqwest::blocking::Client::builder().timeout(REQUEST_TIMEOUT).build().map_err(|err| {
        eprintln!("[golive] failed to build the OpenAI HTTP client: {err}");
        AppError::Network("Could not prepare a connection to OpenAI.".to_string())
    })
}

/// Calls OpenAI's models-list endpoint with `api_key` (TASK-016's "test
/// connection") — cheap (no completion tokens billed), and a 200
/// response only happens if the key was accepted, which is all that
/// needs proving. Separate from `generate_process_draft` above (a
/// different endpoint, no request body, no structured output) but kept
/// in this same file since both are OpenAI-specific mechanics no other
/// module should need to know about.
pub fn test_api_key(api_key: &str) -> Result<(), AppError> {
    let client = http_client()?;

    let response = client.get(MODELS_ENDPOINT).bearer_auth(api_key).send().map_err(|err| {
        eprintln!("[golive] OpenAI connection test failed to reach the API: {err}");
        AppError::Network("Could not reach OpenAI. Check your internet connection.".to_string())
    })?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let text = response.text().unwrap_or_default();
    eprintln!("[golive] OpenAI connection test got status {status}: {text}");
    Err(describe_openai_error(status, &text))
}

/// Turns an OpenAI HTTP failure into a safe, specific `AppError` — shared
/// by `test_api_key` and `OpenAiService::generate_process_draft`. OpenAI
/// error responses are `{"error": {"message", "type", "code", ...}}`
/// (see https://platform.openai.com/docs/guides/error-codes/api-errors,
/// confirmed against a real response during TASK-017's native
/// verification — see DECISIONS.md); `code`/`type` are parsed
/// best-effort (a malformed/unexpected body still falls through to the
/// generic per-status message below, never a parse failure of its own).
fn describe_openai_error(status: reqwest::StatusCode, body_text: &str) -> AppError {
    let code = serde_json::from_str::<Value>(body_text)
        .ok()
        .and_then(|value| value.get("error")?.get("code")?.as_str().map(str::to_string));

    match (status, code.as_deref()) {
        (reqwest::StatusCode::UNAUTHORIZED, _) => AppError::Validation("OpenAI rejected the saved API key.".to_string()),
        (_, Some("insufficient_quota")) => AppError::Validation(
            "This OpenAI account has no available quota. Check billing at platform.openai.com.".to_string(),
        ),
        (reqwest::StatusCode::TOO_MANY_REQUESTS, _) => {
            AppError::Network("OpenAI rate-limited this request. Try again in a moment.".to_string())
        }
        _ => AppError::Network(format!("OpenAI returned an unexpected response (status {status}).")),
    }
}

// --- Request building ---

#[derive(Serialize)]
struct ChatCompletionsRequest {
    model: &'static str,
    messages: Vec<ChatMessage>,
    response_format: ResponseFormat,
}

#[derive(Serialize)]
struct ChatMessage {
    role: &'static str,
    content: Vec<MessageContent>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum MessageContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlValue },
}

#[derive(Serialize)]
struct ImageUrlValue {
    url: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: &'static str,
    json_schema: JsonSchemaSpec,
}

#[derive(Serialize)]
struct JsonSchemaSpec {
    name: &'static str,
    strict: bool,
    schema: Value,
}

const SYSTEM_PROMPT: &str = "\
You help a business-process consultant turn a raw capture session into \
a clear, structured process write-up. The consultant used screen \
capture software to record screenshots, short notes, and screen \
recordings while actually performing a real business process on a \
computer, in the order listed. Produce a step-by-step description of \
the process as it was actually performed, using only what the provided \
material actually shows or says — never invent a step, a system name, \
or a detail that isn't evidenced by the captures. A 'recording' or \
'note' capture with a short title/description is still real evidence — \
describe the step it represents using that text, even without an \
image. Every step must list which capture number(s) (the numbers given \
before each capture below) it is based on; a step may reference more \
than one capture, or — only if it's a genuinely synthesized transition \
between other steps — none at all.";

fn build_request(request: &ProcessDraftRequest) -> ChatCompletionsRequest {
    let mut user_content = vec![MessageContent::Text {
        text: format!(
            "Process name: {}\nProcess description: {}\n\nCaptures, in the order they happened:",
            request.process_name,
            if request.process_description.is_empty() { "(none)" } else { &request.process_description }
        ),
    }];

    for (index, capture) in request.captures.iter().enumerate() {
        let number = index + 1;
        user_content.push(MessageContent::Text {
            text: format!(
                "Capture {number}: type={}, title=\"{}\", description=\"{}\"",
                capture.capture_type,
                capture.title,
                if capture.description.is_empty() { "(none)" } else { &capture.description }
            ),
        });
        if let Some(png_bytes) = &capture.screenshot_png {
            let encoded = base64::engine::general_purpose::STANDARD.encode(png_bytes);
            user_content.push(MessageContent::ImageUrl {
                image_url: ImageUrlValue { url: format!("data:image/png;base64,{encoded}") },
            });
        }
    }

    ChatCompletionsRequest {
        model: MODEL,
        messages: vec![
            ChatMessage { role: "system", content: vec![MessageContent::Text { text: SYSTEM_PROMPT.to_string() }] },
            ChatMessage { role: "user", content: user_content },
        ],
        response_format: ResponseFormat {
            format_type: "json_schema",
            json_schema: JsonSchemaSpec {
                name: "process_draft",
                strict: true,
                schema: process_draft_json_schema(),
            },
        },
    }
}

fn process_draft_json_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "summary": { "type": "string" },
            "steps": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "description": { "type": "string" },
                        "capture_indices": {
                            "type": "array",
                            "items": { "type": "integer" }
                        }
                    },
                    "required": ["title", "description", "capture_indices"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["summary", "steps"],
        "additionalProperties": false
    })
}

// --- Response parsing ---

#[derive(Deserialize)]
struct ChatCompletionsResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Deserialize)]
struct RawProcessDraft {
    summary: String,
    steps: Vec<RawProcessDraftStep>,
}

#[derive(Deserialize)]
struct RawProcessDraftStep {
    title: String,
    description: String,
    capture_indices: Vec<i64>,
}

/// Translates the model's 1-based `capture_indices` (see `SYSTEM_PROMPT`
/// and `build_request` — the numbers the prompt itself assigned) back
/// into real Capture ids, dropping anything out of range rather than
/// trusting the model never hallucinates one — `strict: true` guarantees
/// the *shape* (an array of integers) but says nothing about whether
/// each integer is actually one of the numbers this request handed out.
fn finalize_draft(raw: RawProcessDraft, request: &ProcessDraftRequest) -> ProcessDraft {
    let steps = raw
        .steps
        .into_iter()
        .map(|step| {
            let mut dropped = 0;
            let capture_ids = step
                .capture_indices
                .into_iter()
                .filter_map(|index| {
                    let position = usize::try_from(index).ok().and_then(|i| i.checked_sub(1))?;
                    match request.captures.get(position) {
                        Some(capture) => Some(capture.id.clone()),
                        None => {
                            dropped += 1;
                            None
                        }
                    }
                })
                .collect();
            if dropped > 0 {
                eprintln!(
                    "[golive] dropped {dropped} out-of-range capture reference(s) from a generated step"
                );
            }
            ProcessDraftStep { title: step.title, description: step.description, capture_ids }
        })
        .collect();

    ProcessDraft { summary: raw.summary, steps }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::CaptureForAi;

    fn capture(id: &str) -> CaptureForAi {
        CaptureForAi {
            id: id.to_string(),
            capture_type: "screenshot".to_string(),
            title: "t".to_string(),
            description: "d".to_string(),
            screenshot_png: None,
        }
    }

    #[test]
    fn describe_openai_error_maps_401_to_a_key_rejected_validation_error() {
        let err = describe_openai_error(reqwest::StatusCode::UNAUTHORIZED, "{}");
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn describe_openai_error_maps_insufficient_quota_to_a_specific_validation_error() {
        // The exact real response body OpenAI returns for this case
        // (confirmed during TASK-017's native verification — see
        // DECISIONS.md), not a guessed shape.
        let body = r#"{"error":{"message":"You exceeded your current quota.","type":"insufficient_quota","param":null,"code":"insufficient_quota"}}"#;
        let err = describe_openai_error(reqwest::StatusCode::TOO_MANY_REQUESTS, body);
        match err {
            AppError::Validation(message) => assert!(message.contains("quota")),
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[test]
    fn describe_openai_error_maps_plain_rate_limiting_to_a_network_error() {
        let body = r#"{"error":{"message":"Rate limit reached.","type":"requests","code":"rate_limit_exceeded"}}"#;
        let err = describe_openai_error(reqwest::StatusCode::TOO_MANY_REQUESTS, body);
        assert!(matches!(err, AppError::Network(_)));
    }

    #[test]
    fn describe_openai_error_falls_back_to_a_generic_message_for_an_unparsable_body() {
        let err = describe_openai_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "not json");
        assert!(matches!(err, AppError::Network(_)));
    }

    fn request_with(captures: Vec<CaptureForAi>) -> ProcessDraftRequest {
        ProcessDraftRequest { process_name: "P".to_string(), process_description: "".to_string(), captures }
    }

    #[test]
    fn finalize_draft_maps_one_based_indices_to_capture_ids() {
        let request = request_with(vec![capture("cap-1"), capture("cap-2")]);
        let raw = RawProcessDraft {
            summary: "s".to_string(),
            steps: vec![RawProcessDraftStep {
                title: "t".to_string(),
                description: "d".to_string(),
                capture_indices: vec![1, 2],
            }],
        };

        let draft = finalize_draft(raw, &request);

        assert_eq!(draft.steps[0].capture_ids, vec!["cap-1".to_string(), "cap-2".to_string()]);
    }

    #[test]
    fn finalize_draft_drops_out_of_range_indices() {
        let request = request_with(vec![capture("cap-1")]);
        let raw = RawProcessDraft {
            summary: "s".to_string(),
            steps: vec![RawProcessDraftStep {
                title: "t".to_string(),
                description: "d".to_string(),
                capture_indices: vec![0, 1, 2, 99],
            }],
        };

        let draft = finalize_draft(raw, &request);

        assert_eq!(draft.steps[0].capture_ids, vec!["cap-1".to_string()]);
    }

    #[test]
    fn finalize_draft_handles_a_step_with_no_capture_references() {
        let request = request_with(vec![capture("cap-1")]);
        let raw = RawProcessDraft {
            summary: "s".to_string(),
            steps: vec![RawProcessDraftStep { title: "t".to_string(), description: "d".to_string(), capture_indices: vec![] }],
        };

        let draft = finalize_draft(raw, &request);

        assert!(draft.steps[0].capture_ids.is_empty());
    }

    #[test]
    fn process_draft_json_schema_is_strict_mode_compatible() {
        // Every property referenced anywhere must be listed in that same
        // object's "required" array, and every object must set
        // "additionalProperties": false — OpenAI's strict mode rejects
        // the whole request otherwise. A cheap structural check against
        // regressing that by hand-editing the schema later.
        let schema = process_draft_json_schema();
        assert_eq!(schema["additionalProperties"], false);
        let step_schema = &schema["properties"]["steps"]["items"];
        assert_eq!(step_schema["additionalProperties"], false);
        let required: Vec<&str> = step_schema["required"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        let properties: Vec<&str> = step_schema["properties"].as_object().unwrap().keys().map(String::as_str).collect();
        for prop in &properties {
            assert!(required.contains(prop), "property {prop} is not marked required");
        }
    }
}
