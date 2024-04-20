use crate::{Error, Flag, SeriesCache};

/// Timeseries QC test that compares each observation against its immediate predecessor and
/// successor.
///
/// The sum and difference of the differences between the observation and each of its neighbours
/// is computed. the observation will be flagged as follows
/// - If values are missing for the observation or either neighbour: DataMissing.
/// - If the difference is less than 35% of the sum AND the sum is greater than `max`: Fail.
/// - If the difference is less than 35% of the sum AND the sum is greater than `high`: Warn.
/// - Else: Pass
///
/// As a predecessor and successor to each observation are needed, the [`SeriesCache`] provided must have
/// `num_leading_points` and `num_trailing_points` >= 1.
///
/// ## Errors
///
/// - data is invalid
/// - data has `num_leading_points` <= 1
/// - data has `num_trailing_points` <= 1
pub fn dip_check(data: &SeriesCache, high: f32, max: f32) -> Result<Vec<Flag>, Error> {
    let (leading_trim, lead_overflow) = data.num_leading_points.overflowing_sub(1);
    let (trailing_trim, trail_overflow) = data.num_trailing_points.overflowing_sub(1);

    if lead_overflow
        || trail_overflow
        || (leading_trim + trailing_trim + 3) as usize > data.values.len()
    {
        // TODO: nicer error here?
        return Err(Error::InvalidInputShape("data".to_string()));
    }

    let trimmed = &data.values[leading_trim as usize..(data.values.len() - trailing_trim as usize)];

    let windows = trimmed.windows(3);

    Ok(windows
        .map(|data| {
            if data.contains(&None) {
                return Flag::DataMissing;
            }
            let data: Vec<f32> = data.iter().map(|opt| opt.unwrap()).collect();

            if (data[2] < data[1] && data[0] < data[1]) || (data[2] > data[1] && data[0] > data[1])
            {
                let diffsum = ((data[2] - data[1]).abs() + (data[1] - data[0]).abs()).abs();
                let diffdiff = ((data[2] - data[1]).abs() - (data[1] - data[0]).abs()).abs();

                if diffdiff < (diffsum * 0.35) {
                    if diffsum > max {
                        return Flag::Fail;
                    }

                    if diffsum > high {
                        return Flag::Warn;
                    }
                }
            }
            Flag::Pass
        })
        .collect())
}

// TODO: test cases?
