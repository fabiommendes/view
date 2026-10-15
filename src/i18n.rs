//! Translated messages, backed by Fluent files embedded in the binary.
//!
//! The language comes from `LC_ALL`, `LC_MESSAGES` and `LANG` (see
//! [`pick_language`]). English is the fallback. To add a language, create
//! `locales/<tag>/view.ftl` and add one line to [`LANGUAGES`].

use std::sync::OnceLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// A supported language: the prefix of the locale value that selects it,
/// its BCP 47 tag and its messages.
struct Language {
    prefix: &'static str,
    tag: &'static str,
    ftl: &'static str,
}

/// Supported languages. The first one is the fallback.
const LANGUAGES: &[Language] = &[
    Language {
        prefix: "en",
        tag: "en",
        ftl: include_str!("../locales/en/view.ftl"),
    },
    Language {
        prefix: "pt",
        tag: "pt-BR",
        ftl: include_str!("../locales/pt-BR/view.ftl"),
    },
    Language {
        prefix: "es",
        tag: "es",
        ftl: include_str!("../locales/es/view.ftl"),
    },
];

type Bundle = FluentBundle<FluentResource>;

struct Catalog {
    selected: Bundle,
    fallback: Bundle,
}

static CATALOG: OnceLock<Catalog> = OnceLock::new();

/// Picks the language from the values of `LC_ALL`, `LC_MESSAGES` and `LANG`
/// (in this order). The first non-empty value decides; anything it does not
/// match (including `C` and `POSIX`) means English.
fn pick_language(values: [Option<&str>; 3]) -> &'static Language {
    let fallback = &LANGUAGES[0];
    let Some(value) = values.into_iter().flatten().find(|v| !v.is_empty()) else {
        return fallback;
    };
    let lang = value.split(['_', '.', '@']).next().unwrap_or_default();
    LANGUAGES
        .iter()
        .find(|l| l.prefix == lang)
        .unwrap_or(fallback)
}

fn build_bundle(language: &Language) -> Bundle {
    let id: LanguageIdentifier = language.tag.parse().expect("valid language tag");
    let resource = FluentResource::try_new(language.ftl.to_owned())
        .unwrap_or_else(|(resource, _errors)| resource);
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    bundle.set_use_isolating(false);
    // Duplicate ids are the only possible error; the first definition stays.
    let _ = bundle.add_resource(resource);
    bundle
}

fn catalog() -> &'static Catalog {
    CATALOG.get_or_init(|| {
        // Unit tests always run in pt-BR, whatever the host environment says.
        #[cfg(test)]
        let language = &LANGUAGES[1];
        #[cfg(not(test))]
        let var = |name| std::env::var(name).ok();
        #[cfg(not(test))]
        let (all, messages, lang) = (var("LC_ALL"), var("LC_MESSAGES"), var("LANG"));
        #[cfg(not(test))]
        let language = pick_language([all.as_deref(), messages.as_deref(), lang.as_deref()]);
        Catalog {
            selected: build_bundle(language),
            fallback: build_bundle(&LANGUAGES[0]),
        }
    })
}

fn format(bundle: &Bundle, id: &str, args: Option<&FluentArgs>) -> Option<String> {
    let pattern = bundle.get_message(id)?.value()?;
    let mut errors = vec![];
    Some(
        bundle
            .format_pattern(pattern, args, &mut errors)
            .into_owned(),
    )
}

/// Translates message `id`. A missing message falls back to English and then
/// to the id itself; it never panics.
pub fn t(id: &str) -> String {
    translate(id, None)
}

/// Like [`t`], with named arguments for the message placeholders.
pub fn t_args(id: &str, args: &[(&str, &str)]) -> String {
    let mut fluent_args = FluentArgs::new();
    for (name, value) in args {
        fluent_args.set(*name, FluentValue::from(*value));
    }
    translate(id, Some(&fluent_args))
}

fn translate(id: &str, args: Option<&FluentArgs>) -> String {
    let catalog = catalog();
    format(&catalog.selected, id, args)
        .or_else(|| format(&catalog.fallback, id, args))
        .unwrap_or_else(|| id.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(values: [Option<&str>; 3]) -> &'static str {
        pick_language(values).tag
    }

    #[test]
    fn maps_language_prefixes() {
        assert_eq!(tag([Some("pt_BR.UTF-8"), None, None]), "pt-BR");
        assert_eq!(tag([Some("pt"), None, None]), "pt-BR");
        assert_eq!(tag([Some("pt_PT"), None, None]), "pt-BR");
        assert_eq!(tag([Some("es_AR"), None, None]), "es");
        assert_eq!(tag([Some("en_GB.UTF-8"), None, None]), "en");
        assert_eq!(tag([Some("pt@euro"), None, None]), "pt-BR");
    }

    #[test]
    fn unknown_values_mean_english() {
        for value in ["de_DE.UTF-8", "C", "POSIX", "C.UTF-8"] {
            assert_eq!(tag([Some(value), None, None]), "en", "{value}");
        }
        assert_eq!(tag([None, None, None]), "en");
    }

    #[test]
    fn empty_values_count_as_unset() {
        assert_eq!(tag([Some(""), Some(""), Some("es_ES")]), "es");
        assert_eq!(tag([Some(""), None, Some("pt_BR")]), "pt-BR");
    }

    #[test]
    fn first_set_variable_decides() {
        assert_eq!(tag([Some("de_DE"), Some("pt_BR"), Some("es")]), "en");
        assert_eq!(tag([None, Some("es"), Some("pt_BR")]), "es");
        assert_eq!(tag([Some("C"), Some("pt_BR"), None]), "en");
    }

    #[test]
    fn missing_message_returns_id() {
        assert_eq!(t("no-such-message"), "no-such-message");
    }

    #[test]
    fn bundles_have_no_isolation_marks() {
        let text = format(
            &build_bundle(&LANGUAGES[1]),
            "error-unknown-format",
            Some(&{
                let mut a = FluentArgs::new();
                a.set("name", "x");
                a
            }),
        )
        .unwrap();
        assert_eq!(text, "formato desconhecido: x");
    }
}
