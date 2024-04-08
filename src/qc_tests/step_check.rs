use crate::{Error, Flag, SeriesCache};

pub fn step_check(data: &SeriesCache, high: f32, max: f32) -> Result<Vec<Flag>, Error> {
    let (leading_trim, lead_overflow) = data.num_leading_points.overflowing_sub(1);

    if lead_overflow || (leading_trim + 2) as usize > data.values.len() {
        // TODO: nicer error here?
        return Err(Error::InvalidInputShape("data".to_string()));
    }

    let trimmed = &data.values[leading_trim as usize..];

    let windows = trimmed.windows(2);

    Ok(windows
        .map(|data| {
            if data.contains(&None) {
                return Flag::DataMissing;
            }
            let data: Vec<f32> = data.iter().map(|opt| opt.unwrap()).collect();

            if (data[0] - data[1]).abs() > high {
                return Flag::Warn;
            }
            if (data[0] - data[1]).abs() > max {
                return Flag::Fail;
            }
            Flag::Pass
        })
        .collect())
}
