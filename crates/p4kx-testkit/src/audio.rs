// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt::Write as _;

use crate::{
    CryXmlByteOrder, CryXmlSignature, DataForgeBuilder, TestElement, TestStruct, TestType,
};

/// Adds the structs of Entity audio trigger references to `builder`, next to
/// [`entity_schema`](crate::entity_schema).
///
/// They are shaped as in the game data: `GlobalResourceAudio` with its `audioTrigger` name, held
/// by `SAudioTestComponentParams` directly and through the sound trigger effects of its
/// `triggerEffects`, and `SMeleeAudioTestParams` with a string field `audioTriggerName` that names
/// a trigger and a string field `soundTag` that does not.
#[must_use]
pub fn audio_schema(builder: DataForgeBuilder) -> DataForgeBuilder {
    [
        TestStruct::new("GlobalResourceAudio").field("audioTrigger", TestType::String),
        TestStruct::new("SAudioTestComponentParams")
            .parent("DataForgeComponentParams")
            .array(
                "triggers",
                TestType::Class("GlobalResourceAudio".to_owned()),
            )
            .array(
                "triggerEffects",
                TestType::StrongPointer("SEntityEffectSystem_SoundTriggerEffect".to_owned()),
            ),
        TestStruct::new("SMeleeAudioTestParams")
            .parent("DataForgeComponentParams")
            .field("audioTriggerName", TestType::String)
            .field("soundTag", TestType::String),
        TestStruct::new("SEntityEffectSystem_SoundTriggerEffect")
            .field("name", TestType::String)
            .field("audio", TestType::Class("GlobalResourceAudio".to_owned())),
    ]
    .into_iter()
    .fold(builder, DataForgeBuilder::structure)
}

/// One `ATLTrigger` of a [`TestAtl`]: its name and the Wwise events it posts.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TestTrigger {
    name: String,
    events: Vec<(String, bool)>,
}

/// A synthetic ATL definition file, an `ATLConfig` with audio triggers and preloaded soundbanks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestAtl {
    name: String,
    triggers: Vec<TestTrigger>,
    banks: Vec<String>,
}

impl TestAtl {
    /// Creates an `ATLConfig` named `name` without triggers.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            triggers: Vec::new(),
            banks: Vec::new(),
        }
    }

    /// Adds the trigger `name` posting the Wwise events `events` in order.
    #[must_use]
    pub fn trigger(mut self, name: impl Into<String>, events: &[&str]) -> Self {
        self.triggers.push(TestTrigger {
            name: name.into(),
            events: events
                .iter()
                .map(|event| ((*event).to_owned(), false))
                .collect(),
        });
        self
    }

    /// Adds the trigger `name` posting the localised Wwise event `event`.
    #[must_use]
    pub fn localised_trigger(mut self, name: impl Into<String>, event: impl Into<String>) -> Self {
        self.triggers.push(TestTrigger {
            name: name.into(),
            events: vec![(event.into(), true)],
        });
        self
    }

    /// Adds the soundbank file `file`, e.g. `Ship.bnk`, to the file's preload request.
    #[must_use]
    pub fn bank(mut self, file: impl Into<String>) -> Self {
        self.banks.push(file.into());
        self
    }

    /// Returns the file as an element tree.
    #[must_use]
    pub fn element(&self) -> TestElement {
        let triggers =
            self.triggers
                .iter()
                .fold(TestElement::new("AudioTriggers"), |list, trigger| {
                    let element = trigger.events.iter().fold(
                        TestElement::new("ATLTrigger").attribute("atl_name", &trigger.name),
                        |element, (event, localised)| {
                            let mut event_element = TestElement::new("WwiseEvent")
                                .attribute("wwise_duration_type", "OneShot")
                                .attribute("wwise_name", event);
                            if *localised {
                                event_element = event_element.attribute("wwise_localised", "true");
                            }
                            element.child(event_element)
                        },
                    );
                    list.child(
                        element.child(
                            TestElement::new("ATLPreload").attribute("atl_name", &self.name),
                        ),
                    )
                });
        let group = self.banks.iter().fold(
            TestElement::new("ATLConfigGroup").attribute("atl_name", "default"),
            |group, bank| group.child(TestElement::new("WwiseFile").attribute("wwise_name", bank)),
        );
        TestElement::new("ATLConfig")
            .attribute("atl_name", &self.name)
            .child(triggers)
            .child(
                TestElement::new("AudioPreloads").child(
                    TestElement::new("ATLPreloadRequest")
                        .attribute("atl_name", &self.name)
                        .attribute("path", "Banks/")
                        .child(group),
                ),
            )
    }

    /// Returns the file as little-endian `CryXmlB`, as most Archives store it.
    #[must_use]
    pub fn to_cryxml(&self) -> Vec<u8> {
        self.element()
            .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
    }

    /// Returns the file as UTF-8 text XML.
    #[must_use]
    pub fn to_xml(&self) -> String {
        let mut text = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ATLConfig atl_name=\"{}\">\n  <AudioTriggers>\n",
            escape(&self.name)
        );
        for trigger in &self.triggers {
            let _infallible = writeln!(
                text,
                "    <ATLTrigger atl_name=\"{}\">",
                escape(&trigger.name)
            );
            for (event, localised) in &trigger.events {
                let localised = if *localised {
                    " wwise_localised=\"true\""
                } else {
                    ""
                };
                let _infallible = writeln!(
                    text,
                    "      <WwiseEvent wwise_name=\"{}\"{localised}/>",
                    escape(event)
                );
            }
            text.push_str("    </ATLTrigger>\n");
        }
        text.push_str("  </AudioTriggers>\n</ATLConfig>\n");
        text
    }
}

/// Returns the Wwise short ID of `name`: the 32-bit FNV-1 hash of its ASCII lower case.
#[must_use]
pub fn wwise_short_id(name: &str) -> u32 {
    name.bytes().fold(0x811C_9DC5_u32, |hash, byte| {
        hash.wrapping_mul(0x0100_0193) ^ u32::from(byte.to_ascii_lowercase())
    })
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
