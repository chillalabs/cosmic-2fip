//! UI translations (Fluent files in `app/i18n/<language>/`), the same system
//! libcosmic and COSMIC's own apps use. English is the base language; German,
//! French, Italian, Latin American Spanish (`es-419`) and Brazilian Portuguese
//! (`pt-BR`) are full translations. Spain Spanish (`es`) and European
//! Portuguese (`pt`) only hold the strings that differ from `es-419` / `pt-BR`.

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
pub const LANGUAGES: [(&str, &str); 9] = [
    ("system", ""), // shown as the translated "System default"
    ("de", "Deutsch"),
    ("en", "English"),
    ("es", "Español (España)"),
    ("es-419", "Español (Latinoamérica)"),
    ("fr", "Français"),
    ("it", "Italiano"),
    ("pt", "Português (Portugal)"),
    ("pt-BR", "Português (Brasil)"),
];

/// Translations that only override another one: (language, the full
/// translation its other strings come from).
const OVERRIDES: [(&str, &str); 2] = [("es", "es-419"), ("pt", "pt-BR")];

/// Switches the UI to `code` (see [`LANGUAGES`]). Takes effect
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
    let Some(&(code, _)) = LANGUAGES.iter().find(|(code, _)| *code == language) else {
        return vec!["en"];
    };
    let mut chain = vec![code];
    if let Some(&(_, base)) = OVERRIDES.iter().find(|(overriding, _)| *overriding == code) {
        chain.push(base);
    }
    if code != "en" {
        chain.push("en");
    }
    chain
}

/// The supported language closest to the desktop's: Spanish from Spain (or
/// with no country) is "es", other Spanish "es-419"; Portuguese from Brazil is
/// "pt-BR", other Portuguese "pt"; German, French, Italian from anywhere.
fn system_language() -> String {
    let requested = i18n_embed::DesktopLanguageRequester::requested_languages();
    requested
        .iter()
        .find_map(|id| supported_for(id.language.as_str(), id.region.map(|r| r.to_string())))
        .unwrap_or_else(|| "en".to_string())
}

fn supported_for(language: &str, region: Option<String>) -> Option<String> {
    let code = match (language, region.as_deref()) {
        ("es", None | Some("ES")) => "es",
        ("es", Some(_)) => "es-419",
        ("pt", Some("BR")) => "pt-BR",
        ("pt", _) => "pt",
        ("en" | "de" | "fr" | "it", _) => language,
        _ => return None,
    };
    Some(code.to_string())
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

        set_language("de");
        assert_eq!(crate::fl!("menu-file"), "Datei");
        set_language("fr");
        assert_eq!(crate::fl!("menu-file"), "Fichier");
        // French spacing: non-breaking spaces inside « » and before "?".
        assert_eq!(
            crate::fl!("delete-one", name = "a.txt"),
            "Placer «\u{a0}a.txt\u{a0}» dans la corbeille\u{a0}?"
        );
        set_language("it");
        assert_eq!(crate::fl!("menu-file"), "File");
        assert_eq!(crate::fl!("status-items", count = 2), "2 elementi");

        // Portugal: its own word where it differs, Brazilian otherwise.
        set_language("pt-BR");
        assert_eq!(crate::fl!("menu-file"), "Arquivo");
        set_language("pt");
        assert_eq!(crate::fl!("menu-file"), "Ficheiro");
        assert_eq!(crate::fl!("paste"), "Colar");

        set_language("en");
        assert_eq!(crate::fl!("status-items", count = 1), "1 item");
    }

    #[test]
    fn fallback_chains_go_from_specific_to_english() {
        assert_eq!(fallback_chain("pt"), ["pt", "pt-BR", "en"]);
        assert_eq!(fallback_chain("es"), ["es", "es-419", "en"]);
        assert_eq!(fallback_chain("de"), ["de", "en"]);
        assert_eq!(fallback_chain("en"), ["en"]);
        assert_eq!(fallback_chain("xx"), ["en"]);
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
        for (language, _) in LANGUAGES.iter().filter(|(code, _)| !["system", "en"].contains(code)) {
            let translated = keys(language);
            let unknown: Vec<_> = translated.difference(&english).collect();
            assert!(unknown.is_empty(), "{language} has unknown keys: {unknown:?}");
            let is_override = OVERRIDES.iter().any(|(code, _)| code == language);
            if is_override {
                assert!(!translated.is_empty(), "{language} is empty");
            } else {
                let missing: Vec<_> = english.difference(&translated).collect();
                assert!(missing.is_empty(), "{language} lacks: {missing:?}");
            }
        }
    }

    #[test]
    fn maps_desktop_locales_to_supported_languages() {
        assert_eq!(supported_for("es", Some("ES".into())).as_deref(), Some("es"));
        assert_eq!(supported_for("es", None).as_deref(), Some("es"));
        assert_eq!(supported_for("es", Some("CL".into())).as_deref(), Some("es-419"));
        assert_eq!(supported_for("es", Some("MX".into())).as_deref(), Some("es-419"));
        assert_eq!(supported_for("en", Some("US".into())).as_deref(), Some("en"));
        assert_eq!(supported_for("pt", Some("BR".into())).as_deref(), Some("pt-BR"));
        assert_eq!(supported_for("pt", Some("PT".into())).as_deref(), Some("pt"));
        assert_eq!(supported_for("de", Some("AT".into())).as_deref(), Some("de"));
        assert_eq!(supported_for("fr", Some("CA".into())).as_deref(), Some("fr"));
        assert_eq!(supported_for("it", None).as_deref(), Some("it"));
        assert_eq!(supported_for("ja", Some("JP".into())), None);
    }
}
