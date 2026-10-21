use p4kx_extraction::UiMovie;
use p4kx_extraction::gfx::{
    Assets, Bitmap, BitmapSource, Compression, EmbeddedFormat, Font, FontFormat, MorphShape, Rect,
    Shape, ShapeRecord, Sprite,
};
use serde::Serialize;

/// The JSON shape of `p4kx ui assets`.
#[derive(Serialize)]
pub(crate) struct AssetsJson<'a> {
    pub(crate) movie: &'a str,
    pub(crate) signature: &'static str,
    pub(crate) version: u8,
    pub(crate) compression: &'static str,
    pub(crate) length: usize,
    pub(crate) stage: Rect,
    pub(crate) frame_rate: f32,
    pub(crate) frame_count: u16,
    pub(crate) frames: u32,
    pub(crate) exporter: Option<ExporterJson<'a>>,
    pub(crate) tags: Vec<TagJson>,
    pub(crate) unread_tags: Vec<TagJson>,
    pub(crate) unknown_tags: Vec<UnknownTagJson>,
    pub(crate) shapes: Vec<ShapeJson<'a>>,
    pub(crate) morph_shapes: Vec<ShapeJson<'a>>,
    pub(crate) bitmaps: Vec<BitmapJson<'a>>,
    pub(crate) fonts: Vec<FontJson<'a>>,
    pub(crate) sprites: Vec<SpriteJson<'a>>,
    pub(crate) imports: Vec<ImportJson<'a>>,
    pub(crate) symbol_classes: Vec<SymbolClassJson<'a>>,
    pub(crate) scaling_grids: Vec<ScalingGridJson>,
    pub(crate) failures: Vec<FailureJson>,
}

#[derive(Serialize)]
pub(crate) struct ImportJson<'a> {
    pub(crate) url: &'a str,
    pub(crate) assets: Vec<ImportedAssetJson<'a>>,
}

#[derive(Serialize)]
pub(crate) struct ImportedAssetJson<'a> {
    pub(crate) id: u16,
    pub(crate) name: &'a str,
}

#[derive(Serialize)]
pub(crate) struct SymbolClassJson<'a> {
    pub(crate) id: u16,
    pub(crate) name: &'a str,
}

#[derive(Serialize)]
pub(crate) struct ScalingGridJson {
    pub(crate) id: u16,
    pub(crate) splitter: Rect,
}

#[derive(Serialize)]
pub(crate) struct FrameLabelJson<'a> {
    pub(crate) frame: u32,
    pub(crate) name: &'a str,
}

#[derive(Serialize)]
pub(crate) struct ExporterJson<'a> {
    pub(crate) version: u16,
    pub(crate) flags: u32,
    pub(crate) bitmap_format: u16,
    pub(crate) prefix: &'a str,
    pub(crate) swf_name: &'a str,
}

#[derive(Serialize)]
pub(crate) struct TagJson {
    pub(crate) code: u16,
    pub(crate) name: Option<&'static str>,
    pub(crate) count: u32,
}

#[derive(Serialize)]
pub(crate) struct UnknownTagJson {
    pub(crate) code: u16,
    pub(crate) offset: usize,
    pub(crate) length: usize,
}

#[derive(Serialize)]
pub(crate) struct ShapeJson<'a> {
    pub(crate) id: u16,
    pub(crate) version: u8,
    pub(crate) bounds: Rect,
    pub(crate) fill_styles: usize,
    pub(crate) line_styles: usize,
    pub(crate) edges: usize,
    pub(crate) export_name: Option<&'a str>,
}

#[derive(Serialize)]
pub(crate) struct BitmapJson<'a> {
    pub(crate) id: u16,
    pub(crate) kind: &'static str,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) format: Option<String>,
    pub(crate) file: Option<&'a str>,
    pub(crate) image_id: Option<u16>,
    pub(crate) export_name: Option<&'a str>,
}

#[derive(Serialize)]
pub(crate) struct FontJson<'a> {
    pub(crate) id: u16,
    pub(crate) format: &'static str,
    pub(crate) name: &'a str,
    pub(crate) full_name: Option<&'a str>,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) glyphs: u32,
    pub(crate) outlines: bool,
    pub(crate) export_name: Option<&'a str>,
}

#[derive(Serialize)]
pub(crate) struct SpriteJson<'a> {
    pub(crate) id: u16,
    pub(crate) frame_count: u16,
    pub(crate) frames: u32,
    pub(crate) labels: Vec<&'a str>,
    pub(crate) frame_labels: Vec<FrameLabelJson<'a>>,
    pub(crate) placed: &'a [u16],
    pub(crate) export_name: Option<&'a str>,
}

#[derive(Serialize)]
pub(crate) struct FailureJson {
    pub(crate) code: u16,
    pub(crate) offset: usize,
    pub(crate) error: String,
}

/// Describes a UI movie and its definitions.
pub(crate) fn describe<'a>(movie: &'a UiMovie<'_>, assets: &'a Assets<'a>) -> AssetsJson<'a> {
    let header = movie.movie().header();
    AssetsJson {
        movie: movie.name(),
        signature: header.signature.as_str(),
        version: header.version,
        compression: match header.signature.compression() {
            Compression::None => "none",
            Compression::Zlib => "zlib",
            Compression::Lzma => "lzma",
        },
        length: movie.movie().len(),
        stage: header.frame_size,
        frame_rate: header.frame_rate,
        frame_count: header.frame_count,
        frames: assets.timeline.frames,
        exporter: assets.exporter_info.as_ref().map(|info| ExporterJson {
            version: info.version,
            flags: info.flags,
            bitmap_format: info.bitmap_format,
            prefix: &info.prefix,
            swf_name: &info.swf_name,
        }),
        tags: list(&assets.tag_counts, |count| TagJson {
            code: count.code,
            name: count.name(),
            count: count.count,
        }),
        unread_tags: list(&assets.unread_tags, |count| TagJson {
            code: count.code,
            name: count.name(),
            count: count.count,
        }),
        unknown_tags: list(&assets.unknown_tags, |tag| UnknownTagJson {
            code: tag.code,
            offset: tag.offset,
            length: tag.body.len(),
        }),
        shapes: list(&assets.shapes, |shape| shape_json(assets, shape)),
        morph_shapes: list(&assets.morph_shapes, |morph| morph_json(assets, morph)),
        bitmaps: list(&assets.bitmaps, |bitmap| bitmap_json(assets, bitmap)),
        fonts: list(&assets.fonts, |font| font_json(assets, font)),
        sprites: list(&assets.sprites, |sprite| sprite_json(assets, sprite)),
        imports: imports_json(assets),
        symbol_classes: list(&assets.symbol_classes, |symbol| SymbolClassJson {
            id: symbol.id,
            name: &symbol.name,
        }),
        scaling_grids: list(&assets.scaling_grids, |grid| ScalingGridJson {
            id: grid.id,
            splitter: grid.splitter,
        }),
        failures: list(&assets.failures, |failure| FailureJson {
            code: failure.code,
            offset: failure.offset,
            error: crate::ui::chain(&failure.error),
        }),
    }
}

fn imports_json<'a>(assets: &'a Assets<'_>) -> Vec<ImportJson<'a>> {
    list(&assets.imports, |import| ImportJson {
        url: &import.url,
        assets: list(&import.assets, |asset| ImportedAssetJson {
            id: asset.id,
            name: &asset.name,
        }),
    })
}

fn list<'a, T, J>(items: &'a [T], describe: impl Fn(&'a T) -> J) -> Vec<J> {
    items.iter().map(describe).collect()
}

fn edges(records: &[ShapeRecord]) -> usize {
    records
        .iter()
        .filter(|record| !matches!(record, ShapeRecord::StyleChange(_)))
        .count()
}

fn shape_json<'a>(assets: &'a Assets<'_>, shape: &Shape) -> ShapeJson<'a> {
    ShapeJson {
        id: shape.id,
        version: shape.version,
        bounds: shape.bounds,
        fill_styles: shape.fill_styles.len(),
        line_styles: shape.line_styles.len(),
        edges: edges(&shape.records),
        export_name: assets.export_name(shape.id),
    }
}

fn morph_json<'a>(assets: &'a Assets<'_>, morph: &MorphShape) -> ShapeJson<'a> {
    ShapeJson {
        id: morph.id,
        version: morph.version,
        bounds: morph.start_bounds,
        fill_styles: morph.start_fill_styles.len(),
        line_styles: morph.start_line_styles.len(),
        edges: edges(&morph.start_records),
        export_name: assets.export_name(morph.id),
    }
}

fn bitmap_json<'a>(assets: &'a Assets<'_>, bitmap: &'a Bitmap<'_>) -> BitmapJson<'a> {
    let mut json = BitmapJson {
        id: bitmap.id,
        kind: "",
        width: None,
        height: None,
        format: None,
        file: None,
        image_id: None,
        export_name: assets.export_name(bitmap.id),
    };
    match &bitmap.source {
        BitmapSource::Lossless(lossless) => {
            json.kind = "lossless";
            json.width = Some(u32::from(lossless.width));
            json.height = Some(u32::from(lossless.height));
            json.format = Some(
                match (lossless.format, lossless.has_alpha) {
                    (3, _) => "colour-mapped",
                    (4, _) => "rgb15",
                    (_, true) => "argb32",
                    (_, false) => "rgb24",
                }
                .to_owned(),
            );
        }
        BitmapSource::Jpeg(jpeg) => {
            json.kind = match jpeg.format() {
                EmbeddedFormat::Jpeg => "jpeg",
                EmbeddedFormat::Png => "png",
                EmbeddedFormat::Gif => "gif",
            };
            json.format = Some(
                if jpeg.alpha.is_some() {
                    "with alpha"
                } else {
                    "opaque"
                }
                .to_owned(),
            );
        }
        BitmapSource::External(image) => {
            json.kind = "external";
            json.width = Some(u32::from(image.target_width));
            json.height = Some(u32::from(image.target_height));
            json.format = Some(
                image
                    .format
                    .name()
                    .map_or_else(|| format!("format {}", image.format.0), str::to_owned),
            );
            json.file = Some(&image.file_name);
        }
        BitmapSource::SubImage(sub) => {
            json.kind = "sub_image";
            json.width = Some(u32::from(sub.x2.saturating_sub(sub.x1)));
            json.height = Some(u32::from(sub.y2.saturating_sub(sub.y1)));
            json.image_id = Some(sub.image_id);
        }
    }
    json
}

fn font_json<'a>(assets: &'a Assets<'_>, font: &'a Font) -> FontJson<'a> {
    FontJson {
        id: font.id,
        format: match font.format {
            FontFormat::DefineFont => "DefineFont",
            FontFormat::DefineFont2 => "DefineFont2",
            FontFormat::DefineFont3 => "DefineFont3",
            FontFormat::Compacted => "DefineCompactedFont",
            FontFormat::DefineFont4 => "DefineFont4",
        },
        name: &font.name,
        full_name: font.full_name.as_deref(),
        bold: font.bold,
        italic: font.italic,
        glyphs: font.glyph_count,
        outlines: font.glyphs.iter().any(|glyph| !glyph.records.is_empty()),
        export_name: assets.export_name(font.id),
    }
}

fn sprite_json<'a>(assets: &'a Assets<'_>, sprite: &'a Sprite) -> SpriteJson<'a> {
    SpriteJson {
        id: sprite.id,
        frame_count: sprite.frame_count,
        frames: sprite.timeline.frames,
        labels: sprite
            .timeline
            .labels
            .iter()
            .map(|label| label.name.as_str())
            .collect(),
        frame_labels: list(&sprite.timeline.labels, |label| FrameLabelJson {
            frame: label.frame,
            name: &label.name,
        }),
        placed: &sprite.timeline.placed,
        export_name: assets.export_name(sprite.id),
    }
}
