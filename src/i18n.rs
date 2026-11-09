use crate::theme::AppLocale;
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use unic_langid::LanguageIdentifier;

static LOCALE: Lazy<RwLock<&'static str>> = Lazy::new(|| RwLock::new("en"));

const EN: &str = include_str!("../i18n/en/di-player.ftl");
const CS: &str = include_str!("../i18n/cs/di-player.ftl");
const DE: &str = include_str!("../i18n/de/di-player.ftl");

pub fn set_locale(locale: AppLocale) {
    *LOCALE.write() = locale.resolve();
}

pub fn current_lang() -> &'static str {
    *LOCALE.read()
}

fn resource_for(lang: &str) -> &'static str {
    match lang {
        "cs" => CS,
        "de" => DE,
        _ => EN,
    }
}

fn make_bundle(source: &str, lang: &str) -> Option<FluentBundle<FluentResource>> {
    let lang_id: LanguageIdentifier = lang.parse().ok()?;
    let mut bundle = FluentBundle::new(vec![lang_id]);
    bundle.set_use_isolating(false);
    let res = FluentResource::try_new(source.to_string()).ok()?;
    bundle.add_resource(res).ok()?;
    Some(bundle)
}

pub fn t(key: &str) -> String {
    t_args(key, None)
}

pub fn t_args(key: &str, args: Option<&FluentArgs>) -> String {
    let lang = current_lang();
    if let Some(value) = lookup(resource_for(lang), lang, key, args) {
        return value;
    }
    if lang != "en" {
        if let Some(value) = lookup(EN, "en", key, args) {
            return value;
        }
    }
    key.to_string()
}

fn lookup(source: &str, lang: &str, key: &str, args: Option<&FluentArgs>) -> Option<String> {
    let bundle = make_bundle(source, lang)?;
    let msg = bundle.get_message(key)?;
    let pattern = msg.value()?;
    let mut errors = vec![];
    Some(bundle.format_pattern(pattern, args, &mut errors).to_string())
}

pub fn t_str(key: &str, name: &str, value: &str) -> String {
    let mut args = FluentArgs::new();
    args.set(name, value);
    t_args(key, Some(&args))
}

pub fn t_n(key: &str, name: &str, value: i64) -> String {
    let mut args = FluentArgs::new();
    args.set(name, value);
    t_args(key, Some(&args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::AppLocale;

    #[test]
    fn english_and_czech_keys() {
        set_locale(AppLocale::English);
        assert_eq!(t("app-title"), "DI Player");
        set_locale(AppLocale::Czech);
        assert_eq!(t("settings-title"), "Nastavení");
        set_locale(AppLocale::German);
        assert_eq!(t("settings-title"), "Einstellungen");
        set_locale(AppLocale::English);
    }
}
