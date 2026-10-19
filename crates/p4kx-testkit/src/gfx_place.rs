use crate::gfx::{BitWriter, signed_bits};

/// Writes the body of a `PlaceObject2` or `PlaceObject3` tag.
#[derive(Debug, Clone, Default)]
pub struct TestPlace {
    depth: u16,
    id: Option<u16>,
    moves: bool,
    matrix: Option<([f32; 4], (i32, i32))>,
    color: Option<([f32; 4], [i16; 4])>,
    ratio: Option<u16>,
    name: Option<String>,
    clip_depth: Option<u16>,
    blend_mode: Option<u8>,
    blur: bool,
    visible: Option<bool>,
}

impl TestPlace {
    /// Places the definition `id` at `depth`.
    #[must_use]
    pub fn new(depth: u16, id: u16) -> Self {
        Self {
            depth,
            id: Some(id),
            ..Self::default()
        }
    }

    /// Changes the placement at `depth`.
    #[must_use]
    pub fn modify(depth: u16) -> Self {
        Self {
            depth,
            moves: true,
            ..Self::default()
        }
    }

    /// Sets the matrix: scale x, scale y, rotate-skew 0, rotate-skew 1, and the translation in
    /// twips.
    #[must_use]
    pub const fn matrix(mut self, factors: [f32; 4], translate: (i32, i32)) -> Self {
        self.matrix = Some((factors, translate));
        self
    }

    /// Sets a matrix that only moves by (x, y) twips.
    #[must_use]
    pub const fn at(self, x: i32, y: i32) -> Self {
        self.matrix([1.0, 1.0, 0.0, 0.0], (x, y))
    }

    /// Sets the colour transform: RGBA factors and RGBA addends.
    #[must_use]
    pub const fn color(mut self, multiply: [f32; 4], add: [i16; 4]) -> Self {
        self.color = Some((multiply, add));
        self
    }

    /// Sets the morph ratio.
    #[must_use]
    pub const fn ratio(mut self, ratio: u16) -> Self {
        self.ratio = Some(ratio);
        self
    }

    /// Sets the instance name.
    #[must_use]
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }

    /// Makes the placement a mask clipping the depths up to `depth`.
    #[must_use]
    pub const fn clip(mut self, depth: u16) -> Self {
        self.clip_depth = Some(depth);
        self
    }

    /// Sets the blend mode; written by [`TestPlace::body3`] only.
    #[must_use]
    pub const fn blend_mode(mut self, mode: u8) -> Self {
        self.blend_mode = Some(mode);
        self
    }

    /// Adds a blur filter; written by [`TestPlace::body3`] only.
    #[must_use]
    pub const fn blur(mut self) -> Self {
        self.blur = true;
        self
    }

    /// Sets whether the placement is shown; written by [`TestPlace::body3`] only.
    #[must_use]
    pub const fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    /// Returns the body of a `PlaceObject2` tag (code 26).
    #[must_use]
    pub fn body2(&self) -> Vec<u8> {
        self.body(false)
    }

    /// Returns the body of a `PlaceObject3` tag (code 70).
    #[must_use]
    pub fn body3(&self) -> Vec<u8> {
        self.body(true)
    }

    fn body(&self, version3: bool) -> Vec<u8> {
        let flags = u8::from(self.moves)
            | u8::from(self.id.is_some()) << 1
            | u8::from(self.matrix.is_some()) << 2
            | u8::from(self.color.is_some()) << 3
            | u8::from(self.ratio.is_some()) << 4
            | u8::from(self.name.is_some()) << 5
            | u8::from(self.clip_depth.is_some()) << 6;
        let mut bits = BitWriter::default();
        bits.bytes(&[flags]);
        if version3 {
            let flags3 = u8::from(self.blur)
                | u8::from(self.blend_mode.is_some()) << 1
                | u8::from(self.visible.is_some()) << 5;
            bits.bytes(&[flags3]);
        }
        bits.bytes(&self.depth.to_le_bytes());
        if let Some(id) = self.id {
            bits.bytes(&id.to_le_bytes());
        }
        if let Some((factors, translate)) = self.matrix {
            write_matrix(&mut bits, factors, translate);
        }
        if let Some((multiply, add)) = self.color {
            write_color_transform(&mut bits, multiply, add);
        }
        if let Some(ratio) = self.ratio {
            bits.bytes(&ratio.to_le_bytes());
        }
        if let Some(name) = &self.name {
            bits.bytes(name.as_bytes());
            bits.bytes(&[0]);
        }
        if let Some(depth) = self.clip_depth {
            bits.bytes(&depth.to_le_bytes());
        }
        if version3 {
            if self.blur {
                bits.bytes(&[1, 1]);
                bits.bytes(&[0, 0, 4, 0, 0, 0, 4, 0, 0x20]);
            }
            if let Some(mode) = self.blend_mode {
                bits.bytes(&[mode]);
            }
            if let Some(visible) = self.visible {
                bits.bytes(&[u8::from(visible)]);
            }
        }
        bits.finish()
    }
}

fn fixed16(value: f32) -> i32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "test matrices hold small factors"
    )]
    let fixed = (value * 65_536.0).round() as i32;
    fixed
}

/// Writes a MATRIX of scale x, scale y, rotate-skew 0, rotate-skew 1 and a translation in twips.
pub(crate) fn write_matrix(
    bits: &mut BitWriter,
    [scale_x, scale_y, skew0, skew1]: [f32; 4],
    (x, y): (i32, i32),
) {
    bits.align();
    for (pair, identity) in [([scale_x, scale_y], 1.0), ([skew0, skew1], 0.0)] {
        let present = pair
            .iter()
            .any(|&value| (value - identity).abs() > f32::EPSILON);
        bits.flag(present);
        if present {
            let values = pair.map(fixed16);
            let size = signed_bits(&values);
            bits.ub(5, size);
            for value in values {
                bits.sb(size, value);
            }
        }
    }
    let size = signed_bits(&[x, y]);
    bits.ub(5, size);
    bits.sb(size, x);
    bits.sb(size, y);
    bits.align();
}

fn write_color_transform(bits: &mut BitWriter, multiply: [f32; 4], add: [i16; 4]) {
    #[expect(clippy::cast_possible_truncation, reason = "test factors are small")]
    let factors = multiply.map(|factor| (factor * 256.0).round() as i32);
    let addends = add.map(i32::from);
    let mut all = factors.to_vec();
    all.extend_from_slice(&addends);
    let size = signed_bits(&all);
    bits.align();
    bits.flag(true);
    bits.flag(true);
    bits.ub(4, size);
    for value in all {
        bits.sb(size, value);
    }
    bits.align();
}

/// Returns the body of a `RemoveObject2` tag (code 28) emptying `depth`.
#[must_use]
pub fn remove_object2_body(depth: u16) -> Vec<u8> {
    depth.to_le_bytes().to_vec()
}

/// Returns the body of a `SetBackgroundColor` tag (code 9).
#[must_use]
pub fn background_body([red, green, blue]: [u8; 3]) -> Vec<u8> {
    vec![red, green, blue]
}
