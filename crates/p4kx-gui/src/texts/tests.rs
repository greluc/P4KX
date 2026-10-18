use std::collections::BTreeSet;

use fluent_bundle::FluentValue;
use fluent_syntax::ast;

use super::{BRITISH_ENGLISH, GERMAN, Text, Texts};
use crate::Language;

fn message_ids(source: &str) -> BTreeSet<String> {
    let resource = fluent_syntax::parser::parse(source).unwrap_or_else(|(_, errors)| {
        panic!("the Fluent file has syntax errors: {errors:?}");
    });
    resource
        .body
        .iter()
        .filter_map(|entry| match entry {
            ast::Entry::Message(message) => Some(message.id.name.to_owned()),
            _ => None,
        })
        .collect()
}

fn sample_args() -> Vec<(&'static str, FluentValue<'static>)> {
    vec![
        ("name", "Data".into()),
        ("method", "1".into()),
        ("path", "Data.p4k".into()),
        ("reason", "no Archive".into()),
        ("count", 3.into()),
        ("number", "3".into()),
        ("hidden", "1".into()),
        ("build", "4.3.1".into()),
        ("size", "300.0 MiB".into()),
        ("limit", "256.0 MiB".into()),
        ("shown", "1,048,576".into()),
        ("total", "2,000,000".into()),
        ("done", "1".into()),
        ("percent", "50".into()),
        ("extracted", "2".into()),
        ("converted", "1".into()),
        ("skipped", "1".into()),
        ("failed", "0".into()),
        ("cancelled", "0".into()),
        ("written", "3".into()),
        ("errors", "1".into()),
        ("erroneous", "1".into()),
        ("version", "0.2.0".into()),
        ("channel", "LIVE".into()),
        ("change", "Added".into()),
        ("filter", "*.xml".into()),
        ("stopped", "yes".into()),
        ("current", "1".into()),
        ("first", "10".into()),
        ("last", "16".into()),
        ("line", "12".into()),
        ("text", "<a/>".into()),
        ("lines", "4,194,304".into()),
        ("old", "libs/ships/old".into()),
        ("new", "libs/ships/new".into()),
        ("type", "Hull".into()),
        ("at", "nodes[0]".into()),
        ("field", "hull.armor".into()),
        ("level", "2".into()),
        ("width", "512".into()),
        ("height", "256".into()),
        ("format", "BC7_UNORM".into()),
        ("column", "13".into()),
        ("page", "1".into()),
        ("pages", "3".into()),
    ]
}

#[test]
fn both_files_define_exactly_the_texts_of_the_interface() {
    let expected: BTreeSet<String> = Text::all().map(|text| text.id().to_owned()).collect();

    assert_eq!(message_ids(GERMAN), expected, "de-DE");
    assert_eq!(message_ids(BRITISH_ENGLISH), expected, "en-GB");
}

#[test]
fn every_text_formats_without_errors_in_every_language() {
    let texts = Texts::new(Language::German);
    for language in Language::ALL {
        let bundle = texts.bundle(language);
        let mut args = fluent_bundle::FluentArgs::new();
        for (name, value) in sample_args() {
            args.set(name, value);
        }
        for text in Text::all() {
            let pattern = bundle
                .get_message(text.id())
                .and_then(|message| message.value())
                .unwrap_or_else(|| panic!("{} has no value in {language:?}", text.id()));
            let mut errors = Vec::new();
            let _formatted = bundle.format_pattern(pattern, Some(&args), &mut errors);
            assert!(
                errors.is_empty(),
                "{} in {language:?}: {errors:?}",
                text.id()
            );
        }
    }
}

#[test]
fn entry_counts_use_the_plural_of_the_language() {
    let mut texts = Texts::new(Language::BritishEnglish);
    let entries = |texts: &Texts, count: i32| {
        texts.format(
            Text::StatusEntries,
            &[
                ("count", count.into()),
                ("number", count.to_string().into()),
            ],
        )
    };

    assert_eq!(entries(&texts, 1), "1 Entry", "en-GB singular");
    assert_eq!(entries(&texts, 3), "3 Entries", "en-GB plural");
    texts.set_language(Language::German);
    assert_eq!(entries(&texts, 1), "1 Eintrag", "de-DE singular");
    assert_eq!(entries(&texts, 3), "3 Einträge", "de-DE plural");
}

#[test]
fn placeables_are_not_wrapped_in_isolation_marks() {
    let texts = Texts::new(Language::BritishEnglish);

    let text = texts.format(Text::SelectEntry, &[("name", "ship.cgf".into())]);

    assert_eq!(text, "Select ship.cgf", "no FSI/PDI around the name");
}
