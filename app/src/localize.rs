//! UI translations (Fluent files in `app/i18n/<language>/`), the same system
//! libcosmic and COSMIC's own apps use. English is the base language;
//! `es-419` (Latin America) is a full translation and `es` (Spain) only holds
//! the strings that differ from it.

use std::sync::LazyLock;

use i18n_embed::fluent::{fluent_language_loader, FluentLanguageLoader};
use i18n_embed::unic_langid::LanguageIdentifier;
use i18n_embed::LanguageLoader;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();
    loader
        .load_fallback_language(&Localizations)
        .expect("the English translations are embedded in the binary");
    // No invisible Unicode direction marks around inserted values (file
    // names, counts): they'd end up in dialogs and copied text.
    loader.set_use_isolating(false);
    loader
});

/// A translated string: `fl!("message-id")` or `fl!("message-id", name = value)`.
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id)
    }};
    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

/// The language choices offered in Settings: (code saved in the settings
/// file, name shown). "system" follows the desktop's language.
pub const LANGUAGES: [(&str, &str); 4] = [
    ("system", ""), // shown as the translated "System default"
    ("en", "English"),
    ("es", "Español (España)"),
    ("es-419", "Español (Latinoamérica)"),
];

/// Switches the UI to `code` ("system", "en", "es" or "es-419"). Takes effect
/// on the next redraw, since every view reads its strings through `fl!`.
pub fn set_language(code: &str) {
    let chosen = match code {
        "system" => system_language(),
        other => other.to_string(),
    };
    let chain: Vec<LanguageIdentifier> = fallback_chain(&chosen)
        .iter()
        .filter_map(|id| id.parse().ok())
        .collect();
    if let Err(err) = LANGUAGE_LOADER.load_languages(&Localizations, &chain) {
        eprintln!("failed to load translations for {chosen}: {err}");
    }
    // Loading creates fresh bundles, which isolate inserted values again.
    LANGUAGE_LOADER.set_use_isolating(false);
}

/// The languages to look strings up in, most specific first (English, the
/// fallback, is always searched last by the loader).
fn fallback_chain(language: &str) -> Vec<&'static str> {
    match language {
        "es" => vec!["es", "es-419", "en"],
        "es-419" => vec!["es-419", "en"],
        _ => vec!["en"],
    }
}

/// The supported language closest to the desktop's: Spanish from Spain (or
/// with no country) is "es", Spanish from anywhere else is "es-419".
fn system_language() -> String {
    let requested = i18n_embed::DesktopLanguageRequester::requested_languages();
    requested
        .iter()
        .find_map(|id| supported_for(id.language.as_str(), id.region.map(|r| r.to_string())))
        .unwrap_or_else(|| "en".to_string())
}

fn supported_for(language: &str, region: Option<String>) -> Option<String> {
    match (language, region.as_deref()) {
        ("es", None | Some("ES")) => Some("es".to_string()),
        ("es", Some(_)) => Some("es-419".to_string()),
        ("en", _) => Some("en".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test only: the loader is global, so parallel tests switching
    // languages would race.
    #[test]
    fn languages_resolve_with_fallbacks() {
        set_language("en");
        assert_eq!(crate::fl!("menu-file"), "File");

        set_language("es-419");
        assert_eq!(crate::fl!("menu-file"), "Archivo");
        assert_eq!(crate::fl!("settings"), "Configuración");
        assert_eq!(crate::fl!("status-items", count = 1), "1 elemento");
        assert_eq!(crate::fl!("status-items", count = 3), "3 elementos");

        // Spain: its own word where it differs, Latin American otherwise.
        set_language("es");
        assert_eq!(crate::fl!("settings"), "Ajustes");
        assert_eq!(crate::fl!("menu-file"), "Archivo");
        assert_eq!(
            crate::fl!("delete-one", name = "a.txt"),
            "¿Mover «a.txt» a la papelera?"
        );

        set_language("en");
        assert_eq!(crate::fl!("status-items", count = 1), "1 item");
    }

    /// Message IDs defined in one embedded `.ftl` file.
    fn keys(language: &str) -> std::collections::BTreeSet<String> {
        let file = Localizations::get(&format!("{language}/pa2.ftl"))
            .expect("translation file is embedded");
        std::str::from_utf8(&file.data)
            .unwrap()
            .lines()
            .filter(|line| line.starts_with(|c: char| c.is_ascii_lowercase()))
            .filter_map(|line| line.split_once(" =").map(|(key, _)| key.to_string()))
            .collect()
    }

    #[test]
    fn translations_are_complete() {
        let english = keys("en");
        let latam = keys("es-419");
        let missing: Vec<_> = english.difference(&latam).collect();
        assert!(missing.is_empty(), "es-419 lacks: {missing:?}");
        let unknown: Vec<_> = latam.difference(&english).collect();
        assert!(unknown.is_empty(), "es-419 has unknown keys: {unknown:?}");

        // Spain only overrides; every key it has must exist in English.
        let spain = keys("es");
        let unknown: Vec<_> = spain.difference(&english).collect();
        assert!(unknown.is_empty(), "es has unknown keys: {unknown:?}");
        assert!(!spain.is_empty());
    }

    #[test]
    fn maps_desktop_locales_to_supported_languages() {
        assert_eq!(supported_for("es", Some("ES".into())).as_deref(), Some("es"));
        assert_eq!(supported_for("es", None).as_deref(), Some("es"));
        assert_eq!(supported_for("es", Some("CL".into())).as_deref(), Some("es-419"));
        assert_eq!(supported_for("es", Some("MX".into())).as_deref(), Some("es-419"));
        assert_eq!(supported_for("en", Some("US".into())).as_deref(), Some("en"));
        assert_eq!(supported_for("fr", Some("FR".into())), None);
    }
}
