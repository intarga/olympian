use crate::util;
use rstar::{primitives::GeomWithData, RTree};

// TODO: deprecate this distinction
#[derive(Clone, Copy)]
pub enum CoordinateType {
    Cartesian,
    Geodetic,
}

pub type SpatialPoint = GeomWithData<[f32; 3], usize>;

pub struct SpatialTree {
    pub tree: RTree<SpatialPoint>,
    pub lats: Vec<f32>,
    pub lons: Vec<f32>,
    pub elevs: Vec<f32>,
    pub lafs: Vec<f32>,
    pub ctype: CoordinateType,
}

impl SpatialTree {
    pub fn from_latlons(
        lats: Vec<f32>,
        lons: Vec<f32>,
        elevs: Vec<f32>,
        lafs: Vec<f32>,
        ctype: CoordinateType,
    ) -> Self {
        //TODO: ensure vecs are the same size

        let raw_points: Vec<SpatialPoint> = match ctype {
            CoordinateType::Cartesian => lats
                .iter()
                .zip(lons.iter())
                .enumerate()
                .map(|(i, (lat, lon))| SpatialPoint::new([*lat, *lon, 0.0], i))
                .collect(),
            CoordinateType::Geodetic => lats
                .iter()
                .zip(lons.iter())
                .enumerate()
                .map(|(i, (lat, lon))| {
                    SpatialPoint::new(
                        [
                            (lat.to_radians().cos() * lon.to_radians().cos() * util::RADIUS_EARTH),
                            (lat.to_radians().cos() * lon.to_radians().sin() * util::RADIUS_EARTH),
                            (lat.to_radians().sin() * util::RADIUS_EARTH),
                        ],
                        i,
                    )
                })
                .collect(),
        };

        let tree = RTree::bulk_load(raw_points);

        Self {
            tree,
            lats,
            lons,
            elevs,
            lafs,
            ctype,
        }
    }

    pub fn get_neighbours(
        &self,
        lat: f32,
        lon: f32,
        radius: f32,
        include_match: bool,
    ) -> Vec<&SpatialPoint> {
        let (x, y, z) = convert_coordinates(lat, lon, self.ctype);

        let match_iter = self.tree.locate_within_distance([x, y, z], radius);

        match include_match {
            true => match_iter.collect(),
            false => match_iter
                .filter(|point| *point.geom() != [x, y, z])
                .collect(),
        }
    }

    pub fn get_neighbours_with_distance(
        &self,
        lat: f32,
        lon: f32,
        radius: f32,
        include_match: bool,
    ) -> (Vec<&SpatialPoint>, Vec<f32>) {
        let points = self.get_neighbours(lat, lon, radius, include_match);
        let vec_length = points.len();

        let mut distances = vec![0.; vec_length];

        let (x, y, z) = convert_coordinates(lat, lon, self.ctype);

        for i in 0..vec_length {
            let (x1, y1, z1) = convert_coordinates(
                self.lats[points[i].data],
                self.lons[points[i].data],
                self.ctype,
            );
            distances[i] = calc_distance_xyz(x, y, z, x1, y1, z1)
        }

        (points, distances)
    }

    pub fn get_coords_at_index(&self, i: usize) -> (f32, f32, f32, f32) {
        (self.lats[i], self.lons[i], self.elevs[i], self.lafs[i])
    }
}

pub fn convert_coordinates(lat: f32, lon: f32, ctype: CoordinateType) -> (f32, f32, f32) {
    match ctype {
        CoordinateType::Cartesian => (lat, lon, 0.0),
        CoordinateType::Geodetic => (
            lat.to_radians().cos() * lon.to_radians().cos() * util::RADIUS_EARTH,
            lat.to_radians().cos() * lon.to_radians().sin() * util::RADIUS_EARTH,
            lat.to_radians().sin() * util::RADIUS_EARTH,
        ),
    }
}

pub fn calc_distance(lat1: f32, lon1: f32, lat2: f32, lon2: f32, ctype: CoordinateType) -> f32 {
    match ctype {
        CoordinateType::Cartesian => {
            let dx = lon1 - lon2;
            let dy = lat1 - lat2;
            (dx * dx + dy * dy).sqrt()
        }
        CoordinateType::Geodetic => {
            // TODO: check latlon validity?
            let lat1r = lat1.to_radians();
            let lat2r = lat2.to_radians();
            let lon1r = lon1.to_radians();
            let lon2r = lon2.to_radians();

            let ratio = lat1r.cos() * lon1r.cos() * lat2r.cos() * lon2r.cos()
                + lat1r.cos() * lon1r.sin() * lat2r.cos() * lon2r.sin()
                + lat1r.sin() * lat2r.sin();

            ratio.acos() * util::RADIUS_EARTH
        }
    }
}

pub fn calc_distance_xyz(x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32) -> f32 {
    ((x0 - x1) * (x0 - x1) + (y0 - y1) * (y0 - y1) + (z0 - z1) * (z0 - z1)).sqrt()
}
