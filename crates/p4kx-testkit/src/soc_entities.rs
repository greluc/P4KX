// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::TestElement;

fn join(values: &[f64]) -> String {
    values
        .iter()
        .map(f64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn placed(name: &str, class: &str, position: [f64; 3], rotation: [f64; 4]) -> Entity {
    Entity {
        name: name.to_owned(),
        class: class.to_owned(),
        pos: join(&position),
        rotate: join(&rotation),
        scale: None,
        class_guid: None,
        children: Vec::new(),
    }
}

#[derive(Debug, Clone)]
struct Entity {
    name: String,
    class: String,
    pos: String,
    rotate: String,
    scale: Option<String>,
    class_guid: Option<String>,
    children: Vec<TestElement>,
}

impl Entity {
    fn child(mut self, child: TestElement) -> Self {
        self.children.push(child);
        self
    }

    fn element(&self) -> TestElement {
        let mut element = TestElement::new("Entity")
            .attribute("Name", &self.name)
            .attribute("EntityClass", &self.class)
            .attribute("Pos", &self.pos)
            .attribute("Rotate", &self.rotate);
        if let Some(scale) = &self.scale {
            element = element.attribute("Scale", scale);
        }
        if let Some(guid) = &self.class_guid {
            element = element.attribute("EntityClassGUID", guid);
        }
        self.children
            .iter()
            .cloned()
            .fold(element, TestElement::child)
    }
}

/// The values of one light state of a synthetic light.
#[derive(Debug, Clone, Copy)]
pub struct TestLightState {
    /// The intensity.
    pub intensity: f64,
    /// The colour, red, green and blue.
    pub colour: [f64; 3],
    /// The colour temperature in kelvin.
    pub temperature: f64,
}

/// The `EntityComponentLight` of a synthetic light.
#[derive(Debug, Clone)]
pub struct TestLight {
    light_type: String,
    radius: f64,
    use_temperature: bool,
    fov: f64,
    states: Vec<(&'static str, TestLightState)>,
}

impl TestLight {
    /// Creates a light of the type `light_type` (`Omni`, `Projector`, …) with a default state of
    /// intensity 2, white at 6500 K, a radius of 10 and no projector angle.
    #[must_use]
    pub fn new(light_type: impl Into<String>) -> Self {
        Self {
            light_type: light_type.into(),
            radius: 10.0,
            use_temperature: false,
            fov: 0.0,
            states: vec![(
                "defaultState",
                TestLightState {
                    intensity: 2.0,
                    colour: [1.0; 3],
                    temperature: 6500.0,
                },
            )],
        }
    }

    /// Sets the `lightRadius`.
    #[must_use]
    pub const fn radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }

    /// Sets the projector's full opening angle in degrees.
    #[must_use]
    pub const fn fov(mut self, fov: f64) -> Self {
        self.fov = fov;
        self
    }

    /// Lets the colour temperature tint the light.
    #[must_use]
    pub const fn with_temperature(mut self) -> Self {
        self.use_temperature = true;
        self
    }

    /// Sets the values of the default state.
    #[must_use]
    pub fn default_state(mut self, state: TestLightState) -> Self {
        self.states = vec![("defaultState", state)];
        self
    }

    /// Adds the state `tag` (`auxiliaryState`, `emergencyState`, …) with `state`.
    #[must_use]
    pub fn state(mut self, tag: &'static str, state: TestLightState) -> Self {
        self.states.push((tag, state));
        self
    }

    fn element(&self) -> TestElement {
        let mut component = TestElement::new("EntityComponentLight")
            .attribute("lightType", &self.light_type)
            .attribute(
                "useTemperature",
                if self.use_temperature { "1" } else { "0" },
            )
            .child(
                TestElement::new("sizeParams")
                    .attribute("lightRadius", self.radius.to_string())
                    .attribute("bulbRadius", "0.05"),
            );
        for (tag, state) in &self.states {
            component = component.child(
                TestElement::new(*tag)
                    .attribute("intensity", state.intensity.to_string())
                    .attribute("temperature", state.temperature.to_string())
                    .child(
                        TestElement::new("color")
                            .attribute("r", state.colour[0].to_string())
                            .attribute("g", state.colour[1].to_string())
                            .attribute("b", state.colour[2].to_string()),
                    ),
            );
        }
        component.child(
            TestElement::new("projectorParams")
                .attribute("FOV", self.fov.to_string())
                .attribute("focusedBeam", "0"),
        )
    }
}

/// An entity of a synthetic `.soc` file.
#[derive(Debug, Clone)]
pub struct TestSocEntity {
    entity: Entity,
}

impl TestSocEntity {
    pub(crate) fn element(&self) -> TestElement {
        self.entity.element()
    }

    /// Creates a prop that names its own geometry `model`.
    #[must_use]
    pub fn prop(name: &str, position: [f64; 3], rotation: [f64; 4], model: &str) -> Self {
        Self::prop_choosing(name, position, rotation, model, None, None)
    }

    /// Creates a prop that names its own geometry `model` and, if given, the material library
    /// `material` and the tint palette Record `palette` (GUID text and child path) it is drawn
    /// with.
    #[must_use]
    pub fn prop_choosing(
        name: &str,
        position: [f64; 3],
        rotation: [f64; 4],
        model: &str,
        material: Option<&str>,
        palette: Option<(&str, &str)>,
    ) -> Self {
        let mut data = TestElement::new("Geometry")
            .child(TestElement::new("Geometry").attribute("path", model));
        if let Some(material) = material {
            data = data.child(TestElement::new("Material").attribute("path", material));
        }
        if let Some((root, child)) = palette {
            data = data.child(
                TestElement::new("Palette")
                    .attribute("RootRecord", root)
                    .attribute("ChildPath", child),
            );
        }
        let node = TestElement::new("EntityGeometryResource")
            .child(TestElement::new("Geometry").child(data));
        Self {
            entity: placed(name, "GeomEntity", position, rotation)
                .child(TestElement::new("PropertiesDataCore").child(node)),
        }
    }

    /// Creates an entity of the class `class` that names the Record `guid` as its class and no
    /// geometry of its own, such as a door; its geometry comes from the Record.
    #[must_use]
    pub fn classed(
        name: &str,
        class: &str,
        guid: &str,
        position: [f64; 3],
        rotation: [f64; 4],
    ) -> Self {
        let mut entity = placed(name, class, position, rotation);
        entity.class_guid = Some(guid.to_owned());
        Self { entity }
    }

    /// Adds the baked-in lights `lights` of a light group to the entity, as for
    /// [`Self::light_group`].
    #[must_use]
    pub fn with_lights(mut self, lights: &[(&str, TestLight, [f64; 4], [f64; 3])]) -> Self {
        let group = Self::light_group("", [0.0; 3], [1.0, 0.0, 0.0, 0.0], lights);
        self.entity.children.extend(group.entity.children);
        self
    }

    /// Creates an entity of the class `class` without geometry or light, such as a room.
    #[must_use]
    pub fn plain(name: &str, class: &str, position: [f64; 3]) -> Self {
        Self {
            entity: placed(name, class, position, [1.0, 0.0, 0.0, 0.0]),
        }
    }

    /// Creates an entity of the class `Light`.
    #[must_use]
    pub fn light(name: &str, position: [f64; 3], rotation: [f64; 4], light: &TestLight) -> Self {
        Self {
            entity: placed(name, "Light", position, rotation)
                .child(TestElement::new("PropertiesDataCore").child(light.element())),
        }
    }

    /// Creates a `LightGroup` whose baked-in lights are `lights`, each with its name, rotation
    /// `[w, x, y, z]` and translation relative to the group.
    #[must_use]
    pub fn light_group(
        name: &str,
        position: [f64; 3],
        rotation: [f64; 4],
        lights: &[(&str, TestLight, [f64; 4], [f64; 3])],
    ) -> Self {
        let baked = lights.iter().fold(
            TestElement::new("BakedInLights"),
            |baked, (light_name, light, rotation, translation)| {
                baked.child(
                    TestElement::new("Light")
                        .attribute("Name", *light_name)
                        .child(light.element())
                        .child(
                            TestElement::new("RelativeXForm")
                                .attribute("rotation", join(rotation))
                                .attribute("translation", join(translation)),
                        ),
                )
            },
        );
        Self {
            entity: placed(name, "LightGroup", position, rotation)
                .child(TestElement::new("EntityComponentLightGroup").child(baked)),
        }
    }

    /// Sets the entity's `Scale`.
    #[must_use]
    pub fn scaled(mut self, scale: [f64; 3]) -> Self {
        self.entity.scale = Some(join(&scale));
        self
    }

    /// Replaces the entity's `Pos` with the raw text `pos`, for malformed ones.
    #[must_use]
    pub fn raw_pos(mut self, pos: &str) -> Self {
        pos.clone_into(&mut self.entity.pos);
        self
    }
}
