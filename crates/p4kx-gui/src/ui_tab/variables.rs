// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_screens::{MAX_VARIABLES, Variable};

use super::settings::UiPower;

/// The game variables a screen is drawn with: the settings of the state, then those typed.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Chosen {
    pub(super) variables: Vec<Variable>,
    pub(super) invalid: Vec<String>,
}

/// Reads the `NAME=VALUE` settings of `power` and of `text`, which are separated by blanks or
/// commas; a setting typed later wins over an earlier one. Reads at most one more than
/// [`MAX_VARIABLES`] typed settings, so that the renderer can refuse the excess.
pub(super) fn chosen(power: UiPower, text: &str) -> Chosen {
    let mut chosen = Chosen::default();
    let typed = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|setting| !setting.is_empty())
        .take(MAX_VARIABLES + 1);
    for setting in power.settings().iter().copied().chain(typed) {
        match setting.parse::<Variable>() {
            Ok(variable) => chosen.variables.push(variable),
            Err(_) => chosen.invalid.push(setting.to_owned()),
        }
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(chosen: &Chosen) -> Vec<(&str, &str)> {
        chosen
            .variables
            .iter()
            .map(|variable| (variable.name(), variable.value()))
            .collect()
    }

    #[test]
    fn at_rest_with_nothing_typed_sets_nothing() {
        assert_eq!(chosen(UiPower::AtRest, "  "), Chosen::default());
    }

    #[test]
    fn powered_on_sets_the_power_state_and_typed_settings_follow_it() {
        let chosen = chosen(UiPower::PoweredOn, "isradar=1, powerstate=0\nmode=a");
        assert_eq!(
            pairs(&chosen),
            [
                ("powerstate", "1"),
                ("isradar", "1"),
                ("powerstate", "0"),
                ("mode", "a")
            ]
        );
        assert_eq!(chosen.invalid.first(), None);
    }

    #[test]
    fn what_is_no_setting_is_listed() {
        let chosen = chosen(UiPower::AtRest, "good=1 bad =2 =3");
        assert_eq!(pairs(&chosen), [("good", "1")]);
        assert_eq!(chosen.invalid, ["bad", "=2", "=3"]);
    }

    #[test]
    fn the_typed_settings_are_bounded() {
        let text = "a=1 ".repeat(MAX_VARIABLES * 3);
        assert_eq!(
            chosen(UiPower::AtRest, &text).variables.len(),
            MAX_VARIABLES + 1
        );
    }
}
