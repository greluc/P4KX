// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// The cascade modifiers of textures and vector graphics.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PictureModifiers {
    pub(crate) image_path: Option<String>,
    pub(crate) svg_path: Option<String>,
    pub(crate) overlay: Option<bool>,
    pub(crate) image_flip: [Option<bool>; 2],
    pub(crate) svg_flip: [Option<bool>; 2],
    pub(crate) nine_slice: Option<bool>,
}

impl PictureModifiers {
    pub(crate) fn string(&mut self, field: &str, value: Option<&str>) {
        let value = value.map(str::to_owned);
        match field {
            "ImagePath" => self.image_path = value,
            "SvgPath" => self.svg_path = value,
            _ => {}
        }
    }

    pub(crate) fn boolean(&mut self, field: &str, value: Option<bool>) {
        match field {
            "EnableColorOverlay" => self.overlay = value,
            "EnableNineSliceRect" => self.nine_slice = value,
            "ImageFlipHorizontal" => self.image_flip[0] = value,
            "ImageFlipVertical" => self.image_flip[1] = value,
            "SvgFlipHorizontal" => self.svg_flip[0] = value,
            "SvgFlipVertical" => self.svg_flip[1] = value,
            _ => {}
        }
    }

    pub(crate) fn merge(&mut self, from: &Self) {
        fn set<T: Clone>(slot: &mut Option<T>, value: Option<&T>) {
            if let Some(value) = value {
                *slot = Some(value.clone());
            }
        }
        set(&mut self.image_path, from.image_path.as_ref());
        set(&mut self.svg_path, from.svg_path.as_ref());
        set(&mut self.overlay, from.overlay.as_ref());
        set(&mut self.nine_slice, from.nine_slice.as_ref());
        for (slot, value) in self.image_flip.iter_mut().zip(&from.image_flip) {
            set(slot, value.as_ref());
        }
        for (slot, value) in self.svg_flip.iter_mut().zip(&from.svg_flip) {
            set(slot, value.as_ref());
        }
    }
}
