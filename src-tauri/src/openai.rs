//! A single, standalone connection check against OpenAI's API (TASK-016
//! Settings: "test connection"). Deliberately **not** the AI service
//! abstraction docs/architecture.md §8 describes — that's TASK-017's
//! job, behind a real `AiService` trait with a swappable provider
//! implementation. This is scoped to exactly what TASK-016 needs to
//! prove a saved key actually works, nothing more; TASK-017 is free to
//! reuse or replace this call once the real abstraction exists.

use crate::errors::AppError;
use std::time::Duration;

const MODELS_ENDPOINT: &str = "https://api.openai.com/v1/models";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Calls OpenAI's models-list endpoint with `api_key` — cheap (no
/// completion tokens billed), and a 200 response only happens if the
/// key was accepted, which is all "test connection" needs to prove.
/// Returns `Ok(())` on success; every failure mode is mapped to a
/// specific, author-written `AppError::Network`/`AppError::Validation`
/// message — never the raw `reqwest` error, which could include
/// resolved IPs or other connection detail with no business being
/// shown to a user.
pub fn test_api_key(api_key: &str) -> Result<(), AppError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|err| {
            eprintln!("[golive] failed to build the OpenAI test-connection HTTP client: {err}");
            AppError::Network("Could not prepare a connection to OpenAI.".to_string())
        })?;

    let response = client.get(MODELS_ENDPOINT).bearer_auth(api_key).send().map_err(|err| {
        eprintln!("[golive] OpenAI connection test failed to reach the API: {err}");
        AppError::Network("Could not reach OpenAI. Check your internet connection.".to_string())
    })?;

    let status = response.status();
    if status.is_success() {
        Ok(())
    } else if status == reqwest::StatusCode::UNAUTHORIZED {
        Err(AppError::Validation("OpenAI rejected that API key.".to_string()))
    } else {
        eprintln!("[golive] OpenAI connection test got an unexpected status: {status}");
        Err(AppError::Network(format!(
            "OpenAI returned an unexpected response (status {status})."
        )))
    }
}
