//! The display list of the first frame: placements, changes and removals with matrices, colour
//! transforms and the `PlaceObject3` extras.

use p4kx_gfx::{ColorTransform, Matrix, Movie, OtherDefinition, Rgba};
use p4kx_testkit::{
    TestMovie, TestPlace, TestSignature, background_body, remove_object2_body, sprite_body,
};

fn movie(bytes: &[u8]) -> Movie {
    Movie::parse(bytes.to_vec()).unwrap_or_else(|error| unreachable!("invalid movie: {error}"))
}

fn placements() -> Vec<u8> {
    TestMovie::new(TestSignature::Gfx)
        .tag(9, &background_body([10, 20, 30]))
        .tag(34, &[42, 0, 0])
        .tag(26, &TestPlace::new(1, 5).at(100, -40).body2())
        .tag(
            70,
            &TestPlace::new(2, 6)
                .matrix([2.0, 0.5, 0.25, -0.25], (2000, 0))
                .color([0.5, 1.0, 1.0, 0.25], [10, -20, 0, 0])
                .name("needle")
                .blur()
                .blend_mode(3)
                .visible(false)
                .body3(),
        )
        .tag(26, &TestPlace::new(3, 7).clip(9).ratio(100).body2())
        .tag(26, &TestPlace::modify(1).at(0, 60).body2())
        .tag(28, &remove_object2_body(3))
        .tag(1, &[])
        .tag(26, &TestPlace::new(4, 8).body2())
        .bytes()
}

#[test]
fn the_first_frame_keeps_what_its_tags_place_change_and_remove() {
    let movie = movie(&placements());
    let assets = movie.assets();
    assert!(assets.failures.is_empty(), "{:?}", assets.failures);
    assert_eq!(
        assets.background,
        Some(Rgba {
            red: 10,
            green: 20,
            blue: 30,
            alpha: 255
        }),
        "background"
    );
    assert_eq!(
        assets.other_definitions,
        [OtherDefinition { id: 42, code: 34 }],
        "a button"
    );
    let frame = &assets.timeline.first_frame;
    assert_eq!(
        frame
            .iter()
            .map(|placed| (placed.depth, placed.id))
            .collect::<Vec<_>>(),
        [(1, 5), (2, 6)],
        "depth 3 removed, depth 4 placed in the second frame"
    );
    assert_eq!(
        frame[0].matrix,
        Matrix {
            translate_x: 0,
            translate_y: 60,
            ..Matrix::IDENTITY
        },
        "moved"
    );
    let needle = &frame[1];
    assert_eq!(
        (needle.matrix.scale_x, needle.matrix.scale_y),
        (2.0, 0.5),
        "scale"
    );
    assert_eq!(
        (needle.matrix.rotate_skew0, needle.matrix.rotate_skew1),
        (0.25, -0.25),
        "skew"
    );
    assert_eq!(
        needle.color_transform,
        ColorTransform {
            multiply: [0.5, 1.0, 1.0, 0.25],
            add: [10.0, -20.0, 0.0, 0.0],
        },
        "colour transform"
    );
    assert_eq!(needle.name.as_deref(), Some("needle"), "name");
    assert!(needle.has_filters, "filters");
    assert_eq!(needle.blend_mode, 3, "blend mode after the filters");
    assert!(!needle.visible, "hidden");
    assert_eq!(assets.timeline.placed, [5, 6, 7, 8], "every placed ID");
}

#[test]
fn a_sprite_has_its_own_first_frame() {
    let bytes = TestMovie::new(TestSignature::Gfx)
        .tag(
            39,
            &sprite_body(
                9,
                2,
                &[
                    (26, TestPlace::new(1, 4).clip(3).body2()),
                    (1, Vec::new()),
                    (28, remove_object2_body(1)),
                    (1, Vec::new()),
                ],
            ),
        )
        .bytes();
    let movie = movie(&bytes);
    let assets = movie.assets();
    let frame = &assets.sprites[0].timeline.first_frame;
    assert_eq!(frame.len(), 1, "the removal comes in the second frame");
    assert_eq!(frame[0].clip_depth, Some(3), "a mask");
    assert!(assets.timeline.first_frame.is_empty(), "nothing placed");
}

#[test]
fn colour_transforms_compose_and_apply_with_clamping() {
    let outer = ColorTransform {
        multiply: [0.5, 1.0, 1.0, 1.0],
        add: [0.0, 0.0, 100.0, 0.0],
    };
    let inner = ColorTransform {
        multiply: [1.0, 2.0, 1.0, 0.5],
        add: [20.0, 0.0, 0.0, 0.0],
    };
    let color = Rgba {
        red: 100,
        green: 200,
        blue: 200,
        alpha: 200,
    };
    assert_eq!(
        outer.compose(&inner).apply(color),
        Rgba {
            red: 60,
            green: 255,
            blue: 255,
            alpha: 100
        },
        "(100 + 20) / 2, 400 clamped, 300 clamped, 200 / 2"
    );
    assert!(ColorTransform::IDENTITY.is_identity(), "identity");
}
