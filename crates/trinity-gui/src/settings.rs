//! Persisted GUI settings (`~/.config/razer-trinity-mapper/gui.toml`)
//! and system locale detection.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Public project repository — the single place to update.
pub const PROJECT_URL: &str = "https://github.com/4lador/razer-trinity-mapper";

/// Embedded locales, in display order.
pub const LOCALES: &[&str] = &["fr", "en", "de", "es", "it", "pt-BR"];

/// Native display name (never translated).
pub fn native_name(locale: &str) -> &str {
    match locale {
        "fr" => "Français", // lang-ok: native name
        "en" => "English",
        "de" => "Deutsch",
        "es" => "Español",
        "it" => "Italiano",
        "pt-BR" => "Português (Brasil)", // lang-ok: native name
        other => other,
    }
}

/// System locale derived from `LC_ALL`/`LANG` (`fr_FR.UTF-8` to `fr`,
/// `pt_BR.utf8` to `pt-BR`), falling back to `en` if unknown.
pub fn system_locale() -> &'static str {
    let raw = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    let tag = raw.split('.').next().unwrap_or_default().replace('_', "-");
    if tag.eq_ignore_ascii_case("pt-BR") {
        return "pt-BR";
    }
    let primary = tag.split('-').next().unwrap_or_default().to_lowercase();
    if LOCALES.contains(&primary.as_str()) {
        // The slice contains known 'static strs.
        return LOCALES.iter().find(|&&l| l == primary).unwrap();
    }
    "en"
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuiSettings {
    #[serde(default)]
    pub locale: Option<String>,
}

impl GuiSettings {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("razer-trinity-mapper")
            .join("gui.toml")
    }

    pub fn load() -> Self {
        let Ok(content) = fs::read_to_string(Self::path()) else {
            return Self::default();
        };
        toml::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        let Some(parent) = path.parent() else { return };
        if fs::create_dir_all(parent).is_err() {
            return;
        }
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }

    /// Effective locale: user choice, else system.
    pub fn effective_locale(&self) -> String {
        self.locale
            .clone()
            .filter(|locale| LOCALES.contains(&locale.as_str()))
            .unwrap_or_else(|| system_locale().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_are_declared_once() {
        let mut sorted = LOCALES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), LOCALES.len());
    }

    #[test]
    fn every_locale_file_exists() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/locales");
        for locale in LOCALES {
            let path = format!("{dir}/{locale}.toml");
            assert!(std::path::Path::new(&path).exists(), "missing file: {path}");
        }
    }

    /// Collaborative guard: all files must expose exactly the same key
    /// set (with the same placeholders).
    #[test]
    fn every_locale_exposes_the_same_keys() {
        fn flatten(value: &toml::Value, prefix: &str, out: &mut Vec<String>) {
            match value {
                toml::Value::Table(table) => {
                    for (key, value) in table {
                        flatten(value, &format!("{prefix}{key}."), out);
                    }
                }
                other => {
                    let _ = other;
                    out.push(prefix.trim_end_matches('.').to_string());
                }
            }
        }
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/locales");
        let reference: Vec<String> = {
            let raw = std::fs::read_to_string(format!("{dir}/en.toml")).unwrap();
            let value: toml::Value = toml::from_str(&raw).unwrap();
            let mut keys = Vec::new();
            flatten(&value, "", &mut keys);
            keys.sort();
            keys
        };
        assert!(!reference.is_empty());
        for locale in LOCALES {
            let raw = std::fs::read_to_string(format!("{dir}/{locale}.toml")).unwrap();
            let value: toml::Value = toml::from_str(&raw).unwrap();
            let mut keys = Vec::new();
            flatten(&value, "", &mut keys);
            keys.sort();
            assert_eq!(
                keys, reference,
                "locale {locale} diverges from the reference key set (en)"
            );
        }
    }

    /// rust-i18n v4 only interpolates `%{name}`: any inherited v3
    /// `{{name}}` syntax would render raw.
    #[test]
    fn no_v3_double_brace_placeholders() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/locales");
        for locale in LOCALES {
            let raw = std::fs::read_to_string(format!("{dir}/{locale}.toml")).unwrap();
            assert!(
                !raw.contains("{{"),
                "{locale} : placeholder v3 `{{{{...}}}}` interdit (syntaxe v4 : %{{...}})"
            );
        }
    }

    #[test]
    fn interpolation_renders_values() {
        rust_i18n::set_locale("fr");
        let rendered = rust_i18n::t!("calibration.prompt", button = 3).to_string();
        assert!(rendered.contains('3'), "value not interpolated: {rendered}");
        assert!(
            !rendered.contains("{{") && !rendered.contains("%{"),
            "{}",
            rendered
        );
        let rendered = rust_i18n::t!("calibration.progress", captured = 7, total = 12).to_string();
        assert!(
            rendered.contains("7/12"),
            "values not interpolated: {rendered}"
        );
    }

    #[test]
    fn settings_roundtrip() {
        let original = GuiSettings {
            locale: Some("de".to_owned()),
        };
        let raw = toml::to_string_pretty(&original).unwrap();
        let reloaded: GuiSettings = toml::from_str(&raw).unwrap();
        assert_eq!(reloaded.locale.as_deref(), Some("de"));
    }

    #[test]
    fn effective_locale_falls_back_to_system() {
        let settings = GuiSettings::default();
        assert!(LOCALES.contains(&settings.effective_locale().as_str()));
        let bogus = GuiSettings {
            locale: Some("xx".to_owned()),
        };
        assert!(LOCALES.contains(&bogus.effective_locale().as_str()));
    }
}
