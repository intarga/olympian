const MISSING_VALUE: f32 = f32::NAN;
pub const RADIUS_EARTH: f32 = 6371.0;

pub fn is_valid(value: f32) -> bool {
    !f32::is_nan(value) && !f32::is_infinite(value) && value != MISSING_VALUE
}
