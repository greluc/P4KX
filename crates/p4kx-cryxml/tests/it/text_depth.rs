// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_cryxml::{MAX_TEXT_DEPTH, check_text_depth};

fn nested(depth: usize) -> String {
    format!("{}{}", "<a>".repeat(depth), "</a>".repeat(depth))
}

#[test]
fn the_limit_itself_is_read_and_one_more_is_refused() {
    assert!(check_text_depth(&nested(MAX_TEXT_DEPTH)).is_ok());
    let error = check_text_depth(&nested(MAX_TEXT_DEPTH + 1)).err();
    assert_eq!(error.map(|error| error.limit()), Some(MAX_TEXT_DEPTH));
}

#[test]
fn self_closing_tags_and_closed_siblings_do_not_add_depth() {
    let text = format!("<r>{}</r>", "<a/><b></b>".repeat(10_000));
    assert!(check_text_depth(&text).is_ok());
}

#[test]
fn markup_hidden_in_comments_cdata_and_instructions_is_skipped() {
    let deep = "<a>".repeat(MAX_TEXT_DEPTH + 1);
    for hidden in [
        format!("<![CDATA[{deep}]]>"),
        format!("<!-- {deep} -->"),
        format!("<?p {deep} ?>"),
    ] {
        assert!(check_text_depth(&hidden).is_ok(), "{hidden:.30}");
    }
    let closers = "</a>".repeat(MAX_TEXT_DEPTH);
    let text = format!("{}<!-- {closers} -->{deep}", "<a>".repeat(MAX_TEXT_DEPTH));
    assert!(check_text_depth(&text).is_err());
}

#[test]
fn a_quoted_angle_bracket_does_not_end_a_tag() {
    let text = format!("<r a=\">\">{}</r>", nested(MAX_TEXT_DEPTH - 1));
    assert!(check_text_depth(&text).is_ok());
}

#[test]
fn truncated_input_does_not_panic() {
    for text in ["<", "<a", "<a b=\"", "<!--", "<![CDATA[", "<?", "</"] {
        assert!(check_text_depth(text).is_ok(), "{text}");
    }
}
