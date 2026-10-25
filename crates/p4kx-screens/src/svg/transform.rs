// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::Affine;

use super::number::Scanner;

const MAX_TRANSFORMS: usize = 32;

pub(super) const fn matrix(a: f32, b: f32, c: f32, d: f32, tx: f32, ty: f32) -> Affine {
    Affine { a, b, c, d, tx, ty }
}

/// Parses the value of a `transform` attribute; `None` when it is malformed or not finite.
pub(super) fn parse(text: &str) -> Option<Affine> {
    let mut total = Affine::IDENTITY;
    let mut scanner = Scanner::new(text);
    for _ in 0..MAX_TRANSFORMS {
        scanner.skip_separator();
        if scanner.at_end() {
            return total.is_finite().then_some(total);
        }
        let mut name = String::new();
        while let Some(letter) = scanner.letter() {
            name.push(letter);
        }
        if !scanner.eat('(') {
            return None;
        }
        let mut args = Vec::new();
        while !scanner.eat(')') {
            if args.len() >= 6 {
                return None;
            }
            args.push(scanner.number()?);
        }
        total = total.then(&one(&name, &args)?);
    }
    None
}

fn one(name: &str, args: &[f32]) -> Option<Affine> {
    let at = |index: usize| args.get(index).copied();
    Some(match (name, args.len()) {
        ("matrix", 6) => matrix(at(0)?, at(1)?, at(2)?, at(3)?, at(4)?, at(5)?),
        ("translate", 1 | 2) => matrix(1.0, 0.0, 0.0, 1.0, at(0)?, at(1).unwrap_or(0.0)),
        ("scale", 1 | 2) => {
            let x = at(0)?;
            matrix(x, 0.0, 0.0, at(1).unwrap_or(x), 0.0, 0.0)
        }
        ("rotate", 1 | 3) => {
            let (sin, cos) = at(0)?.to_radians().sin_cos();
            let turn = matrix(cos, sin, -sin, cos, 0.0, 0.0);
            match (at(1), at(2)) {
                (Some(x), Some(y)) => matrix(1.0, 0.0, 0.0, 1.0, x, y)
                    .then(&turn)
                    .then(&matrix(1.0, 0.0, 0.0, 1.0, -x, -y)),
                _ => turn,
            }
        }
        ("skewX", 1) => matrix(1.0, 0.0, at(0)?.to_radians().tan(), 1.0, 0.0, 0.0),
        ("skewY", 1) => matrix(1.0, at(0)?.to_radians().tan(), 0.0, 1.0, 0.0, 0.0),
        _ => return None,
    })
}
