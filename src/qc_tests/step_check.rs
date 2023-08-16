use super::QcError;
use crate::Flag;

pub fn step_check(data: &[Option<f32>], high: f32, max: f32) -> Result<Flag, QcError> {
    if data.len() != 2 {
        return Err(QcError::InvalidInputShape("data".to_string()));
    }

    if data.contains(&None) {
        return Ok(Flag::DataMissing);
    }
    let data: Vec<f32> = data.iter().map(|opt| opt.unwrap()).collect();

    if (data[0] - data[1]).abs() > high {
        return Ok(Flag::Warn);
    }
    if (data[0] - data[1]).abs() > max {
        return Ok(Flag::Fail);
    }
    Ok(Flag::Pass)
}
