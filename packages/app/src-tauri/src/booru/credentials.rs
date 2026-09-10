//! The OS credential store behind a trait (`booru-sites` design D7), so the
//! rest of the app never calls `keyring` directly and no test ever touches a
//! real keychain.
//!
//! Entries are keyed on service `"BooruBox"` and account `"<host>/<username>"`
//! — the booru account the key belongs to, not the `booru_sites` row (design
//! D7): two libraries configuring the same account share one entry, and
//! changing a site's username or host address means a fresh account and a
//! fresh key, deliberately, rather than carrying the old one over.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::{AppError, Result};

/// `get`/`set`/`delete` for one `(host, username)` account. `get` answers
/// `Ok(None)` for "nothing stored yet" — a normal state for a freshly added
/// site — and reserves `Err` for the store itself refusing to answer: locked,
/// access denied, or no backend on this platform. Callers turn `Ok(None)`
/// into their own "no API key" message; they must not conflate the two, since
/// one is fixed by entering a key and the other is not fixable from here.
pub trait Credentials: Send + Sync {
    fn get(&self, host: &str, username: &str) -> Result<Option<String>>;
    fn set(&self, host: &str, username: &str, api_key: &str) -> Result<()>;
    /// Idempotent: deleting an account with nothing stored is not a failure.
    fn delete(&self, host: &str, username: &str) -> Result<()>;
}

fn account(host: &str, username: &str) -> String {
    format!("{host}/{username}")
}

/// One service name for every entry this app creates, so they all group under
/// one app in Keychain Access and Windows Credential Manager (design D7).
const SERVICE: &str = "BooruBox";

/// The real credential store. Builds a fresh `keyring::Entry` per call rather
/// than holding one: an entry is tied to one `(host, username)` pair, and this
/// type serves every site in the library, not one. `Entry::new` itself talks
/// to no backend — it only resolves which platform implementation to use — so
/// constructing this type has no side effect; only `get`/`set`/`delete` touch
/// the OS.
pub struct KeyringCredentials;

fn entry(host: &str, username: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, &account(host, username)).map_err(store_error)
}

fn store_error(error: keyring::Error) -> AppError {
    AppError::Credential {
        reason: error.to_string(),
    }
}

impl Credentials for KeyringCredentials {
    fn get(&self, host: &str, username: &str) -> Result<Option<String>> {
        match entry(host, username)?.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(store_error(error)),
        }
    }

    fn set(&self, host: &str, username: &str, api_key: &str) -> Result<()> {
        entry(host, username)?
            .set_password(api_key)
            .map_err(store_error)
    }

    fn delete(&self, host: &str, username: &str) -> Result<()> {
        match entry(host, username)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(store_error(error)),
        }
    }
}

/// An in-memory store for tests (design D7's "Testability follows from the
/// same place"), with an optional standing refusal so the "credential store
/// cannot be read or written" scenarios can be driven without a real
/// keychain: `InMemoryCredentials::refusing` answers every call with the
/// given reason, exactly as a locked keychain or a denied access would.
#[derive(Default)]
pub struct InMemoryCredentials {
    store: Mutex<HashMap<String, String>>,
    refuse: Option<String>,
}

impl InMemoryCredentials {
    pub fn refusing(reason: impl Into<String>) -> Self {
        InMemoryCredentials {
            store: Mutex::new(HashMap::new()),
            refuse: Some(reason.into()),
        }
    }

    fn refusal(&self) -> Result<()> {
        match &self.refuse {
            Some(reason) => Err(AppError::Credential {
                reason: reason.clone(),
            }),
            None => Ok(()),
        }
    }
}

impl Credentials for InMemoryCredentials {
    fn get(&self, host: &str, username: &str) -> Result<Option<String>> {
        self.refusal()?;
        Ok(self
            .store
            .lock()
            .unwrap()
            .get(&account(host, username))
            .cloned())
    }

    fn set(&self, host: &str, username: &str, api_key: &str) -> Result<()> {
        self.refusal()?;
        self.store
            .lock()
            .unwrap()
            .insert(account(host, username), api_key.to_string());
        Ok(())
    }

    fn delete(&self, host: &str, username: &str) -> Result<()> {
        self.refusal()?;
        self.store.lock().unwrap().remove(&account(host, username));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_credential_not_yet_set_reads_as_none_not_an_error() {
        let store = InMemoryCredentials::default();
        assert_eq!(store.get("danbooru.donmai.us", "alice").unwrap(), None);
    }

    #[test]
    fn set_then_get_round_trips_the_key() {
        let store = InMemoryCredentials::default();
        store
            .set("danbooru.donmai.us", "alice", "secret-key")
            .unwrap();
        assert_eq!(
            store.get("danbooru.donmai.us", "alice").unwrap(),
            Some("secret-key".to_string())
        );
    }

    #[test]
    fn the_same_host_with_a_different_username_is_a_different_account() {
        let store = InMemoryCredentials::default();
        store
            .set("danbooru.donmai.us", "alice", "alice-key")
            .unwrap();
        assert_eq!(store.get("danbooru.donmai.us", "bob").unwrap(), None);
    }

    #[test]
    fn delete_removes_the_key_and_is_idempotent() {
        let store = InMemoryCredentials::default();
        store
            .set("danbooru.donmai.us", "alice", "secret-key")
            .unwrap();

        store.delete("danbooru.donmai.us", "alice").unwrap();
        assert_eq!(store.get("danbooru.donmai.us", "alice").unwrap(), None);
        // Deleting an account with nothing stored must not be an error.
        store.delete("danbooru.donmai.us", "alice").unwrap();
    }

    #[test]
    fn a_refusing_store_fails_every_operation_with_its_reason() {
        let store = InMemoryCredentials::refusing("keychain is locked");

        let get_error = store.get("danbooru.donmai.us", "alice").unwrap_err();
        assert!(
            matches!(get_error, AppError::Credential { reason } if reason == "keychain is locked")
        );

        assert!(store.set("danbooru.donmai.us", "alice", "k").is_err());
        assert!(store.delete("danbooru.donmai.us", "alice").is_err());
    }
}
