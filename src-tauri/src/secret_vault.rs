//! Security upgrade: post-processing API keys are kept in the operating
//! system's credential store (Windows Credential Manager, macOS Keychain)
//! instead of in plain text inside `settings_store.json`.
//!
//! The settings code calls [`to_persisted`] before writing the store, which
//! moves every key into the vault and leaves an empty string on disk, and
//! [`hydrate`] after reading it, which fills the empty strings back in from
//! the vault. A key the vault cannot take stays in the settings file, as
//! before, so nothing is ever lost. Platforms without a supported vault
//! (Linux here) keep the previous plain-text behaviour.

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

/// Credential "service" (Windows: target name prefix) for every key.
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
const SERVICE: &str = "com.pais.handy.post-process";

/// What the vault is known to hold per provider: `Some(key)`, or `None` when
/// it is known to hold nothing. Absent = not looked up yet. Settings are read
/// constantly, so this keeps vault calls to one per provider per run.
static CACHE: Lazy<Mutex<HashMap<String, Option<String>>>> = Lazy::new(Default::default);

/// True when this platform stores keys in an OS credential vault.
pub fn available() -> bool {
    cfg!(any(windows, target_os = "macos"))
}

#[cfg(any(windows, target_os = "macos"))]
mod backend {
    use super::SERVICE;

    pub fn get(provider: &str) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new(SERVICE, provider).map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn set(provider: &str, secret: &str) -> Result<(), String> {
        keyring::Entry::new(SERVICE, provider)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|e| e.to_string())
    }

    pub fn delete(provider: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(SERVICE, provider).map_err(|e| e.to_string())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod backend {
    pub fn get(_provider: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    pub fn set(_provider: &str, _secret: &str) -> Result<(), String> {
        Err("no credential vault on this platform".into())
    }
    pub fn delete(_provider: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Returns the key map to write to disk: every key the vault accepted is
/// replaced by an empty string; keys it could not take stay as they are.
pub fn to_persisted(keys: &HashMap<String, String>) -> HashMap<String, String> {
    if !available() {
        return keys.clone();
    }
    let mut cache = CACHE.lock().unwrap_or_else(|p| p.into_inner());
    let mut on_disk = HashMap::with_capacity(keys.len());
    for (provider, key) in keys {
        let known = cache.get(provider).cloned();
        if key.is_empty() {
            // Cleared (or never set): make sure no stale copy stays in the vault.
            if known != Some(None) {
                match backend::delete(provider) {
                    Ok(()) => {
                        cache.insert(provider.clone(), None);
                    }
                    Err(e) => log::warn!(
                        "Could not remove the {provider} API key from the credential store: {e}"
                    ),
                }
            }
            on_disk.insert(provider.clone(), String::new());
        } else if known.as_ref().and_then(|k| k.as_deref()) == Some(key.as_str()) {
            on_disk.insert(provider.clone(), String::new());
        } else {
            match backend::set(provider, key) {
                Ok(()) => {
                    cache.insert(provider.clone(), Some(key.clone()));
                    on_disk.insert(provider.clone(), String::new());
                }
                Err(e) => {
                    log::warn!(
                        "Could not store the {provider} API key in the credential store ({e}); \
                         keeping it in the settings file"
                    );
                    on_disk.insert(provider.clone(), key.clone());
                }
            }
        }
    }
    on_disk
}

/// Fills empty keys from the vault. Returns true when a plain-text key was
/// found on disk, so the caller re-saves and the key moves into the vault.
pub fn hydrate(keys: &mut HashMap<String, String>) -> bool {
    if !available() {
        return false;
    }
    let mut cache = CACHE.lock().unwrap_or_else(|p| p.into_inner());
    let mut found_plaintext = false;
    for (provider, key) in keys.iter_mut() {
        if !key.is_empty() {
            found_plaintext = true;
            continue;
        }
        let known = match cache.get(provider) {
            Some(known) => known.clone(),
            None => match backend::get(provider) {
                Ok(stored) => {
                    cache.insert(provider.clone(), stored.clone());
                    stored
                }
                Err(e) => {
                    log::warn!(
                        "Could not read the {provider} API key from the credential store: {e}"
                    );
                    None
                }
            },
        };
        if let Some(stored) = known {
            *key = stored;
        }
    }
    found_plaintext
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_vault_keys_stay_on_disk_unchanged() {
        if available() {
            return; // exercised by the platform build instead
        }
        let keys = HashMap::from([
            ("openai".to_string(), "sk-test".to_string()),
            ("groq".to_string(), String::new()),
        ]);
        assert_eq!(to_persisted(&keys), keys);
        let mut loaded = keys.clone();
        assert!(!hydrate(&mut loaded));
        assert_eq!(loaded, keys);
    }
}
