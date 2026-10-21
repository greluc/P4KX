//! The `CrCh` models of a real Archive as GLB, read without the Archive key: Entries that are
//! encrypted are left out. Runs with `--ignored` when `P4KX_DATA_P4K` names an Archive.

use std::error::Error;
use std::io::Read;

use p4kx_chunks::{ChunkFile, crch};
use p4kx_extraction::{Archive, Entry, GlbOptions, ModelSource, PathIndex};
use p4kx_model::ChunkUsage;
use p4kx_testkit::{check_glb, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const EXTENSIONS: [&str; 1] = ["vvg"];
const CARRACK_PROXY: &str = "anvl_carrack_no_colide.vvg";
const BOX_AT: usize = 0x6C;

fn is_crch_model(archive: &Archive, entry: &Entry) -> bool {
    let extension = entry.path().rsplit('.').next().unwrap_or("");
    if entry.is_encrypted()
        || !EXTENSIONS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(extension))
    {
        return false;
    }
    let mut signature = [0_u8; 4];
    archive
        .open_entry(entry)
        .ok()
        .and_then(|mut reader| reader.read_exact(&mut signature).ok())
        .is_some_and(|()| &signature == b"CrCh")
}

fn read_box(bytes: &[u8]) -> Option<[f32; 6]> {
    let file = ChunkFile::parse(bytes).ok()?;
    let (index, _) = file.chunks_of(crch::MESH).next()?;
    let data = file.chunk_data(index)?.get(BOX_AT..BOX_AT + 24)?;
    let mut values = [0.0_f32; 6];
    for (value, bytes) in values.iter_mut().zip(data.as_chunks::<4>().0) {
        *value = f32::from_le_bytes(*bytes);
    }
    Some(values)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_crch_models_of_the_archive_export_valid_glb_with_the_vertices_inside_the_mesh_box()
-> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let index = PathIndex::new(&archive);
    let out = tempfile::tempdir()?;
    let models: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| is_crch_model(&archive, entry))
        .collect();
    assert!(!models.is_empty(), "the Archive holds CrCh models");

    let mut found_proxy = false;
    for (position, entry) in models.into_iter().enumerate() {
        let source = ModelSource::from_index(&index, entry.path())?;
        let model = source.model();
        let glb_path = out.path().join(format!("{position}.glb"));
        source.write_glb(&GlbOptions::default(), &glb_path)?;
        let glb = check_glb(&std::fs::read(&glb_path)?)?;
        assert_eq!(glb.document.nodes().len(), model.nodes.len(), "the nodes");

        if entry.path().to_ascii_lowercase().ends_with(CARRACK_PROXY) {
            found_proxy = true;
            let lod = model.lods.first().ok_or("LOD0")?;
            let mesh = lod.meshes.first().ok_or("a mesh")?;
            assert_eq!(model.nodes.len(), 2, "root and physics proxy");
            assert_eq!((lod.vertex_count(), lod.triangle_count()), (913, 474));
            assert_eq!(mesh.node, 1, "the proxy node");
            assert_eq!(
                model.material_library.as_deref(),
                Some("objects/spaceships/ships/anvl/carrack/exterior/carrack_exterior"),
                "the MtlName chunk"
            );
            assert_eq!(model.unknown_chunks().count(), 1, "the chunk 0x300A");
            assert!(
                model
                    .chunks
                    .iter()
                    .filter(|chunk| chunk.chunk_type == crch::MESH)
                    .all(|chunk| chunk.usage == ChunkUsage::Read),
                "the mesh header is read"
            );
            let mut bytes = Vec::new();
            archive.open_entry(entry)?.read_to_end(&mut bytes)?;
            let [min_x, min_y, min_z, max_x, max_y, max_z] =
                read_box(&bytes).ok_or("the mesh box")?;
            let (min, max) = ([min_x, min_y, min_z], [max_x, max_y, max_z]);
            for position in &mesh.positions {
                for axis in 0..3 {
                    let value = position.get(axis).copied().ok_or("an axis")?;
                    let low = min.get(axis).copied().ok_or("a minimum")?;
                    let high = max.get(axis).copied().ok_or("a maximum")?;
                    assert!(
                        value >= low - 1e-3 && value <= high + 1e-3,
                        "vertex {position:?} outside the box {min:?}..{max:?}"
                    );
                }
            }
        }
    }
    assert!(found_proxy, "the Carrack physics proxy is in the Archive");
    Ok(())
}
