// Copyright © 2016, Canal TP and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
//     the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
//     powered by Canal TP (www.canaltp.fr).
// Help us simplify mobility and open public transport:
//     a non ending quest to the responsive locomotion way of traveling!
//
// LICENCE: This program is free software; you can redistribute it
// and/or modify it under the terms of the GNU Affero General Public
// License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with this program. If not, see
// <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// IRC #navitia on freenode
// https://groups.google.com/d/forum/navitia
// www.navitia.io

use geo_types::{LineString, MultiLineString};
use log::warn;
use osmpbfreader::OsmObj::*;
use std::collections::btree_set::BTreeSet;
use std::collections::BTreeMap;
use std::path::Path;

pub type OsmPbfReader = osmpbfreader::OsmPbfReader<std::fs::File>;

pub trait Id<T> {
    fn id(&self) -> &str;
}

pub trait Shape {
    fn get_shape(&self) -> &Vec<Vec<Coord>>;
}

pub fn shape_to_multi_line_string<T>(container: &T) -> MultiLineString<f64>
where
    T: Shape,
{
    container
        .get_shape()
        .iter()
        .map(|way_coords| {
            way_coords
                .iter()
                .map(|coord| (coord.lon, coord.lat))
                .collect::<LineString<f64>>()
        })
        .collect::<MultiLineString<f64>>()
}

#[derive(Debug, Clone)]
pub struct Coord {
    pub lat: f64,
    pub lon: f64,
}
impl Coord {
    fn new(lat_param: f64, lon_param: f64) -> Coord {
        Coord {
            lat: lat_param,
            lon: lon_param,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StopPointType {
    StopPosition,
    Platform,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct StopPoint {
    pub id: String,
    pub stop_point_type: StopPointType,
    pub coord: Coord,
    pub name: String,
    pub all_osm_tags: osmpbfreader::objects::Tags,
}

#[derive(Debug, Clone)]
pub struct StationAccess {
    pub id: String,
    pub coord: Coord,
    pub name: String,
    pub code: String,
    pub all_osm_tags: osmpbfreader::objects::Tags,
}

#[derive(Debug, Clone)]
pub struct StopArea {
    pub id: String,
    pub coord: Coord,
    pub name: String,
    pub all_osm_tags: osmpbfreader::objects::Tags,
    pub stop_point_ids: Vec<String>,
    pub station_access_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RoutePoint {
    pub role: String,
    pub stop_point_id: String,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub id: String,
    pub name: String,
    pub code: String,
    pub destination: String,
    pub origin: String,
    pub colour: String,
    pub operator: String,
    pub network: String,
    pub mode: String,
    pub frequency: String,
    pub opening_hours: String,
    pub frequency_exceptions: String,
    pub travel_time: String,
    pub all_osm_tags: osmpbfreader::objects::Tags,
    pub ordered_route_points: Vec<RoutePoint>,
    pub shape: Vec<Vec<Coord>>,
}

// small utility since we'll get very often a tag as string
trait ToDefaultString {
    fn get_default_string(&self, key: &str) -> String;
}

impl ToDefaultString for osmpbfreader::objects::Tags {
    fn get_default_string(&self, key: &str) -> String {
        self.get(key).map_or_else(String::new, ToString::to_string)
    }
}

impl Route {
    fn contains_stop_point_id(&self, stop_point_id: &str) -> bool {
        self.ordered_route_points
            .iter()
            .filter(|rp| rp.stop_point_id == *stop_point_id)
            .count()
            != 0
    }
}

impl Route {
    fn get_stop_point_roles<'a>(&'a self, stop_point_id: &str) -> Vec<&'a String> {
        self.ordered_route_points
            .iter()
            .filter(|rp| rp.stop_point_id == *stop_point_id)
            .map(|rp| &rp.role)
            .collect()
    }
}

impl Id<Route> for Route {
    fn id(&self) -> &str {
        self.id.as_str()
    }
}

impl Shape for Route {
    fn get_shape(&self) -> &Vec<Vec<Coord>> {
        &self.shape
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    pub id: String,
    pub name: String,
    pub code: String,
    pub colour: String,
    pub operator: String,
    pub network: String,
    pub mode: String,
    pub frequency: String,
    pub opening_hours: String,
    pub frequency_exceptions: String,
    pub all_osm_tags: osmpbfreader::objects::Tags,
    pub shape: Vec<Vec<Coord>>,
    pub routes_id: Vec<String>,
}

impl Id<Line> for Line {
    fn id(&self) -> &str {
        self.id.as_str()
    }
}

impl Shape for Line {
    fn get_shape(&self) -> &Vec<Vec<Coord>> {
        &self.shape
    }
}

#[allow(dead_code)]
/* to_multilinestring is to be used when issue #8 is resolved*/
impl Route {
    fn to_multilinestring(&self) -> wkt::types::MultiLineString<f64> {
        let wkt_linestrings = self
            .shape
            .iter()
            .map(|vec_coord| {
                vec_coord
                    .iter()
                    .map(|coord| wkt::types::Coord {
                        x: coord.lon,
                        y: coord.lat,
                        z: None,
                        m: None,
                    })
                    .collect()
            })
            .map(wkt::types::LineString)
            .collect();
        wkt::types::MultiLineString(wkt_linestrings)
    }
}

fn shape_to_wkt(shape: &[Vec<Coord>]) -> String {
    if shape.is_empty() {
        "".to_string()
    } else {
        let linestring: String = shape
            .iter()
            .map(|vec_coord| {
                vec_coord
                    .iter()
                    .map(|coord| format!("{} {}", coord.lon.to_string(), coord.lat.to_string()))
                    .collect::<Vec<String>>()
                    .join(", ")
            })
            .collect::<Vec<String>>()
            .join("), (");
        format!("MULTILINESTRING(({}))", linestring)
    }
}

pub struct OsmTcResponse {
    pub stop_points: Vec<StopPoint>,
    pub station_accesses: Vec<StationAccess>,
    pub stop_areas: Vec<StopArea>,
    pub routes: Option<Vec<Route>>,
    pub lines: Option<Vec<Line>>,
}

pub fn parse_osm_pbf(path: &str) -> OsmPbfReader {
    let path = std::path::Path::new(&path);
    osmpbfreader::OsmPbfReader::new(std::fs::File::open(&path).unwrap())
}

fn is_stop_point(obj: &osmpbfreader::OsmObj) -> bool {
    (obj.is_node() || obj.is_way())
        && (obj.tags().contains("public_transport", "platform")
            || obj.tags().contains("public_transport", "stop_position")
            || obj.tags().contains("highway", "bus_stop")
            || obj.tags().contains("railway", "tram_stop"))
}

fn is_station_access(obj: &osmpbfreader::OsmObj) -> bool {
    obj.is_node()
        && (obj.tags().contains("railway", "subway_entrance")
            || obj.tags().contains("railway", "train_station_entrance"))
}

fn is_stop_area(obj: &osmpbfreader::OsmObj) -> bool {
    obj.is_relation() && obj.tags().contains("public_transport", "stop_area")
}

fn is_pt_route_type(
    osm_id: osmpbfreader::objects::RelationId,
    tag_name: &str,
    route_type: Option<&str>,
) -> bool {
    let non_pt_route_type = vec![
        "bicycle",
        "canoe",
        "detour",
        "fitness_trail",
        "foot",
        "hiking",
        "horse",
        "inline_skates",
        "mtb",
        "nordic_walking",
        "pipeline",
        "piste",
        "power",
        "proposed",
        "road",
        "running",
        "ski",
        "historic",
        "path",
        "junction",
        "tracks",
    ];
    let pt_route_type = vec![
        "trolleybus",
        "bus",
        "train",
        "subway",
        "light_rail",
        "monorail",
        "tram",
        "railway",
        "ferry",
        "coach",
        "aerialway",
        "funicular",
        "rail",
        "share_taxi",
    ];
    match route_type {
        Some(r) => {
            let is_in_white_list = pt_route_type.contains(&r);
            let is_in_black_list = non_pt_route_type.contains(&r);
            if !is_in_white_list && !is_in_black_list {
                warn!("tag {} is unknown : relation {} is extracted. Update mode list in crate to remove this message.", tag_name, osm_id.0);
            }
            !is_in_black_list
        }
        None => {
            warn!(
                "tag {} is empty : relation {} is ignored",
                tag_name, osm_id.0
            );
            false
        }
    }
}

fn is_line(obj: &osmpbfreader::OsmObj) -> bool {
    let route_type = "route_master";
    obj.is_relation()
        && obj.tags().contains("type", route_type)
        && is_pt_route_type(
            obj.id().relation().unwrap(),
            route_type,
            obj.tags().get(route_type).map(|s| s.as_str()),
        )
}

fn is_route(obj: &osmpbfreader::OsmObj) -> bool {
    let route_type = "route";
    obj.is_relation()
        && obj.tags().contains("type", route_type)
        && is_pt_route_type(
            obj.id().relation().unwrap(),
            route_type,
            obj.tags().get(route_type).map(|s| s.as_str()),
        )
}

const STOP_ROLES: [&str; 7] = [
    "stop",
    "platform",
    "stop_exit_only",
    "stop_entry_only",
    "platform_exit_only",
    "platform_entry_only",
    "fixme",
];

fn is_stop(refe: &osmpbfreader::Ref) -> bool {
    STOP_ROLES.contains(&refe.role.as_str())
}

fn get_one_coord_from_way(
    obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    way: &osmpbfreader::objects::Way,
) -> Coord {
    way.nodes
        .iter()
        .filter_map(|node_id| {
            obj_map
                .get(&(*node_id).into())
                .and_then(|obj| obj.node())
                .map(|node| Coord::new(node.lat(), node.lon()))
        })
        .next()
        .unwrap_or_else(|| Coord::new(0., 0.))
}

fn get_one_coord_from_rel(
    obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    rel: &osmpbfreader::objects::Relation,
) -> Coord {
    rel.refs
        .iter()
        .filter_map(|refe| obj_map.get(&refe.member))
        .filter_map(|osm_obj| match *osm_obj {
            Way(ref way) => Some(get_one_coord_from_way(obj_map, way)),
            Node(ref node) => Some(Coord::new(node.lat(), node.lon())),
            Relation(..) => None,
        })
        .next()
        .unwrap_or_else(|| Coord::new(0., 0.))
}

fn osm_way_to_vec(
    obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    osm_way: &osmpbfreader::Way,
) -> Vec<Coord> {
    osm_way
        .nodes
        .iter()
        .filter_map(|id| obj_map.get(&osmpbfreader::OsmId::Node(*id)))
        .filter_map(|osm_obj| osmpbfreader::OsmObj::node(osm_obj))
        .map(|node| Coord::new(node.lat(), node.lon()))
        .collect()
}

fn osm_route_to_shape(
    obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    osm_relation: &osmpbfreader::Relation,
) -> Vec<Vec<Coord>> {
    osm_relation
        .refs
        .iter()
        .filter(|refe| !is_stop(*refe))
        .filter_map(|refe| obj_map.get(&refe.member))
        .filter_map(|osm_obj| osmpbfreader::OsmObj::way(osm_obj))
        .filter_map(|osm_way| {
            let coord_vec = osm_way_to_vec(obj_map, osm_way);
            match coord_vec.len() {
                0 | 1 => None,
                _ => Some(coord_vec),
            }
        })
        .collect()
}

fn osm_stop_area_to_station_access_list(
    obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    osm_relation: &osmpbfreader::Relation,
) -> Vec<String> {
    osm_relation
        .refs
        .iter()
        .filter(|refe| !is_stop(*refe))
        .filter_map(|refe| obj_map.get(&refe.member))
        .filter(|osm_obj| is_station_access(*osm_obj))
        .filter_map(|osm_obj| osmpbfreader::OsmObj::node(osm_obj))
        .map(|node| format!("node:{}", node.id.0))
        .collect()
}

fn osm_line_to_shape(
    obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    osm_relations_ref: &[osmpbfreader::Ref],
) -> Vec<Vec<Coord>> {
    osm_relations_ref
        .iter()
        .filter_map(|refe| obj_map.get(&refe.member))
        .filter_map(|osm_obj| osmpbfreader::OsmObj::relation(osm_obj))
        .flat_map(|relation| osm_route_to_shape(obj_map, relation))
        .collect()
}

fn osm_route_to_route_points_list(osm_relation: &osmpbfreader::Relation) -> Vec<RoutePoint> {
    osm_relation
        .refs
        .iter()
        .filter(|refe| is_stop(*refe))
        .map(|refe| {
            let stop_point_id = match refe.member {
                osmpbfreader::OsmId::Node(obj_id) => format!("node:{}", obj_id.0),
                osmpbfreader::OsmId::Way(obj_id) => format!("way:{}", obj_id.0),
                osmpbfreader::OsmId::Relation(obj_id) => format!("relation:{}", obj_id.0),
            };
            RoutePoint {
                role: refe.role.to_string(),
                stop_point_id,
            }
        })
        .collect()
}

fn osm_line_to_routes_list(route_master: &osmpbfreader::Relation) -> Vec<String> {
    route_master
        .refs
        .iter()
        .filter_map(|refe| match refe.member {
            osmpbfreader::OsmId::Relation(rel_id) => Some(format!("relation:{}", rel_id.0)),
            _ => None,
        })
        .collect()
}

fn osm_obj_to_route(
    obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    obj: &osmpbfreader::OsmObj,
) -> Option<Route> {
    let osm_tags = obj.tags().clone();
    obj.relation().map(|rel| Route {
        id: format!("relation:{}", rel.id.0),
        name: rel.tags.get_default_string("name"),
        code: rel.tags.get_default_string("ref"),
        destination: rel.tags.get_default_string("to"),
        origin: rel.tags.get_default_string("from"),
        mode: rel.tags.get_default_string("route"),
        colour: rel.tags.get_default_string("colour"),
        operator: rel.tags.get_default_string("operator"),
        network: rel.tags.get_default_string("network"),
        frequency: rel.tags.get_default_string("interval"),
        opening_hours: rel.tags.get_default_string("opening_hours"),
        frequency_exceptions: rel.tags.get_default_string("interval:conditional"),
        travel_time: rel.tags.get_default_string("duration"),
        all_osm_tags: osm_tags,
        ordered_route_points: osm_route_to_route_points_list(rel),
        shape: osm_route_to_shape(obj_map, rel),
    })
}

fn osm_obj_to_line(
    obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    obj: &osmpbfreader::OsmObj,
) -> Option<Line> {
    let osm_tags = obj.tags().clone();
    obj.relation().map(|rel| Line {
        id: format!("relation:{}", rel.id.0),
        name: rel.tags.get_default_string("name"),
        code: rel.tags.get_default_string("ref"),
        colour: rel.tags.get_default_string("colour"),
        mode: rel.tags.get_default_string("route_master"),
        operator: rel.tags.get_default_string("operator"),
        network: rel.tags.get_default_string("network"),
        frequency: rel.tags.get_default_string("interval"),
        opening_hours: rel.tags.get_default_string("opening_hours"),
        frequency_exceptions: rel.tags.get_default_string("interval:conditional"),
        all_osm_tags: osm_tags,
        shape: osm_line_to_shape(obj_map, &rel.refs),
        routes_id: osm_line_to_routes_list(rel),
    })
}

fn osm_obj_to_stop_point(
    obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    obj: &osmpbfreader::OsmObj,
) -> StopPoint {
    let (obj_type, obj_id, coord) = match *obj {
        Relation(ref rel) => ("relation", rel.id.0, get_one_coord_from_rel(obj_map, rel)),
        Way(ref way) => ("way", way.id.0, get_one_coord_from_way(obj_map, way)),
        Node(ref node) => (
            "node",
            node.id.0,
            Coord {
                lat: node.lat(),
                lon: node.lon(),
            },
        ),
    };
    let name = obj.tags().get_default_string("name");
    let id = format!("{}:{}", obj_type, obj_id);
    let osm_tags = obj.tags().clone();
    StopPoint {
        id,
        stop_point_type: StopPointType::Unknown,
        name,
        coord,
        all_osm_tags: osm_tags,
    }
}

impl From<&osmpbfreader::OsmObj> for StationAccess {
    fn from(obj: &osmpbfreader::OsmObj) -> Self {
        let node = &*obj.node().unwrap();
        let osm_tags = obj.tags().clone();
        StationAccess {
            id: format!("node:{}", node.id.0),
            name: obj.tags().get_default_string("name"),
            code: obj.tags().get_default_string("ref"),
            coord: Coord {
                lat: node.lat(),
                lon: node.lon(),
            },
            all_osm_tags: osm_tags,
        }
    }
}

fn osm_obj_to_stop_area(
    obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    obj: &osmpbfreader::OsmObj,
) -> StopArea {
    let rel = &*obj.relation().unwrap();
    let (obj_type, obj_id, coord) = ("relation", rel.id.0, get_one_coord_from_rel(obj_map, &rel));
    StopArea {
        id: format!("{}:{}", obj_type, obj_id),
        name: obj.tags().get_default_string("name"),
        coord,
        all_osm_tags: obj.tags().clone(),
        stop_point_ids: osm_stop_area_to_stop_point_list(rel),
        station_access_ids: osm_stop_area_to_station_access_list(obj_map, rel),
    }
}

fn osm_stop_area_to_stop_point_list(osm_relation: &osmpbfreader::Relation) -> Vec<String> {
    osm_relation
        .refs
        .iter()
        .filter(|refe| refe.role.as_str() == "platform")
        .map(|refe| match refe.member {
            osmpbfreader::OsmId::Node(obj_id) => format!("node:{}", obj_id.0),
            osmpbfreader::OsmId::Way(obj_id) => format!("way:{}", obj_id.0),
            osmpbfreader::OsmId::Relation(obj_id) => format!("relation:{}", obj_id.0),
        })
        .collect()
}

pub fn get_stop_points_from_osm(pbf: &mut OsmPbfReader) -> Vec<StopPoint> {
    let objects = pbf.get_objs_and_deps(is_stop_point).unwrap();
    objects
        .values()
        .filter(|x| is_stop_point(*x))
        .map(|obj| osm_obj_to_stop_point(&objects, obj))
        .collect()
}

pub fn get_station_accesses_from_osm(pbf: &mut OsmPbfReader) -> Vec<StationAccess> {
    let objects = pbf.get_objs_and_deps(is_station_access).unwrap();
    objects
        .values()
        .filter(|x| is_station_access(*x))
        .map(StationAccess::from)
        .collect()
}

pub fn get_stop_areas_from_osm(pbf: &mut OsmPbfReader) -> Vec<StopArea> {
    let objects = pbf.get_objs_and_deps(is_stop_area).unwrap();
    objects
        .values()
        .filter(|x| is_stop_area(*x))
        .map(|obj| osm_obj_to_stop_area(&objects, obj))
        .collect()
}

pub fn get_routes_from_osm(pbf: &mut OsmPbfReader) -> Vec<Route> {
    let objects = pbf.get_objs_and_deps(is_route).unwrap();
    objects
        .values()
        .filter(|x| is_route(*x))
        .filter_map(|obj| osm_obj_to_route(&objects, obj))
        .collect()
}

pub fn get_lines_from_osm(pbf: &mut OsmPbfReader) -> Vec<Line> {
    let objects = pbf.get_objs_and_deps(is_line).unwrap();
    objects
        .values()
        .filter(|x| is_line(*x))
        .filter_map(|obj| osm_obj_to_line(&objects, obj))
        .collect()
}

pub fn get_routes_from_stop<'a>(routes: &'a [Route], stop_point: &StopPoint) -> Vec<&'a Route> {
    routes
        .iter()
        .filter(|route| route.contains_stop_point_id(&stop_point.id))
        .collect()
}

pub fn categorize_stop_point(stop_point: &mut StopPoint, routes: Vec<&Route>) {
    if stop_point
        .all_osm_tags
        .contains("public_transport", "platform")
    {
        stop_point.stop_point_type = StopPointType::Platform;
    } else if stop_point
        .all_osm_tags
        .contains("public_transport", "stop_position")
    {
        stop_point.stop_point_type = StopPointType::StopPosition;
    } else {
        let mut routes_ptv2: Vec<&Route> = routes
            .into_iter()
            .filter(|r| r.all_osm_tags.contains("public_transport:version", "2"))
            .collect();
        warn!(
            "categorization of stop_point {} needs pt_v2 routes. {} ptv2 routes found",
            stop_point.id,
            routes_ptv2.len()
        );
        routes_ptv2.sort_by(|a, b| b.id.cmp(&a.id));
        for route in routes_ptv2 {
            let ptv2_stop_point_uses = route.get_stop_point_roles(&stop_point.id);
            if ptv2_stop_point_uses.contains(&&String::from("platform"))
                || ptv2_stop_point_uses.contains(&&String::from("platform_exit_only"))
                || ptv2_stop_point_uses.contains(&&String::from("platform_entry_only"))
            {
                stop_point.stop_point_type = StopPointType::Platform;
            } else if ptv2_stop_point_uses.contains(&&String::from("stop"))
                || ptv2_stop_point_uses.contains(&&String::from("stop_exit_only"))
                || ptv2_stop_point_uses.contains(&&String::from("stop_entry_only"))
            {
                stop_point.stop_point_type = StopPointType::StopPosition;
            }
            if stop_point.stop_point_type != StopPointType::Unknown {
                break;
            }
        }
    }
}

pub fn update_stop_points_type(stop_points: &mut [StopPoint], routes: &[Route]) {
    stop_points.iter_mut().for_each(|mut sp| {
        let route_from_stops = get_routes_from_stop(&routes, sp);
        categorize_stop_point(&mut sp, route_from_stops);
    })
}

pub fn get_osm_tcobjects(parsed_pbf: &mut OsmPbfReader, stops_only: bool) -> OsmTcResponse {
    let mut stop_points = get_stop_points_from_osm(parsed_pbf);
    let station_accesses = get_station_accesses_from_osm(parsed_pbf);
    let stop_areas = get_stop_areas_from_osm(parsed_pbf);
    if stops_only {
        OsmTcResponse {
            stop_points,
            station_accesses,
            stop_areas,
            routes: None,
            lines: None,
        }
    } else {
        let routes = get_routes_from_osm(parsed_pbf);
        let lines = get_lines_from_osm(parsed_pbf);
        update_stop_points_type(&mut stop_points, &routes);
        OsmTcResponse {
            stop_points,
            station_accesses,
            stop_areas,
            routes: Some(routes),
            lines: Some(lines),
        }
    }
}

pub fn write_stop_points_to_csv<P: AsRef<Path>>(
    stop_points: &[StopPoint],
    output_dir: P,
    all_tags: bool,
) {
    let output_dir = output_dir.as_ref();
    let csv_file = output_dir.join("osm-transit-extractor_stop_points.csv");

    let mut wtr = csv::Writer::from_path(csv_file).unwrap();
    let default_header = ["stop_point_id", "lat", "lon", "name", "stop_point_type"];
    let osm_tag_list: BTreeSet<String> = stop_points
        .iter()
        .flat_map(|s| s.all_osm_tags.keys().map(|s| s.to_string()))
        .collect();
    if all_tags {
        let osm_header = osm_tag_list.iter().map(|s| format!("osm:{}", s));
        let v: Vec<_> = default_header
            .iter()
            .map(|&s| s.to_string())
            .chain(osm_header)
            .collect();
        wtr.serialize(v).unwrap();
    } else {
        wtr.serialize(default_header).unwrap();
    }

    for sp in stop_points {
        let mut csv_row = vec![
            format!("StopPoint:{}", sp.id),
            sp.coord.lat.to_string(),
            sp.coord.lon.to_string(),
            sp.name.to_string(),
            format!("{:?}", sp.stop_point_type),
        ];
        if all_tags {
            csv_row = csv_row
                .into_iter()
                .chain(
                    osm_tag_list
                        .iter()
                        .map(|k| sp.all_osm_tags.get_default_string(k.as_str())),
                )
                .collect();
        }
        wtr.serialize(csv_row).unwrap();
    }
}

pub fn write_station_accesses_to_csv<P: AsRef<Path>>(
    station_accesses: &[StationAccess],
    output_dir: P,
    all_tags: bool,
) {
    let output_dir = output_dir.as_ref();
    let csv_file = output_dir.join("osm-transit-extractor_station_accesses.csv");

    let mut wtr = csv::Writer::from_path(csv_file).unwrap();
    let default_header = ["station_access_id", "lat", "lon", "name", "code"];
    let osm_tag_list: BTreeSet<_> = station_accesses
        .iter()
        .flat_map(|s| s.all_osm_tags.keys().map(|s| s.to_string()))
        .collect();
    if all_tags {
        let osm_header = osm_tag_list.iter().map(|s| format!("osm:{}", s));
        let v: Vec<_> = default_header
            .iter()
            .map(ToString::to_string)
            .chain(osm_header)
            .collect();
        wtr.serialize(v).unwrap();
    } else {
        wtr.serialize(default_header).unwrap();
    }

    for access in station_accesses {
        let mut csv_row = vec![
            format!("StationAccess:{}", access.id),
            access.coord.lat.to_string(),
            access.coord.lon.to_string(),
            access.name.to_string(),
            access.code.to_string(),
        ];
        if all_tags {
            csv_row = csv_row
                .into_iter()
                .chain(
                    osm_tag_list
                        .iter()
                        .map(|k| access.all_osm_tags.get_default_string(&k)),
                )
                .collect();
        }
        wtr.serialize(csv_row).unwrap();
    }
}

pub fn write_stop_areas_stop_point_to_csv<P: AsRef<Path>>(stop_areas: &[StopArea], output_dir: P) {
    let output_dir = output_dir.as_ref();
    let csv_file = output_dir.join("osm-transit-extractor_stop_areas_stop_point.csv");

    let mut wtr = csv::Writer::from_path(csv_file).unwrap();
    let default_header = ["stop_area_id", "stop_point_id"];
    wtr.serialize(default_header).unwrap();
    for sa in stop_areas {
        for sp_id in &sa.stop_point_ids {
            let csv_row = vec![
                format!("StopArea:{}", sa.id),
                format!("StopPoint:{}", sp_id),
            ];
            wtr.serialize(csv_row).unwrap();
        }
    }
}

pub fn write_stop_areas_station_accesses_to_csv<P: AsRef<Path>>(
    stop_areas: &[StopArea],
    output_dir: P,
) {
    let output_dir = output_dir.as_ref();
    let csv_file = output_dir.join("osm-transit-extractor_stop_areas_station_accesses.csv");

    let mut wtr = csv::Writer::from_path(csv_file).unwrap();
    let default_header = ["stop_area_id", "station_access_id"];
    wtr.serialize(default_header).unwrap();
    for sa in stop_areas {
        for access_id in &sa.station_access_ids {
            let csv_row = vec![
                format!("StopArea:{}", sa.id),
                format!("StationAccess:{}", access_id),
            ];
            wtr.serialize(csv_row).unwrap();
        }
    }
}

pub fn write_stop_areas_to_csv<P: AsRef<Path>>(
    stop_areas: &[StopArea],
    output_dir: P,
    all_tags: bool,
) {
    let output_dir = output_dir.as_ref();
    let csv_file = output_dir.join("osm-transit-extractor_stop_areas.csv");

    let mut wtr = csv::Writer::from_path(csv_file).unwrap();
    let osm_tag_list: BTreeSet<String> = stop_areas
        .iter()
        .flat_map(|s| s.all_osm_tags.keys().map(|s| s.to_string()))
        .collect();
    let default_header = ["stop_area_id", "lat", "lon", "name"];
    if all_tags {
        let osm_header = osm_tag_list.iter().map(|s| format!("osm:{}", s));
        let v: Vec<_> = default_header
            .iter()
            .map(|&s| s.to_string())
            .chain(osm_header)
            .collect();
        wtr.serialize(v).unwrap();
    } else {
        wtr.serialize(default_header).unwrap();
    }

    for sa in stop_areas {
        let mut csv_row = vec![
            format!("StopArea:{}", sa.id),
            sa.coord.lat.to_string(),
            sa.coord.lon.to_string(),
            sa.name.to_string(),
        ];
        if all_tags {
            csv_row = csv_row
                .into_iter()
                .chain(
                    osm_tag_list
                        .iter()
                        .map(|k| sa.all_osm_tags.get_default_string(k.as_str())),
                )
                .collect();
        }
        wtr.serialize(csv_row).unwrap();
    }
}
pub fn write_routes_to_csv<P: AsRef<Path>>(routes: Vec<Route>, output_dir: P, all_tags: bool) {
    let output_dir = output_dir.as_ref();
    let csv_route_file = output_dir.join("osm-transit-extractor_routes.csv");
    let csv_route_points_file = output_dir.join("osm-transit-extractor_route_points.csv");
    let mut wtr_route = csv::Writer::from_path(csv_route_file).unwrap();
    let mut wtr_route_points = csv::Writer::from_path(csv_route_points_file).unwrap();
    let osm_tag_list: BTreeSet<String> = routes
        .iter()
        .flat_map(|r| r.all_osm_tags.keys().map(|s| s.to_string()))
        .collect();
    let osm_header = osm_tag_list.iter().map(|s| format!("osm:{}", s));
    wtr_route_points
        .serialize(("route_id", "role", "stop_id"))
        .unwrap();
    let default_header = [
        "route_id",
        "name",
        "code",
        "destination",
        "origin",
        "colour",
        "operator",
        "network",
        "mode",
        "frequency",
        "opening_hours",
        "frequency_exceptions",
        "travel_time",
        "shape",
    ];
    if all_tags {
        let v: Vec<_> = default_header
            .iter()
            .map(|&s| s.to_string())
            .chain(osm_header)
            .collect();
        wtr_route.serialize(v).unwrap();
    } else {
        wtr_route.serialize(default_header).unwrap();
    }

    for r in &routes {
        // writing route csv
        let mut csv_row = vec![
            format!("Route:{}", r.id),
            r.name.to_string(),
            r.code.to_string(),
            r.destination.to_string(),
            r.origin.to_string(),
            r.colour.to_string(),
            r.operator.to_string(),
            r.network.to_string(),
            r.mode.to_string(),
            r.frequency.to_string(),
            r.opening_hours.to_string(),
            r.frequency_exceptions.to_string(),
            r.travel_time.to_string(),
            shape_to_wkt(&r.shape),
        ];
        if all_tags {
            csv_row = csv_row
                .into_iter()
                .chain(
                    osm_tag_list
                        .iter()
                        .map(|k| r.all_osm_tags.get_default_string(k.as_str())),
                )
                .collect();
        }
        wtr_route.serialize(csv_row).unwrap();

        //writing the route_points csv
        for rp in &r.ordered_route_points {
            let row = vec![
                format!("Route:{}", r.id),
                rp.role.to_string(),
                format!("StopPoint:{}", rp.stop_point_id),
            ];
            wtr_route_points.write_record(row.into_iter()).unwrap();
        }
    }
}

pub fn write_lines_to_csv<P: AsRef<Path>>(lines: Vec<Line>, output_dir: P, all_tags: bool) {
    let output_dir = output_dir.as_ref();
    let lines_csv_file = output_dir.join("osm-transit-extractor_lines.csv");
    let mut lines_wtr = csv::Writer::from_path(lines_csv_file).unwrap();
    let osm_tag_list: BTreeSet<String> = lines
        .iter()
        .flat_map(|r| r.all_osm_tags.keys().map(|s| s.to_string()))
        .collect();
    let default_header = [
        "line_id",
        "name",
        "code",
        "colour",
        "operator",
        "network",
        "mode",
        "frequency",
        "opening_hours",
        "frequency_exceptions",
        "shape",
    ];
    if all_tags {
        let osm_header = osm_tag_list.iter().map(|s| format!("osm:{}", s));
        let v: Vec<_> = default_header
            .iter()
            .map(|&s| s.to_string())
            .chain(osm_header)
            .collect();
        lines_wtr.serialize(v).unwrap();
    } else {
        lines_wtr.serialize(default_header).unwrap();
    }

    let csv_file = output_dir.join("osm-transit-extractor_line_routes.csv");
    let mut wtr = csv::Writer::from_path(csv_file).unwrap();
    wtr.serialize(("line_id", "route_id")).unwrap();

    for l in &lines {
        // writing lines csv
        let mut csv_row = vec![
            format!("Line:{}", l.id),
            l.name.to_string(),
            l.code.to_string(),
            l.colour.to_string(),
            l.operator.to_string(),
            l.network.to_string(),
            l.mode.to_string(),
            l.frequency.to_string(),
            l.opening_hours.to_string(),
            l.frequency_exceptions.to_string(),
            shape_to_wkt(&l.shape),
        ];
        if all_tags {
            csv_row = csv_row
                .into_iter()
                .chain(
                    osm_tag_list
                        .iter()
                        .map(|k| l.all_osm_tags.get_default_string(k.as_str())),
                )
                .collect();
        }
        lines_wtr.serialize(csv_row).unwrap();

        //Writing line-route csv file
        for r in &l.routes_id {
            wtr.serialize((format!("Line:{}", &l.id), format!("Route:{}", &r)))
                .unwrap();
        }
    }
}
