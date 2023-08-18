pub(crate) const RADIUS_EARTH: f32 = 6371.0;

pub(crate) fn is_valid(value: f32) -> bool {
    !f32::is_nan(value) && !f32::is_infinite(value)
}
