//! Fixtures of the vehicle implementation tests: a ship, its implementation file and the patch files of its Modifications.

use std::collections::HashMap;
use std::error::Error;

use p4kx_assembly::{
    EntityNode, LoadoutFileError, LoadoutFiles, LoadoutResolver, PortVisibility, VehiclePart,
};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{DataForgeBuilder, TestEntity, TestPort, entity_schema};

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) const DEFINITION: &str = "Scripts/Entities/Vehicles/Implementations/Xml/TEST_Ship.xml";

pub(crate) const BASE: &str = r#"<Vehicle name="TEST_Ship" displayname="x">
  <Parts>
    <Part name="body" class="Animated" mass="1500" damageMax="1000">
      <Parts>
        <Part name="Hardpoint_Gun" class="ItemPort" id="partGun" damageMax="120">
          <ItemPort id="portGun" minSize="1" maxSize="3" flags="invisible $uneditable"/>
        </Part>
        <Part name="hardpoint_seat" class="ItemPort"><ItemPort minsize="2" maxsize="2" flags=""/></Part>
        <Part name="hardpoint_extra" class="ItemPort" id="partExtra" skipPart="1">
          <ItemPort flags="invisible"/>
        </Part>
        <Part name="left_tread" class="Tread">
          <Tread filename="tread.cgf"><Wheels><Wheel partName="wheel_l1"/><Wheel partName="wheel_l2"/></Wheels></Tread>
        </Part>
        <Part name="wheel_l1" class="SubPartWheel" mass="12.5kg"/>
        <Part name="wheel_l2" class="subpartwheel" mass="n/a"/>
      </Parts>
    </Part>
  </Parts>
  <Modifications>
    <Modification name="Open">
      <Elems>
        <Elem idRef="portGun" name="flags" value="$uneditable"/>
        <Elem idRef="partExtra" name="skipPart" value="0"/>
        <Elem idRef="partGun" name="damagemax" value="300"/>
      </Elems>
    </Modification>
    <Modification name="Patched" patchFile="Modifications/TEST_Ship_Patched"/>
    <Modification name="PartsOnly" patchFile="Modifications/Parts_Only.xml"/>
    <Modification name="FullOverride" patchFile="Modifications\Full"/>
    <Modification name="Missing" patchFile="Modifications/Nope"/>
  </Modifications>
</Vehicle>"#;

pub(crate) const PATCHED: &str = r#"<Modifications>
  <Part id="partGun" name="Hardpoint_Gun" class="ItemPort">
    <ItemPort id="portGun" minSize="1" maxSize="3" flags=""/>
  </Part>
</Modifications>"#;

pub(crate) const PARTS_ONLY: &str = r#"<Modifications><Parts>
  <Part name="body" class="Animated" mass="900">
    <Parts><Part name="hardpoint_new" class="ItemPort"><ItemPort flags="invisible"/></Part></Parts>
  </Part>
</Parts></Modifications>"#;

pub(crate) const FULL: &str = r#"<Modifications><Vehicle name="TEST_Ship">
  <Parts><Part name="shell" class="Animated" mass="700"/></Parts>
  <Modifications><Modification name="Ignored"/></Modifications>
</Vehicle></Modifications>"#;

pub(crate) const FALLBACK: &str = r#"<Modifications><Parts>
  <Part name="body" class="Animated">
    <Parts><Part name="hardpoint_fallback" class="ItemPort"><ItemPort flags=""/></Part></Parts>
  </Part>
</Parts></Modifications>"#;

pub(crate) struct Files(pub(crate) HashMap<String, Vec<u8>>);

impl LoadoutFiles for Files {
    fn read(&self, path: &str) -> Result<Vec<u8>, LoadoutFileError> {
        self.0
            .get(&path.to_ascii_lowercase())
            .cloned()
            .ok_or(LoadoutFileError::NotFound)
    }
}

pub(crate) fn files(entries: &[(&str, &str)]) -> Files {
    Files(
        entries
            .iter()
            .map(|(path, text)| (path.to_ascii_lowercase(), text.as_bytes().to_vec()))
            .collect(),
    )
}

pub(crate) fn standard_files() -> Files {
    files(&[
        (DEFINITION, BASE),
        (
            "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/TEST_Ship_Patched.xml",
            PATCHED,
        ),
        (
            "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/Parts_Only.xml",
            PARTS_ONLY,
        ),
        (
            "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/Full.xml",
            FULL,
        ),
        (
            "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/TEST_Ship_Fallback.xml",
            FALLBACK,
        ),
    ])
}

pub(crate) fn database(definition: &str, modification: &str) -> DataForgeBuilder {
    let ship = TestEntity::new(0x1, "TEST_Ship", "entities/test_ship.xml")
        .vehicle(definition, modification)
        .port(TestPort::new("hardpoint_gun"))
        .port(TestPort::new("hardpoint_seat"))
        .port(TestPort::new("hardpoint_unnamed"))
        .landing_gear("landing_gear_front", "Objects/gear_front.cgf")
        .landing_gear("landing_gear_back", "Objects/gear_back.cgf");
    entity_schema(DataForgeBuilder::new()).record(ship.record())
}

pub(crate) fn resolve(
    definition: &str,
    modification: &str,
    files: Option<&Files>,
) -> Result<EntityNode, Box<dyn Error>> {
    let bytes = database(definition, modification).build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    if let Some(files) = files {
        resolver = resolver.with_files(files);
    }
    let ship = resolver.entities().find("TEST_Ship")?;
    Ok(resolver.resolve(&ship)?)
}

pub(crate) fn visibility(node: &EntityNode) -> Vec<(String, PortVisibility)> {
    node.ports()
        .iter()
        .map(|port| (port.name().to_owned(), port.visibility()))
        .collect()
}

pub(crate) fn part_names<'a>(parts: impl Iterator<Item = &'a VehiclePart>) -> Vec<&'a str> {
    parts.map(VehiclePart::name).collect()
}
