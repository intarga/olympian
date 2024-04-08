use crate::{Error, Flag, SeriesCache};

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

                if diffsum > high && diffdiff < (diffsum * 35. / 100.) {
                    return Flag::Warn;
                }

                if diffsum > max && diffdiff < (diffsum * 35. / 100.) {
                    return Flag::Fail;
                }
            }
            Flag::Pass
        })
        .collect())
}

// TODO: test cases?
