use crate::{Flag, SeriesCache};

/// Range check with a correction for wind direction outside 0-360.
///
/// If the direction is -20-0, or 360-380, 360 will be added or subtracted to get it back into
/// the correct range.
pub fn range_check_wind_direction(data: &SeriesCache) -> Vec<(Flag, Option<f32>)> {
    let trimmed = &data.values
        [data.num_leading_points as usize..(data.values.len() - data.num_trailing_points as usize)];

    let windows = trimmed.iter();

    // TODO: get to the bottom of weird -3.0 handling: kvalobs code looks for a value -3.0, and
    // avoids flagging that if X_5 (lowest?) is also -3.0. From comments in the code, it looks like
    // this has to do with a special param_id?

    // TODO: figure out what the Y param in kvalobs code is. Windspeed? Why check it's not zero for the correction runs, but not the first check?

    windows
        .map(|data| match data {
            None => (Flag::DataMissing, None),
            Some(data) => {
                if *data > 380. || *data < -20. {
                    return (Flag::Fail, None);
                } else if *data < 0. {
                    // TODO: is Warn the correct flag here?
                    return (Flag::Warn, Some(*data + 360.));
                } else if *data >= 360. {
                    return (Flag::Warn, Some(*data - 360.));
                }
                (Flag::Pass, None)
            }
        })
        .collect()
}
