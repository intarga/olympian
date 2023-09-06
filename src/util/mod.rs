pub mod spatial_tree;
use crate::Error;

#[derive(Copy, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Flag {
    Pass,
    Fail,
    Warn,
    Inconclusive,
    Invalid,
    DataMissing,
    Isolated,
}

pub(crate) const RADIUS_EARTH: f32 = 6371.0;

pub(crate) fn is_valid(value: f32) -> bool {
    !f32::is_nan(value) && !f32::is_infinite(value)
}

pub(crate) fn convert_coordinates(lat: f32, lon: f32) -> (f32, f32, f32) {
    (
        lat.to_radians().cos() * lon.to_radians().cos() * RADIUS_EARTH,
        lat.to_radians().cos() * lon.to_radians().sin() * RADIUS_EARTH,
        lat.to_radians().sin() * RADIUS_EARTH,
    )
}

pub(crate) fn calc_distance(lat1: f32, lon1: f32, lat2: f32, lon2: f32) -> Result<f32, Error> {
    // lons are checked against 360 here, not 180, because some people apparently use
    // conventions of 0-360 and -360-0...
    if lat1.abs() > 90. || lat2.abs() > 90. || lon1.abs() > 360. || lon2.abs() > 360. {
        return Err(Error::InvalidArg(
            "latlon".to_string(),
            "outside valid range".to_string(),
        ));
    }
    if lat1 == lat2 && lon1 == lon2 {
        return Ok(0.);
    }
    let lat1r = lat1.to_radians();
    let lat2r = lat2.to_radians();
    let lon1r = lon1.to_radians();
    let lon2r = lon2.to_radians();

    let ratio = lat1r.cos() * lon1r.cos() * lat2r.cos() * lon2r.cos()
        + lat1r.cos() * lon1r.sin() * lat2r.cos() * lon2r.sin()
        + lat1r.sin() * lat2r.sin();

    // floating point chaos was leading to this leaking outside the 0-1 range that's
    // valid for arccos, hence the enforcement
    let norm_ratio = ratio.max(0.).min(1.);

    Ok(norm_ratio.acos() * RADIUS_EARTH)
}

pub(crate) fn calc_distance_xyz(x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32) -> f32 {
    ((x0 - x1) * (x0 - x1) + (y0 - y1) * (y0 - y1) + (z0 - z1) * (z0 - z1)).sqrt()
}
