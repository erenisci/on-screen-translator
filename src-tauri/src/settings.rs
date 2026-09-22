//! Settings: one JSON file under `%APPDATA%`, plus built-in defaults.
//!
//! API keys are conspicuously absent — they live in Windows Credential Manager
//! (invariant 7). Anyone finding a key in this file has found a bug.
//!
//! The file is user-editable, so it is treated as untrusted input: a corrupt or
//! partially-invalid file must yield a working app and a clear message, never a
//! crash and never a silent reset (FR-61, docs/operations/configuration.md).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_HOTKEY: &str = "Ctrl+Shift+T";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,

    pub target_language: String,
    /// `None` = auto-detect (FR-33).
    pub source_language: Option<String>,

    pub hotkey: String,
    pub result_display_mode: String,

    pub provider: String,
    pub llm_endpoint: Option<String>,
    pub llm_model: Option<String>,

    pub start_with_windows: bool,
    pub theme: String,

    pub log_level: String,

    /// Fields this version doesn't know about.
    ///
    /// Preserved verbatim on write so that rolling back to an older build does
    /// not strip a newer build's settings — the one behaviour that makes a
    /// rollback survivable (docs/operations/rollback.md).
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            // NFR-U4: the app should be right for the user before they open
            // Settings, so this is the OS display language, not "en".
            target_language: system_display_language(),
            source_language: None,
            hotkey: DEFAULT_HOTKEY.to_string(),
            result_display_mode: "both".to_string(),
            provider: "libretranslate".to_string(),
            llm_endpoint: None,
            llm_model: None,
            // Opt-in. An app that adds itself to startup uninvited is rude.
            start_with_windows: false,
            theme: "system".to_string(),
            log_level: "info".to_string(),
            extra: Map::new(),
        }
    }
}

/// The user's Windows display language as a BCP-47 tag, e.g. "tr-TR" -> "tr".
fn system_display_language() -> String {
    #[cfg(windows)]
    {
        use windows::Win32::Globalization::GetUserDefaultLocaleName;
        let mut buffer = [0u16; 85]; // LOCALE_NAME_MAX_LENGTH
        let len = unsafe { GetUserDefaultLocaleName(&mut buffer) };
        if len > 1 {
            let name = String::from_utf16_lossy(&buffer[..(len as usize - 1)]);
            // Keep the primary subtag: "tr-TR" -> "tr". Providers disagree on
            // regional tags, and the base language is what users mean.
            if let Some(primary) = name.split('-').next() {
                if !primary.is_empty() {
                    return primary.to_ascii_lowercase();
                }
            }
        }
    }
    "en".to_string()
}

pub fn settings_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("on-screen-translator")
}

pub fn settings_path() -> PathBuf {
    settings_dir().join("settings.json")
}

/// How loading went. The caller decides what to tell the user; this type just
/// refuses to hide that a fallback happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadOutcome {
    Loaded,
    CreatedDefaults,
    /// The file was unreadable and was preserved as `settings.corrupt.json`.
    RecoveredFromCorrupt(String),
}

pub struct Loaded {
    pub settings: Settings,
    pub outcome: LoadOutcome,
}

pub fn load() -> Loaded {
    let path = settings_path();

    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => {
            return Loaded {
                settings: Settings::default(),
                outcome: LoadOutcome::CreatedDefaults,
            }
        }
    };

    match parse(&raw) {
        Ok(settings) => Loaded {
            settings,
            outcome: LoadOutcome::Loaded,
        },
        Err(reason) => {
            // Preserve, never delete: the user may have hand-edited it and a
            // typo should not cost them their configuration.
            let backup = settings_dir().join("settings.corrupt.json");
            let _ = std::fs::rename(&path, &backup);
            Loaded {
                settings: Settings::default(),
                outcome: LoadOutcome::RecoveredFromCorrupt(reason),
            }
        }
    }
}

/// Parse and normalize. Invalid individual values fall back per field rather
/// than discarding the whole file.
pub fn parse(raw: &str) -> Result<Settings, String> {
    let mut settings: Settings = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    settings.normalize();
    Ok(settings)
}

impl Settings {
    /// Repair individual invalid values, leaving everything else intact.
    pub fn normalize(&mut self) {
        let defaults = Settings::default();

        if self.target_language.trim().is_empty() {
            self.target_language = defaults.target_language.clone();
        }
        if self
            .source_language
            .as_deref()
            .is_some_and(|s| s.trim().is_empty())
        {
            self.source_language = None;
        }
        if self.hotkey.trim().is_empty() {
            self.hotkey = defaults.hotkey.clone();
        }
        if !matches!(
            self.result_display_mode.as_str(),
            "overlay" | "panel" | "both"
        ) {
            self.result_display_mode = defaults.result_display_mode.clone();
        }
        if !matches!(
            self.provider.as_str(),
            "libretranslate" | "deepl" | "google" | "llm"
        ) {
            self.provider = defaults.provider.clone();
        }
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            self.theme = defaults.theme.clone();
        }
        // A custom LLM endpoint is user-trusted but must still be TLS
        // (docs/operations/security.md).
        if self
            .llm_endpoint
            .as_deref()
            .is_some_and(|url| !url.starts_with("https://"))
        {
            self.llm_endpoint = None;
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = settings_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(settings_path(), json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_usable_with_no_configuration() {
        // NFR-U1: install -> hotkey -> translation, no key and no account.
        let settings = Settings::default();
        assert_eq!(settings.provider, "libretranslate");
        assert_eq!(settings.hotkey, DEFAULT_HOTKEY);
        assert!(!settings.start_with_windows);
        assert!(!settings.target_language.is_empty());
        assert!(settings.source_language.is_none());
    }

    #[test]
    fn round_trips_through_json_in_camel_case() {
        let settings = Settings::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"targetLanguage\""));
        assert!(json.contains("\"startWithWindows\""));
        assert!(json.contains("\"schemaVersion\""));
        assert_eq!(parse(&json).unwrap(), settings);
    }

    #[test]
    fn never_serializes_anything_key_shaped() {
        let json = serde_json::to_string(&Settings::default())
            .unwrap()
            .to_lowercase();
        for forbidden in ["apikey", "api_key", "secret", "token", "password"] {
            assert!(
                !json.contains(forbidden),
                "settings JSON mentions {forbidden}"
            );
        }
    }

    #[test]
    fn unknown_fields_survive_a_round_trip() {
        // A newer build wrote `futureSetting`; this build must not strip it.
        let raw = r#"{
            "schemaVersion": 1,
            "targetLanguage": "tr",
            "sourceLanguage": null,
            "hotkey": "Ctrl+Shift+T",
            "resultDisplayMode": "both",
            "provider": "deepl",
            "llmEndpoint": null,
            "llmModel": null,
            "startWithWindows": false,
            "theme": "system",
            "logLevel": "info",
            "futureSetting": {"nested": true}
        }"#;
        let settings = parse(raw).unwrap();
        assert!(settings.extra.contains_key("futureSetting"));

        let written = serde_json::to_string(&settings).unwrap();
        assert!(written.contains("futureSetting"));
        assert!(written.contains("nested"));
    }

    #[test]
    fn invalid_enum_values_fall_back_per_field_not_wholesale() {
        let raw = r#"{
            "schemaVersion": 1,
            "targetLanguage": "tr",
            "sourceLanguage": null,
            "hotkey": "Ctrl+Alt+Q",
            "resultDisplayMode": "hologram",
            "provider": "not-a-provider",
            "llmEndpoint": null,
            "llmModel": null,
            "startWithWindows": true,
            "theme": "neon",
            "logLevel": "info"
        }"#;
        let settings = parse(raw).unwrap();

        // Bad values repaired...
        assert_eq!(settings.result_display_mode, "both");
        assert_eq!(settings.provider, "libretranslate");
        assert_eq!(settings.theme, "system");
        // ...good ones kept. Discarding these would be the silent reset that
        // FR-61 forbids.
        assert_eq!(settings.hotkey, "Ctrl+Alt+Q");
        assert_eq!(settings.target_language, "tr");
        assert!(settings.start_with_windows);
    }

    #[test]
    fn a_plaintext_llm_endpoint_is_rejected() {
        let mut settings = Settings {
            llm_endpoint: Some("http://insecure.invalid/v1".into()),
            ..Default::default()
        };
        settings.normalize();
        assert_eq!(settings.llm_endpoint, None);

        let mut settings = Settings {
            llm_endpoint: Some("https://secure.invalid/v1".into()),
            ..Default::default()
        };
        settings.normalize();
        assert_eq!(
            settings.llm_endpoint.as_deref(),
            Some("https://secure.invalid/v1")
        );
    }

    #[test]
    fn an_empty_source_language_means_auto_detect() {
        let mut settings = Settings {
            source_language: Some("   ".into()),
            ..Default::default()
        };
        settings.normalize();
        assert_eq!(settings.source_language, None);
    }

    #[test]
    fn a_blank_required_field_falls_back_to_its_default() {
        let mut settings = Settings {
            hotkey: "  ".into(),
            target_language: String::new(),
            ..Default::default()
        };
        settings.normalize();
        assert_eq!(settings.hotkey, DEFAULT_HOTKEY);
        assert!(!settings.target_language.is_empty());
    }

    #[test]
    fn malformed_json_is_an_error_not_a_panic() {
        assert!(parse("{ this is not json").is_err());
        assert!(parse("").is_err());
        assert!(parse("[]").is_err());
    }

    #[test]
    fn system_display_language_is_a_plausible_primary_subtag() {
        let lang = system_display_language();
        assert!(!lang.is_empty());
        assert!(!lang.contains('-'), "expected a primary subtag, got {lang}");
        assert_eq!(lang, lang.to_ascii_lowercase());
    }
}
