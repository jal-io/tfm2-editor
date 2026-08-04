use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const DEFAULT_LANGUAGE: &str = "en-US";
const BUILTIN_ENGLISH: &str = include_str!("../locales/en-US.json");
const EXTERNAL_ENGLISH_FILENAME: &str = "en-US.json";

#[derive(Debug, Clone, Deserialize)]
struct LocaleFile {
    locale: String,
    name: String,
    strings: HashMap<String, String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct AppSettings {
    language: String,
}

pub struct Localization {
    fallback: LocaleFile,
    locales: HashMap<String, LocaleFile>,
    current_language: String,
    #[cfg(feature = "dev")]
    diagnostics: Vec<String>,
    missing_keys: RefCell<BTreeSet<String>>,
    #[cfg(feature = "dev")]
    locale_dir: PathBuf,
}

impl Localization {
    pub fn load() -> Self {
        let fallback: LocaleFile = serde_json::from_str(BUILTIN_ENGLISH)
            .expect("The built-in en-US locale must be valid JSON");

        let mut locales = HashMap::new();
        locales.insert(fallback.locale.clone(), fallback.clone());

        let mut diagnostics = Vec::new();
        let locale_dir = Self::locale_dir();
        Self::load_external_locales(&locale_dir, &mut locales, &mut diagnostics);

        let requested_language = Self::load_or_create_settings(&mut diagnostics);

        let current_language = if locales.contains_key(&requested_language) {
            requested_language
        } else {
            diagnostics.push(format!(
                "Saved language '{requested_language}' is unavailable; using {DEFAULT_LANGUAGE}"
            ));
            let fallback_settings = AppSettings {
                language: DEFAULT_LANGUAGE.to_string(),
            };
            if let Err(error) = Self::save_settings(&fallback_settings) {
                diagnostics.push(error);
            }
            DEFAULT_LANGUAGE.to_string()
        };

        Self {
            fallback,
            locales,
            current_language,
            #[cfg(feature = "dev")]
            diagnostics,
            missing_keys: RefCell::new(BTreeSet::new()),
            #[cfg(feature = "dev")]
            locale_dir,
        }
    }

    pub fn current_language(&self) -> &str {
        &self.current_language
    }

    pub fn current_language_name(&self) -> &str {
        self.locales
            .get(&self.current_language)
            .map(|locale| locale.name.as_str())
            .unwrap_or(self.fallback.name.as_str())
    }

    pub fn available_languages(&self) -> Vec<(String, String)> {
        let mut languages = self
            .locales
            .values()
            .map(|locale| (locale.locale.clone(), locale.name.clone()))
            .collect::<Vec<_>>();
        languages.sort_by(|left, right| left.1.to_lowercase().cmp(&right.1.to_lowercase()));
        languages
    }

    pub fn select_language(&mut self, language: &str) -> Result<(), String> {
        if !self.locales.contains_key(language) {
            return Err(format!("Language '{language}' is not available"));
        }

        Self::save_settings(&AppSettings {
            language: language.to_string(),
        })?;
        self.current_language = language.to_string();
        Ok(())
    }

    #[cfg(feature = "dev")]
    pub fn reload(&mut self) {
        *self = Self::load();
    }

    pub fn tr(&self, key: &str) -> String {
        if let Some(value) = self
            .locales
            .get(&self.current_language)
            .and_then(|locale| locale.strings.get(key))
        {
            return value.clone();
        }

        if let Some(value) = self.fallback.strings.get(key) {
            if self.current_language != DEFAULT_LANGUAGE {
                self.missing_keys.borrow_mut().insert(key.to_string());
            }
            return value.clone();
        }

        self.missing_keys.borrow_mut().insert(key.to_string());

        #[cfg(feature = "dev")]
        {
            format!("[[{key}]]")
        }

        #[cfg(not(feature = "dev"))]
        {
            key.to_string()
        }
    }

    pub fn tr_with(&self, key: &str, values: &[(&str, &str)]) -> String {
        let mut text = self.tr(key);
        for (name, value) in values {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }

    #[cfg(feature = "dev")]
    pub fn debug_issue_count(&self) -> usize {
        self.diagnostics.len() + self.missing_keys.borrow().len()
    }

    #[cfg(feature = "dev")]
    pub fn debug_report(&self) -> String {
        let mut lines = vec![
            format!("Locale directory: {}", self.locale_dir.display()),
            format!("Active language: {}", self.current_language),
            "Built-in English fallback: embedded in executable".to_string(),
            "External en-US override: enabled in Development builds".to_string(),
        ];
        lines.extend(self.diagnostics.clone());
        lines.extend(
            self.missing_keys
                .borrow()
                .iter()
                .map(|key| format!("Missing translation key: {key}")),
        );

        if self.debug_issue_count() == 0 {
            lines.push("No localization issues detected".to_string());
        }

        lines.join("\n")
    }

    fn load_external_locales(
        locale_dir: &Path,
        locales: &mut HashMap<String, LocaleFile>,
        diagnostics: &mut Vec<String>,
    ) {
        if !locale_dir.is_dir() {
            return;
        }

        // Development builds may override the embedded English locale with an external
        // en-US.json file. This keeps translation work reloadable without making the English
        // file a runtime dependency for Community builds.
        #[cfg(feature = "dev")]
        {
            let external_english_path = locale_dir.join(EXTERNAL_ENGLISH_FILENAME);
            if external_english_path.is_file() {
                match Self::read_locale_file(&external_english_path) {
                    Ok(locale) if locale.locale == DEFAULT_LANGUAGE => {
                        locales.insert(locale.locale.clone(), locale);
                    }
                    Ok(locale) => diagnostics.push(format!(
                        "Invalid locale {}: expected locale '{}', found '{}'",
                        external_english_path.display(),
                        DEFAULT_LANGUAGE,
                        locale.locale
                    )),
                    Err(error) => diagnostics.push(error),
                }
            }
        }

        match fs::read_dir(locale_dir) {
            Ok(entries) => {
                for entry_result in entries {
                    let entry = match entry_result {
                        Ok(entry) => entry,
                        Err(error) => {
                            diagnostics.push(format!(
                                "Could not read an entry in locale directory {}: {error}",
                                locale_dir.display()
                            ));
                            continue;
                        }
                    };

                    let path = entry.path();
                    let is_json = path
                        .extension()
                        .and_then(|value| value.to_str())
                        .map(|value| value.eq_ignore_ascii_case("json"))
                        .unwrap_or(false);
                    if !is_json {
                        continue;
                    }

                    let is_external_english = path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .map(|value| value.eq_ignore_ascii_case(EXTERNAL_ENGLISH_FILENAME))
                        .unwrap_or(false);
                    // en-US is embedded. Development builds already handled it above as a
                    // hot-reloadable override; Community builds intentionally ignore it.
                    if is_external_english {
                        continue;
                    }

                    match Self::read_locale_file(&path) {
                        Ok(locale) => {
                            locales.insert(locale.locale.clone(), locale);
                        }
                        Err(error) => diagnostics.push(error),
                    }
                }
            }
            Err(error) => diagnostics.push(format!(
                "Could not read locale directory {}: {error}",
                locale_dir.display()
            )),
        }
    }

    fn read_locale_file(path: &Path) -> Result<LocaleFile, String> {
        let raw = fs::read_to_string(path)
            .map_err(|error| format!("Could not read locale {}: {error}", path.display()))?;
        let locale: LocaleFile = serde_json::from_str(&raw)
            .map_err(|error| format!("Invalid locale {}: {error}", path.display()))?;

        if locale.locale.trim().is_empty() {
            return Err(format!(
                "Invalid locale {}: 'locale' cannot be empty",
                path.display()
            ));
        }
        if locale.name.trim().is_empty() {
            return Err(format!(
                "Invalid locale {}: 'name' cannot be empty",
                path.display()
            ));
        }

        Ok(locale)
    }

    fn app_dir() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn locale_dir() -> PathBuf {
        Self::app_dir().join("locales")
    }

    fn settings_path() -> PathBuf {
        Self::app_dir().join("tfm2_editor_settings.json")
    }

    fn load_or_create_settings(diagnostics: &mut Vec<String>) -> String {
        let path = Self::settings_path();

        match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<AppSettings>(&raw) {
                Ok(settings) if !settings.language.trim().is_empty() => settings.language,
                Ok(_) => {
                    diagnostics.push(format!(
                        "Invalid settings {}: 'language' cannot be empty; using {DEFAULT_LANGUAGE}",
                        path.display()
                    ));
                    Self::write_default_settings(diagnostics);
                    DEFAULT_LANGUAGE.to_string()
                }
                Err(error) => {
                    diagnostics.push(format!(
                        "Invalid settings {}: {error}; using {DEFAULT_LANGUAGE}",
                        path.display()
                    ));
                    Self::write_default_settings(diagnostics);
                    DEFAULT_LANGUAGE.to_string()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Self::write_default_settings(diagnostics);
                DEFAULT_LANGUAGE.to_string()
            }
            Err(error) => {
                diagnostics.push(format!(
                    "Could not read settings {}: {error}; using {DEFAULT_LANGUAGE}",
                    path.display()
                ));
                DEFAULT_LANGUAGE.to_string()
            }
        }
    }

    fn write_default_settings(diagnostics: &mut Vec<String>) {
        let settings = AppSettings {
            language: DEFAULT_LANGUAGE.to_string(),
        };
        if let Err(error) = Self::save_settings(&settings) {
            diagnostics.push(error);
        }
    }

    fn save_settings(settings: &AppSettings) -> Result<(), String> {
        let path = Self::settings_path();
        let raw = serde_json::to_string_pretty(settings)
            .map_err(|error| format!("Could not serialize settings: {error}"))?;
        fs::write(&path, raw)
            .map_err(|error| format!("Could not save settings to {}: {error}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_english_is_valid() {
        let locale: LocaleFile = serde_json::from_str(BUILTIN_ENGLISH).unwrap();
        assert_eq!(locale.locale, DEFAULT_LANGUAGE);
        assert!(locale.strings.contains_key("tabs.economy"));
    }

    #[test]
    fn placeholders_are_replaced() {
        let localization = Localization::load();
        assert_eq!(
            localization.tr_with("app.made_by", &[("author", "jal-io")]),
            "Made by jal-io"
        );
    }

    #[cfg(feature = "dev")]
    #[test]
    fn invalid_external_english_is_reported() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tfm2_editor_localization_test_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(EXTERNAL_ENGLISH_FILENAME), "{").unwrap();

        let fallback: LocaleFile = serde_json::from_str(BUILTIN_ENGLISH).unwrap();
        let mut locales = HashMap::new();
        locales.insert(fallback.locale.clone(), fallback);
        let mut diagnostics = Vec::new();

        Localization::load_external_locales(&dir, &mut locales, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("Invalid locale"));
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(not(feature = "dev"))]
    #[test]
    fn community_ignores_external_english_override() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tfm2_editor_localization_community_test_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(EXTERNAL_ENGLISH_FILENAME), "{").unwrap();

        let fallback: LocaleFile = serde_json::from_str(BUILTIN_ENGLISH).unwrap();
        let mut locales = HashMap::new();
        locales.insert(fallback.locale.clone(), fallback);
        let mut diagnostics = Vec::new();

        Localization::load_external_locales(&dir, &mut locales, &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert_eq!(locales.len(), 1);
        let _ = fs::remove_dir_all(dir);
    }
}
