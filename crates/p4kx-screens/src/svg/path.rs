// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::{PathCommand, ScenePoint};

use super::SvgError;
use super::arc::{Arc, ArcShape};
use super::cubic::Cubic;
use super::number::{MAX_MAGNITUDE, Scanner};

type Pt = (f32, f32);

/// Builds the commands of one path, counting them against a budget.
pub(super) struct Builder<'b> {
    pub(super) commands: Vec<PathCommand>,
    budget: &'b mut usize,
    at: Pt,
    start: Pt,
}

const fn point((x, y): Pt) -> ScenePoint {
    ScenePoint { x, y }
}

fn sane((x, y): Pt) -> bool {
    x.is_finite() && y.is_finite() && x.abs() <= MAX_MAGNITUDE && y.abs() <= MAX_MAGNITUDE
}

impl<'b> Builder<'b> {
    pub(super) const fn new(budget: &'b mut usize) -> Self {
        Self {
            commands: Vec::new(),
            budget,
            at: (0.0, 0.0),
            start: (0.0, 0.0),
        }
    }

    fn push(&mut self, command: PathCommand) -> Result<(), SvgError> {
        *self.budget = self
            .budget
            .checked_sub(1)
            .ok_or(SvgError::TooManyCommands)?;
        self.commands.push(command);
        Ok(())
    }

    pub(super) fn move_to(&mut self, to: Pt) -> Result<(), SvgError> {
        self.at = to;
        self.start = to;
        self.push(PathCommand::MoveTo(point(to)))
    }

    pub(super) fn line_to(&mut self, to: Pt) -> Result<(), SvgError> {
        self.at = to;
        self.push(PathCommand::LineTo(point(to)))
    }

    fn quad_to(&mut self, control: Pt, to: Pt) -> Result<(), SvgError> {
        self.at = to;
        self.push(PathCommand::QuadTo(point(control), point(to)))
    }

    pub(super) fn close(&mut self) -> Result<(), SvgError> {
        self.at = self.start;
        self.push(PathCommand::Close)
    }

    fn cubic_to(&mut self, c1: Pt, c2: Pt, to: Pt) -> Result<(), SvgError> {
        let cubic = Cubic::new(self.at, c1, c2, to);
        for (control, end) in cubic.quads() {
            self.quad_to(control, end)?;
        }
        Ok(())
    }

    /// Draws an elliptical arc to `to`, as SVG defines its parameters.
    pub(super) fn arc_to(&mut self, shape: &ArcShape, to: Pt) -> Result<(), SvgError> {
        let from = self.at;
        if from == to {
            return Ok(());
        }
        let Some(arc) = Arc::of(from, to, shape) else {
            return self.line_to(to);
        };
        let count = arc.pieces();
        for (piece, (c1, c2, end)) in arc.cubics().enumerate() {
            self.cubic_to(c1, c2, if piece + 1 == count { to } else { end })?;
        }
        Ok(())
    }
}

fn pair(scanner: &mut Scanner<'_>, origin: Pt) -> Result<Pt, SvgError> {
    let x = scanner.number().ok_or(SvgError::Path)?;
    let y = scanner.number().ok_or(SvgError::Path)?;
    Ok((origin.0 + x, origin.1 + y))
}

fn single(scanner: &mut Scanner<'_>) -> Result<f32, SvgError> {
    scanner.number().ok_or(SvgError::Path)
}

fn mirrored(from: Pt, control: Option<Pt>) -> Pt {
    control.map_or(from, |c| {
        (2.0f32.mul_add(from.0, -c.0), 2.0f32.mul_add(from.1, -c.1))
    })
}

/// The reflection points the smooth commands `S` and `T` continue from.
#[derive(Default)]
struct Controls {
    cubic: Option<Pt>,
    quad: Option<Pt>,
}

fn arc(scanner: &mut Scanner<'_>, origin: Pt) -> Result<(ArcShape, Pt), SvgError> {
    let shape = ArcShape {
        radii: pair(scanner, (0.0, 0.0))?,
        turn_degrees: single(scanner)?,
        large: scanner.flag().ok_or(SvgError::Path)?,
        sweep: scanner.flag().ok_or(SvgError::Path)?,
    };
    Ok((shape, pair(scanner, origin)?))
}

fn curve(
    out: &mut Builder<'_>,
    scanner: &mut Scanner<'_>,
    letter: char,
    origin: Pt,
    last: &Controls,
) -> Result<Controls, SvgError> {
    let mut next = Controls::default();
    match letter {
        'C' | 'S' => {
            let c1 = if letter == 'C' {
                pair(scanner, origin)?
            } else {
                mirrored(out.at, last.cubic)
            };
            let c2 = pair(scanner, origin)?;
            out.cubic_to(c1, c2, pair(scanner, origin)?)?;
            next.cubic = Some(c2);
        }
        'Q' | 'T' => {
            let control = if letter == 'Q' {
                pair(scanner, origin)?
            } else {
                mirrored(out.at, last.quad)
            };
            out.quad_to(control, pair(scanner, origin)?)?;
            next.quad = Some(control);
        }
        _ => {
            let (shape, to) = arc(scanner, origin)?;
            out.arc_to(&shape, to)?;
        }
    }
    Ok(next)
}

/// Parses the `d` attribute of a path into commands, counting them against `budget`.
pub(super) fn parse(data: &str, budget: &mut usize) -> Result<Vec<PathCommand>, SvgError> {
    let mut out = Builder::new(budget);
    let mut scanner = Scanner::new(data);
    let mut command = None;
    let mut last = Controls::default();
    while !scanner.at_end() {
        if let Some(letter) = scanner.letter() {
            command = Some(letter);
        } else if !scanner.starts_number() {
            return Err(SvgError::Path);
        }
        let Some(letter) = command else {
            return Err(SvgError::Path);
        };
        let relative = letter.is_ascii_lowercase();
        let origin = if relative { out.at } else { (0.0, 0.0) };
        let upper = letter.to_ascii_uppercase();
        last = match upper {
            'Z' => {
                out.close()?;
                command = None;
                Controls::default()
            }
            'M' => {
                out.move_to(pair(&mut scanner, origin)?)?;
                command = Some(if relative { 'l' } else { 'L' });
                Controls::default()
            }
            'L' => {
                out.line_to(pair(&mut scanner, origin)?)?;
                Controls::default()
            }
            'H' => {
                out.line_to((origin.0 + single(&mut scanner)?, out.at.1))?;
                Controls::default()
            }
            'V' => {
                out.line_to((out.at.0, origin.1 + single(&mut scanner)?))?;
                Controls::default()
            }
            'C' | 'S' | 'Q' | 'T' | 'A' => curve(&mut out, &mut scanner, upper, origin, &last)?,
            _ => return Err(SvgError::Path),
        };
        if !sane(out.at) {
            return Err(SvgError::Path);
        }
    }
    Ok(out.commands)
}
