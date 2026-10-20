use std::collections::{BTreeMap, HashMap};

use crate::GfxError;
use crate::assets::{Assets, OtherDefinition, TagCount, TagFailure, UnknownTag};
use crate::bitmap::{read_external, read_jpeg, read_lossless, read_sub_image};
use crate::font::{FontFormat, read_compacted_font, read_font1, read_font2};
use crate::font_info::{FontInfo, read_font_info, read_font_name, read_font4};
use crate::morph::read_morph_shape;
use crate::place::read_place;
use crate::shape::{RecordBudget, read_shape};
use crate::tag::{Tag, TagStream, tag_name};
use crate::text::{GlyphBudget, read_edit_text, read_static_text};
use crate::timeline::{DisplayList, Sprite, Timeline, placed_id, read_exporter_info, read_exports};

/// The definition tags that a placement may name but that are not read: buttons and video.
const OTHER_DEFINITIONS: [u16; 3] = [7, 34, 60];

/// Applies a `PlaceObject` or `RemoveObject` tag of the first frame to its display list.
fn read_display_tag(tag: &Tag<'_>, display_list: &mut DisplayList) -> Result<(), GfxError> {
    match tag.code {
        4 | 26 | 70 => display_list.place(read_place(tag)?),
        5 | 28 => display_list.remove(tag)?,
        _ => {}
    }
    Ok(())
}

/// The deepest nesting of sprites read; a sprite deeper than this is reported as a failure.
pub const MAX_SPRITE_DEPTH: u32 = 16;

/// Reads every tag of a movie and its sprites into [`Assets`].
pub(crate) struct Walker<'a> {
    assets: Assets<'a>,
    counts: BTreeMap<u16, u32>,
    budget: RecordBudget,
    text_budget: GlyphBudget,
    font_infos: Vec<FontInfo>,
    font_names: Vec<(u16, String)>,
}

impl<'a> Walker<'a> {
    /// Reads the tags of the main timeline in `data`, which starts at `base` in the movie.
    pub(crate) fn read(data: &'a [u8], base: usize) -> Assets<'a> {
        let mut walker = Self {
            assets: Assets::default(),
            counts: BTreeMap::new(),
            budget: RecordBudget::MOVIE,
            text_budget: GlyphBudget::MOVIE,
            font_infos: Vec::new(),
            font_names: Vec::new(),
        };
        walker.assets.timeline = walker.walk(data, base, 0);
        walker.finish()
    }

    fn finish(mut self) -> Assets<'a> {
        let mut fonts: HashMap<u16, usize> = HashMap::with_capacity(self.assets.fonts.len());
        for (index, font) in self.assets.fonts.iter().enumerate() {
            fonts.entry(font.id).or_insert(index);
        }
        for info in self.font_infos {
            if let Some(font) = fonts
                .get(&info.id)
                .and_then(|&index| self.assets.fonts.get_mut(index))
            {
                info.apply(font);
            }
        }
        for (id, name) in self.font_names {
            if let Some(font) = fonts
                .get(&id)
                .and_then(|&index| self.assets.fonts.get_mut(index))
            {
                font.full_name = Some(name);
            }
        }
        self.assets.tag_counts = self
            .counts
            .into_iter()
            .map(|(code, count)| TagCount { code, count })
            .collect();
        self.assets
    }

    fn walk(&mut self, data: &'a [u8], base: usize, depth: u32) -> Timeline {
        let mut timeline = Timeline::default();
        let mut display_list = DisplayList::default();
        let mut stream = TagStream::new(data, base);
        while let Some(next) = stream.next_tag() {
            let tag = match next {
                Ok(tag) => tag,
                Err(error) => {
                    let (code, offset) = match &error {
                        GfxError::TagLength { code, offset, .. } => (*code, *offset),
                        _ => (0, base + data.len()),
                    };
                    self.fail(code, offset, error);
                    break;
                }
            };
            *self.counts.entry(tag.code).or_default() += 1;
            timeline.tag_count += 1;
            if let Err(error) = self.read_tag(&tag, &mut timeline, depth) {
                self.fail(tag.code, tag.offset, error);
            } else if timeline.frames == 0
                && let Err(error) = read_display_tag(&tag, &mut display_list)
            {
                self.fail(tag.code, tag.offset, error);
            }
        }
        timeline.finish(display_list);
        timeline
    }

    fn fail(&mut self, code: u16, offset: usize, error: GfxError) {
        self.assets.failures.push(TagFailure {
            code,
            offset,
            error,
        });
    }

    #[expect(clippy::too_many_lines, reason = "one arm per tag code")]
    fn read_tag(
        &mut self,
        tag: &Tag<'a>,
        timeline: &mut Timeline,
        depth: u32,
    ) -> Result<(), GfxError> {
        let assets = &mut self.assets;
        match tag.code {
            1 => timeline.frames += 1,
            4 | 26 | 70 => timeline.placed.extend(placed_id(tag)?),
            43 => timeline.labels.push(tag.reader().c_string()?.into_owned()),
            2 => assets.shapes.push(read_shape(tag, 1, &mut self.budget)?),
            22 => assets.shapes.push(read_shape(tag, 2, &mut self.budget)?),
            32 => assets.shapes.push(read_shape(tag, 3, &mut self.budget)?),
            83 => assets.shapes.push(read_shape(tag, 4, &mut self.budget)?),
            46 => assets
                .morph_shapes
                .push(read_morph_shape(tag, 1, &mut self.budget)?),
            84 => assets
                .morph_shapes
                .push(read_morph_shape(tag, 2, &mut self.budget)?),
            6 => assets.bitmaps.push(read_jpeg(tag, 1)?),
            21 => assets.bitmaps.push(read_jpeg(tag, 2)?),
            35 => assets.bitmaps.push(read_jpeg(tag, 3)?),
            90 => assets.bitmaps.push(read_jpeg(tag, 4)?),
            8 => {
                assets.jpeg_tables.get_or_insert(tag.body);
            }
            20 => assets.bitmaps.push(read_lossless(tag, false)?),
            36 => assets.bitmaps.push(read_lossless(tag, true)?),
            1001 => assets.bitmaps.push(read_external(tag, 1)?),
            1009 => assets.bitmaps.push(read_external(tag, 2)?),
            1008 => assets.bitmaps.push(read_sub_image(tag)?),
            10 => assets.fonts.push(read_font1(tag, &mut self.budget)?),
            48 => assets
                .fonts
                .push(read_font2(tag, FontFormat::DefineFont2, &mut self.budget)?),
            75 => assets
                .fonts
                .push(read_font2(tag, FontFormat::DefineFont3, &mut self.budget)?),
            1005 => assets.fonts.push(read_compacted_font(tag)?),
            91 => assets.fonts.push(read_font4(tag)?),
            11 => assets
                .static_texts
                .push(read_static_text(tag, false, &mut self.text_budget)?),
            33 => assets
                .static_texts
                .push(read_static_text(tag, true, &mut self.text_budget)?),
            37 => assets.edit_texts.push(read_edit_text(tag)?),
            13 | 62 => self.font_infos.push(read_font_info(tag)?),
            88 => self.font_names.push(read_font_name(tag)?),
            56 => assets.exports.extend(read_exports(tag)?),
            1000 => assets.exporter_info = Some(read_exporter_info(tag)?),
            39 => self.read_sprite(tag, depth)?,
            9 => {
                assets.background.get_or_insert(tag.reader().rgb()?);
            }
            code if OTHER_DEFINITIONS.contains(&code) => {
                assets.other_definitions.push(OtherDefinition {
                    id: tag.reader().u16()?,
                    code,
                });
            }
            code if tag_name(code).is_none() => assets.unknown_tags.push(UnknownTag {
                code,
                offset: tag.offset,
                body: tag.body,
            }),
            _ => {}
        }
        Ok(())
    }

    fn read_sprite(&mut self, tag: &Tag<'a>, depth: u32) -> Result<(), GfxError> {
        if depth >= MAX_SPRITE_DEPTH {
            return Err(GfxError::SpriteDepth {
                limit: MAX_SPRITE_DEPTH,
            });
        }
        let mut reader = tag.reader();
        let id = reader.u16()?;
        let frame_count = reader.u16()?;
        let base = reader.offset();
        let index = self.assets.sprites.len();
        self.assets.sprites.push(Sprite {
            id,
            frame_count,
            timeline: Timeline::default(),
        });
        let timeline = self.walk(reader.rest(), base, depth + 1);
        if let Some(sprite) = self.assets.sprites.get_mut(index) {
            sprite.timeline = timeline;
        }
        Ok(())
    }
}
