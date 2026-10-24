// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{AssemblyExportProgress, AssemblyExportStage};

/// The share of the export each step stands for: the models are the longest step, and the
/// Interior fills the span from the models to the file as its containers and, within one, its
/// model files are read; a container on its own counts as one container.
#[expect(
    clippy::cast_precision_loss,
    reason = "the containers and model files of an Interior fit f32's integers"
)]
pub(super) fn stage_fraction(progress: AssemblyExportProgress) -> f32 {
    match progress.stage {
        AssemblyExportStage::Resolving => 0.1,
        AssemblyExportStage::Reading => 0.4,
        AssemblyExportStage::Interior => {
            let models = if progress.total == 0 {
                0.0
            } else {
                progress.done as f32 / progress.total as f32
            };
            let containers = progress.containers_total.max(1) as f32;
            let read = (progress.containers_done as f32 + models.clamp(0.0, 1.0)) / containers;
            0.3_f32.mul_add(read.clamp(0.0, 1.0), 0.4)
        }
        AssemblyExportStage::Writing => 0.8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_interior_never_steps_back_when_the_next_container_starts() {
        let steps = [
            interior((0, 3), (0, 10)),
            interior((0, 3), (10, 10)),
            interior((1, 3), (0, 0)),
            interior((1, 3), (5, 10)),
            interior((2, 3), (0, 0)),
            interior((2, 3), (10, 10)),
        ];

        let fractions: Vec<f32> = steps.into_iter().map(stage_fraction).collect();

        assert!(
            fractions.windows(2).all(|pair| pair[0] <= pair[1]),
            "{fractions:?}"
        );
        assert!(
            fractions.iter().all(|f| (0.4..=0.7001).contains(f)),
            "{fractions:?}"
        );
    }

    #[test]
    fn a_container_on_its_own_fills_the_span_with_its_model_files() {
        let start = stage_fraction(interior((0, 0), (0, 4)));
        let end = stage_fraction(interior((0, 0), (4, 4)));

        assert!(start < end, "{start} before {end}");
        assert!((end - 0.7).abs() < 1e-6, "{end}");
    }
}
