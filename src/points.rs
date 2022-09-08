use crate::util;
use rstar::{primitives::GeomWithData, RTree};

pub enum CoordinateType {
    Cartesian,
    Geodetic,
}

type Point = GeomWithData<[f32; 3], usize>;

pub struct Points {
    tree: RTree<Point>,
    lats: Vec<f32>,
    lons: Vec<f32>,
    elevs: Vec<f32>,
    lafs: Vec<f32>,
    ctype: CoordinateType,
}

impl Points {
    pub fn from_latlons(
        lats: Vec<f32>,
        lons: Vec<f32>,
        elevs: Vec<f32>,
        lafs: Vec<f32>,
        ctype: CoordinateType,
    ) -> Self {
        //TODO: ensure vecs are the same size

        let raw_points: Vec<Point> = match ctype {
            CoordinateType::Cartesian => lats
                .iter()
                .zip(lons.iter())
                .enumerate()
                .map(|(i, (lat, lon))| Point::new([*lat, *lon, 0.0], i))
                .collect(),
            CoordinateType::Geodetic => lats
                .iter()
                .zip(lons.iter())
                .enumerate()
                .map(|(i, (lat, lon))| {
                    Point::new(
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
    ) -> Vec<&Point> {
        let (x, y, z) = convert_coordinates(lat, lon, self.ctype);

        let match_iter = self.tree.locate_within_distance([x, y, z], radius);

        match include_match {
            true => match_iter.collect(),
            false => match_iter
                .filter(|point| point.geom() != (&[x, y, z])) //TODO make sure this acts as expected
                .collect(),
        }
    }

    pub fn get_coords_at_index(&self, i: usize) -> (f32, f32, f32, f32) {
        (self.lats[i], self.lons[i], self.elevs[i], self.lafs[i])
    }
}

pub fn convert_coordinates(lat: f32, lon: f32, ctype: CoordinateType) -> (f32, f32, f32) {
    match ctype {
        Cartesian => (lat, lon, 0.0),
        Geodetic => (
            lat.to_radians().cos() * lon.to_radians().cos() * util::RADIUS_EARTH,
            lat.to_radians().cos() * lon.to_radians().sin() * util::RADIUS_EARTH,
            lat.to_radians().sin() * util::RADIUS_EARTH,
        ),
    }
}
