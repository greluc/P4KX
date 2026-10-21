//! Export as GLB over a synthetic Archive with a cube model, driven headless.

mod glb_exporting;
mod headless;
mod viewing;

use std::path::Path;

use egui_kittest::kittest::Queryable;
use glb_exporting::{Window, finished, start};
use headless::{opened, ready, status_is_ready};
use p4kx_extraction::TextureRole;
use p4kx_gui::{Language, Model, ModelExport, ModelExportStatus, ModelExportSummary};
use p4kx_settings::ModelMaterials;
use p4kx_testkit::{TestEntry, TestModel, TestMtl, TestMtlMaterial, check_glb};
use viewing::{Site, TestResult, entries, opened_at, select};

fn summary(model: &Model) -> Result<ModelExportSummary, String> {
    match model.model_export().map(ModelExport::status) {
        Some(ModelExportStatus::Finished(summary)) => Ok(summary.clone()),
        other => Err(format!("no summary: {other:?}")),
    }
}

fn leftovers(folder: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut parts = Vec::new();
    for item in std::fs::read_dir(folder)? {
        let path = item?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "part")
        {
            parts.push(path.display().to_string());
        }
    }
    Ok(parts)
}

fn lodded() -> Vec<TestEntry> {
    let coarse = TestModel::cube().scaled(1.0);
    let model = TestModel::cube().with_lod(coarse);
    let mut files = vec![
        TestEntry::zstd("Data\\Objects\\lodded.cgf", model.cgf()),
        TestEntry::zstd("Data\\Objects\\lodded.cgfm", model.cgfm()),
    ];
    if let (Some(cgf), Some(cgfm)) = (model.lod_cgf(1), model.lod_cgfm(1)) {
        files.push(TestEntry::zstd("Data\\Objects\\lodded_lod1.cgf", cgf));
        files.push(TestEntry::zstd("Data\\Objects\\lodded_lod1.cgfm", cgfm));
    }
    files
}

#[test]
fn exporting_a_model_writes_a_valid_glb_and_reports_what_it_wrote() -> TestResult {
    let (folder, mut harness): Window = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "cube.cgf")?;
    assert!(status_is_ready(&harness), "the model is read");
    let file = folder.path().join("cube.glb");

    assert!(start(&mut harness, &file)?, "the export started");
    finished(&mut harness)?;

    let summary = summary(harness.state())?;
    let bytes = std::fs::read(&file)?;
    let checked = check_glb(&bytes)?;
    assert_eq!(summary.file(), file, "the file");
    assert_eq!(summary.lod(), 0, "the most detailed level");
    assert_eq!(
        u64::try_from(bytes.len())?,
        summary.written().bytes,
        "the length the writer counted"
    );
    assert_eq!(summary.written().triangles, 12, "the cube's triangles");
    assert_eq!(checked.document.meshes().count(), summary.written().meshes);
    assert!(leftovers(folder.path())?.is_empty(), "no temporary file");
    Ok(())
}

#[test]
fn the_choices_are_remembered_for_the_next_export() -> TestResult {
    let (folder, mut harness): Window = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "cube.cgf")?;
    harness.state_mut().begin_model_export();
    let setup = harness
        .state_mut()
        .model_export_setup_mut()
        .ok_or("no setup")?;
    setup.set_vertex_colours(false);
    setup.set_file(folder.path().join("first.glb"));
    assert!(harness.state_mut().submit_model_export(|| {}), "started");
    finished(&mut harness)?;

    harness.state_mut().begin_model_export();

    let remembered = harness.state().model_export_settings().clone();
    assert!(!remembered.vertex_colours, "vertex colours stay off");
    assert_eq!(
        remembered.folder.as_deref(),
        Some(folder.path()),
        "the folder"
    );
    let next = harness.state().model_export_setup().ok_or("no setup")?;
    assert!(!next.vertex_colours(), "a new setup starts with them");
    assert!(next.file().is_none(), "but no file");
    assert_eq!(
        next.folder(),
        Some(folder.path()),
        "the dialog starts there"
    );
    Ok(())
}

#[test]
fn every_level_of_detail_is_offered_and_the_chosen_one_is_exported() -> TestResult {
    let site = Site {
        entries: lodded(),
        installation: false,
        key: false,
    };
    let (folder, mut harness) = opened_at(Language::BritishEnglish, ready(), site, |b| b)?;
    select(&mut harness, "lodded.cgf")?;
    harness.state_mut().begin_model_export();
    let setup = harness
        .state_mut()
        .model_export_setup_mut()
        .ok_or("no setup")?;
    let levels: Vec<u32> = setup.lods().iter().map(|lod| lod.level).collect();
    assert_eq!(levels, [0, 1], "both levels");
    setup.set_lod(1);
    setup.set_lod(7);
    assert_eq!(setup.lod(), 1, "a level the model lacks is ignored");
    setup.set_file(folder.path().join("lodded"));
    assert_eq!(
        setup.file(),
        Some(folder.path().join("lodded.glb").as_path()),
        "a file name without extension gets .glb"
    );

    assert!(harness.state_mut().submit_model_export(|| {}), "started");
    finished(&mut harness)?;

    assert_eq!(summary(harness.state())?.lod(), 1, "the chosen level");
    assert_eq!(harness.state().model_export_settings().lod, 1, "remembered");
    Ok(())
}

#[test]
fn a_file_inside_an_installation_is_refused_before_anything_starts() -> TestResult {
    let site = Site {
        entries: entries(),
        installation: true,
        key: false,
    };
    let (folder, mut harness) = opened_at(Language::BritishEnglish, ready(), site, |b| b)?;
    select(&mut harness, "cube.cgf")?;
    let inside = folder
        .path()
        .join("StarCitizen")
        .join("LIVE")
        .join("cube.glb");

    let started = start(&mut harness, &inside)?;

    assert!(!started, "nothing starts");
    let setup = harness.state().model_export_setup().ok_or("no setup")?;
    assert!(setup.refusal().is_some(), "the setup says why");
    assert!(!setup.is_ready(), "and cannot start");
    assert!(!inside.exists(), "nothing was written");
    harness.run();
    assert!(
        harness
            .query_by_label_contains("never writes there")
            .is_some(),
        "the notice is shown"
    );
    Ok(())
}

#[test]
fn cancelling_leaves_no_file_behind() -> TestResult {
    let (folder, mut harness): Window = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "cube.cgf")?;
    let file = folder.path().join("cube.glb");
    assert!(start(&mut harness, &file)?, "started");

    harness.state().cancel_model_export();
    finished(&mut harness)?;

    match harness.state().model_export().map(ModelExport::status) {
        Some(ModelExportStatus::Cancelled) => assert!(!file.exists(), "no file"),
        Some(ModelExportStatus::Finished(_)) => {
            assert!(file.exists(), "the export beat the cancel");
        }
        other => return Err(format!("unexpected: {other:?}").into()),
    }
    assert!(leftovers(folder.path())?.is_empty(), "no temporary file");
    Ok(())
}

#[test]
fn stopping_the_window_waits_for_the_export_and_removes_partial_files() -> TestResult {
    let (folder, mut harness): Window = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "cube.cgf")?;
    assert!(
        start(&mut harness, &folder.path().join("cube.glb"))?,
        "started"
    );

    harness.state_mut().stop_model_export();

    assert!(
        !harness
            .state()
            .model_export()
            .is_some_and(ModelExport::is_running),
        "ended"
    );
    assert!(leftovers(folder.path())?.is_empty(), "no temporary file");
    Ok(())
}

#[test]
fn exporting_with_textures_lists_the_textures_the_archive_lacks() -> TestResult {
    let material = TestMtlMaterial::new("hull").texture("TexSlot1", "textures/missing_diff.tif");
    let mut files = entries();
    files.push(TestEntry::zstd(
        "Data\\objects\\test\\cube.mtl",
        TestMtl::single(&material).cryxml(),
    ));
    let site = Site {
        entries: files,
        installation: false,
        key: false,
    };
    let (folder, mut harness): Window = opened_at(Language::BritishEnglish, ready(), site, |b| b)?;
    select(&mut harness, "cube.cgf")?;
    harness.state_mut().begin_model_export();
    let setup = harness
        .state_mut()
        .model_export_setup_mut()
        .ok_or("no setup")?;
    setup.set_materials(ModelMaterials::Textures);
    setup.set_texture_mip(2);
    setup.set_file(folder.path().join("cube.glb"));
    assert!(harness.state_mut().submit_model_export(|| {}), "started");

    finished(&mut harness)?;

    let done = summary(harness.state())?;
    let [missing] = done.missing_textures() else {
        return Err(format!("missing: {:?}", done.missing_textures()).into());
    };
    assert_eq!(missing.role, TextureRole::Diffuse, "role");
    assert!(missing.file.contains("missing_diff"), "{missing:?}");
    let remembered = harness.state().model_export_settings();
    assert_eq!(remembered.materials, ModelMaterials::Textures, "mode");
    assert_eq!(remembered.texture_mip, 2, "mip");
    check_glb(&std::fs::read(folder.path().join("cube.glb"))?)?;
    Ok(())
}
