//! The Archive with the model, the material library and the split textures, and the command.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, CheckedGlb, DecodedImage, SplitDds, TestEntry, TestModel, TestMtl,
    TestMtlMaterial, TestTextureFormat,
};

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) const MODEL_ENTRY: &str = "Data\\Objects\\Test\\cube.cgf";
pub(crate) const GEOMETRY_ENTRY: &str = "Data\\Objects\\Test\\cube.cgfm";
pub(crate) const MATERIAL_ENTRY: &str = "Data\\Objects\\Test\\cube.mtl";
pub(crate) const DIFFUSE_ENTRY: &str = "Data\\Objects\\Test\\plate_diff.dds";
pub(crate) const NORMAL_ENTRY: &str = "Data\\Objects\\Test\\plate_ddna.dds";
pub(crate) const SPECULAR_ENTRY: &str = "Data\\Objects\\Test\\plate_spec.dds";
pub(crate) const SIDE: u32 = 8;

pub(crate) const BC5: TestTextureFormat = TestTextureFormat::Dx10Block {
    dxgi_format: 83,
    block_bytes: 16,
};
pub(crate) const BC4: TestTextureFormat = TestTextureFormat::Dx10Block {
    dxgi_format: 80,
    block_bytes: 8,
};

pub(crate) fn diffuse() -> SplitDds {
    SplitDds::new(TestTextureFormat::LegacyDxt1, SIDE, SIDE)
        .mips(4)
        .split(1)
}

pub(crate) fn normal() -> SplitDds {
    SplitDds::new(BC5, SIDE, SIDE).mips(4).split(1).alpha(BC4)
}

pub(crate) fn specular() -> SplitDds {
    SplitDds::new(TestTextureFormat::LegacyDxt1, SIDE, SIDE).mips(4)
}

pub(crate) fn plate(name: &str) -> TestMtlMaterial {
    TestMtlMaterial::new(name)
        .diffuse([1.0; 3])
        .specular([1.0; 3])
        .texture("TexSlot1", "Objects/Test/plate_diff.tif")
        .texture("TexSlot2", "Objects/Test/plate_ddna.tif")
        .texture("TexSlot4", "Objects/Test/plate_spec.tif")
}

pub(crate) fn mtl() -> TestMtl {
    TestMtl::multi(&[
        TestMtlMaterial::new("unused"),
        plate("caps"),
        plate("sides"),
    ])
}

pub(crate) fn archive(
    folder: &Path,
    mtl: &TestMtl,
    textures: &[(&str, SplitDds)],
) -> Result<PathBuf, Box<dyn Error>> {
    let model = TestModel::cube();
    let path = folder.join("Data.p4k");
    let mut builder = ArchiveBuilder::new()
        .entry(TestEntry::zstd(MODEL_ENTRY, model.cgf()))
        .entry(TestEntry::zstd(GEOMETRY_ENTRY, model.cgfm()))
        .entry(TestEntry::zstd(MATERIAL_ENTRY, mtl.cryxml()));
    for (header, dds) in textures {
        for entry in dds.entries(header) {
            builder = builder.entry(entry);
        }
    }
    builder.write_to(&path)?;
    Ok(path)
}

pub(crate) fn all_textures() -> [(&'static str, SplitDds); 3] {
    [
        (DIFFUSE_ENTRY, diffuse()),
        (NORMAL_ENTRY, normal()),
        (SPECULAR_ENTRY, specular()),
    ]
}

pub(crate) fn glb(
    archive: &Path,
    out: &Path,
    extra: &[&str],
) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?
        .args(["model", "glb"])
        .arg(archive)
        .arg(MODEL_ENTRY)
        .arg("--out")
        .arg(out)
        .args(extra)
        .output()?)
}

pub(crate) fn images(glb: &CheckedGlb) -> Result<Vec<DecodedImage>, Box<dyn Error>> {
    glb.document
        .images()
        .map(|image| Ok(glb.decode(&image)?))
        .collect()
}
