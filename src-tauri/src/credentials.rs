//! Secure storage for the user's OpenAI API key (TASK-016) — the
//! Windows Credential Manager boundary docs/architecture.md §12 commits
//! to. The key is never written to SQLite, JSON, or any file under the
//! app-data directory; it lives entirely in the OS credential store,
//! survives an application restart because Windows (not GoLive) owns
//! it, and this module's public surface never returns it back out
//! except to whatever actually needs to send it to OpenAI
//! (`openai::test_api_key` today; the real AI service, TASK-017, later).
//!
//! Same "own one OS resource behind a small trait" shape as
//! `native::screenshot::ScreenshotEngine`/`native::recording::RecordingEngine`
//! — a trait so `services::settings::SettingsService` can be tested
//! against a fake, plus one real implementation
//! (`WindowsCredentialStore`) backed by the `keyring` crate's Windows
//! Credential Manager backend. Unlike those two native engines, this
//! one's own tests run for real (not `#[ignore]`d) — reading/writing/
//! deleting a credential is fast and side-effect-free once cleaned up,
//! unlike capturing actual video/audio — using a distinct test service
//! name so they can never collide with or clobber the real stored key.

use crate::errors::AppError;
use keyring::Entry;

/// Identifies this app's entry in the Windows Credential Manager —
/// distinct from `tauri.conf.json`'s `identifier` (`com.golive.app`,
/// that one is for OS app-identity/bundling, this one is just a
/// credential-store namespace, and deliberately kept as its own
/// constant rather than reusing that one so a future rename of either
/// doesn't silently orphan the other).
const SERVICE: &str = "GoLive";
const ACCOUNT: &str = "openai_api_key";

/// Abstraction over "the one secret GoLive stores" — currently just the
/// OpenAI API key, following `services::settings::SettingsService`'s own
/// scope (see roadmap.md TASK-016: storage + settings only, no AI
/// service yet).
pub trait CredentialStore: Send + Sync {
    fn save_api_key(&self, key: &str) -> Result<(), AppError>;
    /// `Ok(None)` means no key has ever been saved (or it was cleared)
    /// — a normal, expected state, not an error.
    fn get_api_key(&self) -> Result<Option<String>, AppError>;
    /// Clearing an already-absent key is success, not an error — same
    /// "already-deleted is fine" convention `media::MediaStorage`'s
    /// `delete_capture`/`delete_video` use.
    fn clear_api_key(&self) -> Result<(), AppError>;
}

pub struct WindowsCredentialStore;

impl WindowsCredentialStore {
    fn entry(&self) -> Result<Entry, AppError> {
        Entry::new(SERVICE, ACCOUNT).map_err(|err| {
            eprintln!("[golive] failed to open Windows Credential Manager entry: {err}");
            AppError::Credential("Could not access Windows Credential Manager.".to_string())
        })
    }
}

impl CredentialStore for WindowsCredentialStore {
    fn save_api_key(&self, key: &str) -> Result<(), AppError> {
        self.entry()?.set_password(key).map_err(|err| {
            eprintln!("[golive] failed to save API key to Windows Credential Manager: {err}");
            AppError::Credential("Could not save the API key.".to_string())
        })
    }

    fn get_api_key(&self) -> Result<Option<String>, AppError> {
        match self.entry()?.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => {
                eprintln!("[golive] failed to read API key from Windows Credential Manager: {err}");
                Err(AppError::Credential("Could not read the saved API key.".to_string()))
            }
        }
    }

    fn clear_api_key(&self) -> Result<(), AppError> {
        match self.entry()?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => {
                eprintln!("[golive] failed to clear API key from Windows Credential Manager: {err}");
                Err(AppError::Credential("Could not clear the saved API key.".to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `WindowsCredentialStore` pointed at a test-only service/account
    /// name (never `SERVICE`/`ACCOUNT` above) so these tests can run for
    /// real against the actual Windows Credential Manager without any
    /// risk of colliding with — or a failed test leaving behind — an
    /// entry under the real production name.
    struct TestCredentialStore {
        service: String,
        account: String,
    }

    impl TestCredentialStore {
        fn new(test_name: &str) -> Self {
            Self {
                service: format!("GoLive.Test.{test_name}"),
                account: "openai_api_key".to_string(),
            }
        }

        fn entry(&self) -> Entry {
            Entry::new(&self.service, &self.account).expect("test credential entry")
        }
    }

    impl CredentialStore for TestCredentialStore {
        fn save_api_key(&self, key: &str) -> Result<(), AppError> {
            self.entry().set_password(key).map_err(|err| AppError::Credential(err.to_string()))
        }
        fn get_api_key(&self) -> Result<Option<String>, AppError> {
            match self.entry().get_password() {
                Ok(password) => Ok(Some(password)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(err) => Err(AppError::Credential(err.to_string())),
            }
        }
        fn clear_api_key(&self) -> Result<(), AppError> {
            match self.entry().delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(err) => Err(AppError::Credential(err.to_string())),
            }
        }
    }

    /// Guarantees `clear_api_key` runs even if a test assertion panics
    /// partway through — otherwise a failing test would leave a real
    /// (if harmless, test-namespaced) entry behind in the actual
    /// Windows Credential Manager.
    struct CleanupGuard(TestCredentialStore);
    impl Drop for CleanupGuard {
        fn drop(&mut self) {
            let _ = self.0.clear_api_key();
        }
    }

    #[test]
    fn save_then_get_round_trips_the_same_key() {
        let guard = CleanupGuard(TestCredentialStore::new("round_trip"));
        guard.0.save_api_key("sk-test-round-trip-key").expect("save");
        assert_eq!(guard.0.get_api_key().expect("get"), Some("sk-test-round-trip-key".to_string()));
    }

    #[test]
    fn get_before_any_save_returns_none_not_an_error() {
        let guard = CleanupGuard(TestCredentialStore::new("unset"));
        assert_eq!(guard.0.get_api_key().expect("get on unset entry"), None);
    }

    #[test]
    fn clear_removes_a_saved_key() {
        let guard = CleanupGuard(TestCredentialStore::new("clear"));
        guard.0.save_api_key("sk-test-clear-key").expect("save");
        guard.0.clear_api_key().expect("clear");
        assert_eq!(guard.0.get_api_key().expect("get after clear"), None);
    }

    #[test]
    fn clearing_an_already_absent_key_is_not_an_error() {
        let guard = CleanupGuard(TestCredentialStore::new("clear_absent"));
        // No save happened — clearing must still succeed (same
        // already-deleted-is-fine convention as media::MediaStorage).
        guard.0.clear_api_key().expect("clear of an absent key should be Ok");
    }

    #[test]
    fn saving_again_overwrites_the_previous_key() {
        let guard = CleanupGuard(TestCredentialStore::new("overwrite"));
        guard.0.save_api_key("sk-test-first").expect("first save");
        guard.0.save_api_key("sk-test-second").expect("second save");
        assert_eq!(guard.0.get_api_key().expect("get"), Some("sk-test-second".to_string()));
    }
}
