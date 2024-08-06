use crate::{Flag, SeriesCache};

/// Range check with a correction for humidity over 100%.
///
/// Humidity less than 5% or greater than 105% returns Flag::Fail, between 100% and 105% it is,
/// corrected down to 100%.
pub fn range_check_humidity(data: &SeriesCache) -> Vec<(Flag, Option<f32>)> {
    let trimmed = &data.values
        [data.num_leading_points as usize..(data.values.len() - data.num_trailing_points as usize)];

    let windows = trimmed.iter();

    windows
        .map(|data| match data {
            None => (Flag::DataMissing, None),
            Some(data) => {
                if *data > 105. || *data < 5. {
                    return (Flag::Fail, None);
                } else if *data > 100. {
                    // TODO: is Warn the correct flag here?
                    return (Flag::Warn, Some(100.));
                }
                (Flag::Pass, None)
            }
        })
        .collect()
}
