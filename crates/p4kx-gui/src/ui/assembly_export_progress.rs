// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{AssemblyExportProgress, AssemblyExportStage};

/// How far the step the export is at has come, from `0.0` to `1.0`, or `None` for a step whose
/// size is not known beforehand: only the Interior counts its containers and, within one, its
/// model files; a container on its own counts as one container.
#[expect(
    clippy::cast_precision_loss,
    reason = "the containers and model files of an Interior fit f32's integers"
)]
pub(super) fn stage_share(progress: AssemblyExportProgress) -> Option<f32> {
    match progress.stage {
        AssemblyExportStage::Interior => {
            let models = if progress.total == 0 {
                0.0
            } else {
                progress.done as f32 / progress.total as f32
            };
            let containers = progress.containers_total.max(1) as f32;
            let read = (progress.containers_done as f32 + models.clamp(0.0, 1.0)) / containers;
            Some(read.clamp(0.0, 1.0))
        }
        AssemblyExportStage::Resolving
        | AssemblyExportStage::Reading
        | AssemblyExportStage::Writing => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(stage: AssemblyExportStage) -> AssemblyExportProgress {
        AssemblyExportProgress {
            stage,
            done: 7,
            total: 9,
            containers_done: 1,
            containers_total: 2,
        }
    }

    fn interior(containers: (u64, u64), models: (u64, u64)) -> AssemblyExportProgress {
        AssemblyExportProgress {
            stage: AssemblyExportStage::Interior,
            done: models.0,
            total: models.1,
            containers_done: containers.0,
            containers_total: containers.1,
        }
    }

    #[test]
    fn a_step_of_unknown_size_has_no_share() {
        for stage in [
            AssemblyExportStage::Resolving,
            AssemblyExportStage::Reading,
            AssemblyExportStage::Writing,
        ] {
            assert_eq!(stage_share(at(stage)), None, "{stage:?}");
        }
    }

    #[test]
    fn the_interior_never_steps_back_when_the_next_container_starts() {
        let steps = [
            interior((0, 3), (0, 10)),
            interior((0, 3), (10, 10)),
            interior((1, 3), (0, 0)),
            interior((1, 3), (5, 10)),
            interior((2, 3), (0, 0)),
            interior((2, 3), (10, 10)),
        ];

        let shares: Vec<f32> = steps.into_iter().filter_map(stage_share).collect();

        assert_eq!(shares.len(), 6, "every step has a share");
        assert!(
            shares.windows(2).all(|pair| pair[0] <= pair[1]),
            "{shares:?}"
        );
        assert!(
            shares.iter().all(|share| (0.0..=1.0).contains(share)),
            "{shares:?}"
        );
    }

    #[test]
    fn a_container_on_its_own_fills_the_bar_with_its_model_files() {
        let start = stage_share(interior((0, 0), (0, 4)));
        let end = stage_share(interior((0, 0), (4, 4)));

        assert_eq!(start, Some(0.0));
        assert_eq!(end, Some(1.0));
    }
}
