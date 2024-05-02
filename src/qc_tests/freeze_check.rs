use crate::{Error, Flag, SeriesCache};

/// Timeseries QC test that checks for streaks of repeating values.
///
/// If `num_points` observations in a row are identical, the last will be flagged as Flag::Fail, if
/// any of the last `num_points` observations are missing, it will be flagged as Flag::DataMissing,
/// else Flag::Pass.
///
/// As (`num_points` - 1) predecessors to each observation are needed, the [`SeriesCache`] provided must have
/// `num_leading_points` >= `num_points` - 1.
///
/// ## Errors
///
/// - data is invalid
/// - data has `num_leading_points` < `num_points` - 1
pub fn freeze_check(data: &SeriesCache, num_points: u8) -> Result<Vec<Flag>, Error> {
    let (leading_trim, lead_overflow) = data
        .num_leading_points
        .overflowing_sub(num_points.saturating_sub(1));

    if lead_overflow || (leading_trim + num_points) as usize > data.values.len() {
        // TODO: nicer error here?
        return Err(Error::InvalidInputShape("data".to_string()));
    }

    let trimmed = &data.values
        [leading_trim as usize..(data.values.len() - data.num_trailing_points as usize)];

    let windows = trimmed.windows(num_points as usize);

    Ok(windows
        .map(|data| {
            if data.contains(&None) {
                return Flag::DataMissing;
            }
            let data: Vec<f32> = data.iter().map(|opt| opt.unwrap()).collect();

            let base = data[0];
            if !data.iter().any(|x| *x != base) {
                return Flag::Fail;
            }
            Flag::Pass
        })
        .collect())
}
