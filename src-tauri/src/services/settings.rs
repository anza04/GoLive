//! `SettingsService`: business logic for the one setting GoLive has so
//! far — the OpenAI API key (TASK-016). Thin by design (see
//! roadmap.md: this task is storage + settings UI only, no AI service
//! abstraction yet) — validates and trims the key before delegating to
//! `credentials::CredentialStore`, the same "service validates, trait
//! implementation just does the OS/storage operation" split every other
//! domain in this app uses.

use crate::credentials::CredentialStore;
use crate::errors::AppError;

/// Generous but not unbounded — real OpenAI keys are well under this
/// (typically under 200 characters); the limit exists only to reject
/// obviously-wrong pasted input (e.g. an entire pasted document) before
/// it ever reaches the credential store, not to constrain legitimate
/// keys.
const MAX_KEY_LENGTH: usize = 2000;

pub struct SettingsService {
    credentials: Box<dyn CredentialStore>,
}

impl SettingsService {
    pub fn new(credentials: Box<dyn CredentialStore>) -> Self {
        Self { credentials }
    }

    /// Trims and validates `raw_key`, then saves it. Overwrites any
    /// previously saved key — there is only ever one stored key, not a
    /// history of them (see docs/architecture.md §12: this is a secret,
    /// not versioned domain content).
    pub fn save_api_key(&self, raw_key: &str) -> Result<(), AppError> {
        let trimmed = raw_key.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation("API key is required.".to_string()));
        }
        if trimmed.chars().count() > MAX_KEY_LENGTH {
            return Err(AppError::Validation(format!(
                "API key must be {MAX_KEY_LENGTH} characters or fewer."
            )));
        }
        self.credentials.save_api_key(trimmed)
    }

    /// Whether a key is currently saved — the frontend's only way to
    /// know "is a key set" without ever being handed the key itself
    /// (see docs/architecture.md §12: never displayed back in
    /// plaintext once saved).
    pub fn has_api_key(&self) -> Result<bool, AppError> {
        Ok(self.credentials.get_api_key()?.is_some())
    }

    pub fn clear_api_key(&self) -> Result<(), AppError> {
        self.credentials.clear_api_key()
    }

    /// Reads the currently saved key (never exposed to the frontend
    /// directly) and hands it to `test_connection` — the one place the
    /// key's plaintext value briefly exists outside the credential
    /// store, for exactly as long as it takes to make one outbound call.
    pub fn test_connection(
        &self,
        test_connection: impl FnOnce(&str) -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        let key = self
            .credentials
            .get_api_key()?
            .ok_or_else(|| AppError::Validation("Save an API key first.".to_string()))?;
        test_connection(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// An in-memory `CredentialStore` fake — mirrors how other services'
    /// tests fake their repository trait (see `services::process::tests`
    /// etc.), never touching the real Windows Credential Manager. Kept
    /// local to this test module rather than exported from
    /// `credentials.rs`, since nothing outside this file's tests needs
    /// it (`credentials::tests` has its own real, cleanup-guarded
    /// coverage of the actual OS store).
    struct FakeCredentialStore {
        stored: Mutex<Option<String>>,
    }

    impl FakeCredentialStore {
        fn empty() -> Self {
            Self { stored: Mutex::new(None) }
        }
    }

    impl CredentialStore for FakeCredentialStore {
        fn save_api_key(&self, key: &str) -> Result<(), AppError> {
            *self.stored.lock().unwrap() = Some(key.to_string());
            Ok(())
        }
        fn get_api_key(&self) -> Result<Option<String>, AppError> {
            Ok(self.stored.lock().unwrap().clone())
        }
        fn clear_api_key(&self) -> Result<(), AppError> {
            *self.stored.lock().unwrap() = None;
            Ok(())
        }
    }

    fn service_with(stored: Option<&str>) -> SettingsService {
        let store = FakeCredentialStore::empty();
        if let Some(key) = stored {
            store.save_api_key(key).unwrap();
        }
        SettingsService::new(Box::new(store))
    }

    #[test]
    fn save_trims_and_stores_the_key() {
        let service = service_with(None);
        service.save_api_key("  sk-abc123  ").expect("save");
        assert!(service.has_api_key().expect("has_api_key"));
    }

    #[test]
    fn save_rejects_an_empty_key() {
        let service = service_with(None);
        let result = service.save_api_key("   ");
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn save_rejects_a_key_over_the_length_limit() {
        let service = service_with(None);
        let too_long = "a".repeat(MAX_KEY_LENGTH + 1);
        let result = service.save_api_key(&too_long);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn has_api_key_is_false_before_any_save() {
        let service = service_with(None);
        assert!(!service.has_api_key().expect("has_api_key"));
    }

    #[test]
    fn has_api_key_is_true_after_a_save() {
        let service = service_with(Some("sk-abc123"));
        assert!(service.has_api_key().expect("has_api_key"));
    }

    #[test]
    fn clear_removes_the_saved_key() {
        let service = service_with(Some("sk-abc123"));
        service.clear_api_key().expect("clear");
        assert!(!service.has_api_key().expect("has_api_key"));
    }

    #[test]
    fn saving_again_overwrites_the_previous_key() {
        let service = service_with(Some("sk-first"));
        service.save_api_key("sk-second").expect("second save");
        // Can't read the key back through this service directly (by
        // design — see docs/architecture.md §12), so overwrite is
        // observed through `test_connection`'s callback instead, the
        // one path that ever sees the plaintext value.
        let mut seen = None;
        service.test_connection(|key| {
            seen = Some(key.to_string());
            Ok(())
        }).expect("test_connection");
        assert_eq!(seen, Some("sk-second".to_string()));
    }

    #[test]
    fn test_connection_fails_fast_when_no_key_is_saved() {
        let service = service_with(None);
        let result = service.test_connection(|_| panic!("must not be called with no saved key"));
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_connection_passes_the_saved_key_to_the_callback() {
        let service = service_with(Some("sk-abc123"));
        let mut seen = None;
        let result = service.test_connection(|key| {
            seen = Some(key.to_string());
            Ok(())
        });
        assert!(result.is_ok());
        assert_eq!(seen, Some("sk-abc123".to_string()));
    }

    #[test]
    fn test_connection_propagates_the_callback_s_error() {
        let service = service_with(Some("sk-abc123"));
        let result = service.test_connection(|_| Err(AppError::Network("nope".to_string())));
        assert!(matches!(result, Err(AppError::Network(_))));
    }
}
