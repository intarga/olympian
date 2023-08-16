use super::QcError;
use crate::Flag;

pub fn dip_check(data: &[Option<f32>], high: f32, max: f32) -> Result<Flag, QcError> {
    if data.len() != 3 {
        return Err(QcError::InvalidInputShape("data".to_string()));
    }

    if data.contains(&None) {
        return Ok(Flag::DataMissing);
    }
    let data: Vec<f32> = data.iter().map(|opt| opt.unwrap()).collect();

    if (data[2] < data[1] && data[0] < data[1]) || (data[2] > data[1] && data[0] > data[1]) {
        let diffsum = ((data[2] - data[1]).abs() + (data[1] - data[0]).abs()).abs();
        let diffdiff = ((data[2] - data[1]).abs() - (data[1] - data[0]).abs()).abs();

        if diffsum > high && diffdiff < (diffsum * 35. / 100.) {
            return Ok(Flag::Warn);
        }

        if diffsum > max && diffdiff < (diffsum * 35. / 100.) {
            return Ok(Flag::Fail);
        }
    }
    Ok(Flag::Pass)
}
