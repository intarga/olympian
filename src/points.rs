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
        let radius_earth = 6371.0; //TODO define this somewhere more sensible?

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
                            (lat.to_radians().cos() * lon.to_radians().cos() * radius_earth),
                            (lat.to_radians().cos() * lon.to_radians().sin() * radius_earth),
                            (lat.to_radians().sin() * radius_earth),
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
}
