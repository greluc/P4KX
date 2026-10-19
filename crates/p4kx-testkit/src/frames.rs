use std::time::Duration;

/// Checks that the measured frame times of a test stay short without failing on one stall.
///
/// A machine under load delays single frames, so the check asks for the 90th percentile to stay
/// below `typical` and the slowest frame below `ceiling`.
///
/// # Panics
///
/// Panics, naming `label`, when either bound is exceeded.
pub fn assert_steady_frames(
    label: &str,
    frames: &[Duration],
    typical: Duration,
    ceiling: Duration,
) {
    let mut sorted = frames.to_vec();
    sorted.sort_unstable();
    let Some(&slowest) = sorted.last() else {
        return;
    };
    let rank = (sorted.len() * 9).div_ceil(10).saturating_sub(1);
    let percentile = sorted.get(rank).copied().unwrap_or(slowest);
    assert!(
        percentile < typical,
        "{label}: the 90th percentile of {} frames was {percentile:?}, limit {typical:?}",
        sorted.len()
    );
    assert!(
        slowest < ceiling,
        "{label}: the slowest of {} frames took {slowest:?}, ceiling {ceiling:?}",
        sorted.len()
    );
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::assert_steady_frames;

    const TYPICAL: Duration = Duration::from_millis(16);
    const CEILING: Duration = Duration::from_millis(100);

    fn millis(values: &[u64]) -> Vec<Duration> {
        values.iter().copied().map(Duration::from_millis).collect()
    }

    #[test]
    fn one_stall_among_steady_frames_passes() {
        let mut frames = millis(&[5; 19]);
        frames.push(Duration::from_millis(40));
        assert_steady_frames("stall", &frames, TYPICAL, CEILING);
    }

    #[test]
    fn no_frames_pass() {
        assert_steady_frames("none", &[], TYPICAL, CEILING);
    }

    #[test]
    #[should_panic(expected = "90th percentile")]
    fn slow_frames_in_general_fail() {
        assert_steady_frames("slow", &millis(&[20; 10]), TYPICAL, CEILING);
    }

    #[test]
    #[should_panic(expected = "slowest")]
    fn one_frame_over_the_ceiling_fails() {
        let mut frames = millis(&[5; 19]);
        frames.push(Duration::from_millis(150));
        assert_steady_frames("ceiling", &frames, TYPICAL, CEILING);
    }
}
