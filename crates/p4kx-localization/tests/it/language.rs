//! Language names, codes and Entry paths, through the public API.

use p4kx_localization::Language;

#[test]
fn the_eleven_languages_of_a_build_name_their_code_folder_and_entry() {
    let folders = [
        "english",
        "german_(germany)",
        "french_(france)",
        "italian_(italy)",
        "spanish_(spain)",
        "spanish_(latin_america)",
        "portuguese_(brazil)",
        "japanese_(japan)",
        "korean_(south_korea)",
        "chinese_(simplified)",
        "chinese_(traditional)",
    ];
    let codes = [
        "en", "de", "fr", "it", "es-ES", "es-419", "pt-BR", "ja", "ko", "zh-Hans", "zh-Hant",
    ];
    assert_eq!(Language::ALL.len(), folders.len(), "eleven");
    for ((language, folder), code) in Language::ALL.into_iter().zip(folders).zip(codes) {
        assert_eq!(language.folder(), folder, "folder of {language}");
        assert_eq!(language.code(), code, "code of {language}");
        assert_eq!(language.to_string(), code, "display of {language}");
        assert_eq!(
            language.global_ini_path(),
            format!("Data\\Localization\\{folder}\\global.ini"),
            "Entry of {language}"
        );
    }
}

#[test]
fn an_entry_path_names_its_language_ignoring_case_and_separator_kind() {
    for language in Language::ALL {
        let path = language.global_ini_path();
        assert_eq!(
            Language::from_global_ini_path(path),
            Some(language),
            "{path}"
        );
        assert_eq!(
            Language::from_global_ini_path(&path.replace('\\', "/").to_uppercase()),
            Some(language),
            "{path} upper case with /"
        );
    }
}

#[test]
fn other_paths_belong_to_no_language() {
    for path in [
        "Data\\Localization\\klingon\\global.ini",
        "Data\\Localization\\english\\global.ini.bak",
        "Data\\Localization\\english\\extra\\global.ini",
        "Data\\Localization\\global.ini",
        "Data\\Other\\english\\global.ini",
        "Localization\\english\\global.ini",
        "",
    ] {
        assert_eq!(Language::from_global_ini_path(path), None, "{path:?}");
    }
}

#[test]
fn a_language_parses_from_its_code_folder_or_unambiguous_name() {
    for language in Language::ALL {
        assert_eq!(language.code().parse(), Ok(language), "code");
        assert_eq!(
            language.code().to_lowercase().parse(),
            Ok(language),
            "lower case"
        );
        assert_eq!(language.folder().parse(), Ok(language), "folder");
    }
    assert_eq!(
        "es_419".parse(),
        Ok(Language::SpanishLatinAmerica),
        "underscore"
    );
    assert_eq!("german".parse(), Ok(Language::German), "name");
    assert_eq!(
        "pt".parse(),
        Ok(Language::PortugueseBrazil),
        "unique primary tag"
    );
    assert_eq!(" EN ".parse(), Ok(Language::English), "trimmed");
}

#[test]
fn an_ambiguous_or_unknown_text_is_no_language() {
    for text in ["es", "spanish", "zh", "chinese", "klingon", ""] {
        assert!(text.parse::<Language>().is_err(), "{text:?}");
    }
    let message = "xx"
        .parse::<Language>()
        .map(|_| ())
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("zh-Hans") && message.contains("\"xx\""),
        "{message}"
    );
}
