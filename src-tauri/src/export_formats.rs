// GeoJSON + KML export — Sprint 17.
//
// Converts Shapefile features to GeoJSON and KML for web map handoff
// and Google Earth visualization. These are the #2 and #3 most-requested
// export formats after Shapefile.
//
// GeoJSON: RFC 7946 — straightforward JSON with geometry + properties.
// KML: OGC KML 2.2 — XML with Placemark elements for each feature.

use crate::formats::shapefile::{Shape, Shapefile, ShapefileFeature};
use std::fs;
use std::path::Path;

/// Export a Shapefile's features as a GeoJSON FeatureCollection.
pub fn export_geojson(shp: &Shapefile, output_path: &Path) -> Result<(), String> {
    let features: Vec<String> = shp.features.iter().map(|f| feature_to_geojson(f)).collect();
    let json = format!(
        r#"{{"type":"FeatureCollection","features":[{}]}}"#,
        features.join(",")
    );
    fs::write(output_path, json).map_err(|e| format!("writing GeoJSON: {e}"))
}

fn feature_to_geojson(feature: &ShapefileFeature) -> String {
    let geometry = shape_to_geojson_geometry(&feature.geometry);
    let properties: Vec<String> = feature
        .attributes
        .iter()
        .map(|(k, v)| format!("\"{}\":\"{}\"", escape_json(k), escape_json(v)))
        .collect();
    format!(
        r#"{{"type":"Feature","geometry":{},"properties":{{{}}}}}"#,
        geometry,
        properties.join(",")
    )
}

fn shape_to_geojson_geometry(shape: &Shape) -> String {
    match shape {
        Shape::Point { x, y } => {
            format!(r#"{{"type":"Point","coordinates":[{},{}]}}"#, x, y)
        }
        Shape::MultiPoint { points } => {
            let coords: Vec<String> = points.iter().map(|p| format!("[{},{}]", p[0], p[1])).collect();
            format!(r#"{{"type":"MultiPoint","coordinates":[{}]}}"#, coords.join(","))
        }
        Shape::Polyline { parts } => {
            if parts.len() == 1 {
                // Single line → LineString
                let coords: Vec<String> = parts[0].iter().map(|p| format!("[{},{}]", p[0], p[1])).collect();
                format!(r#"{{"type":"LineString","coordinates":[{}]}}"#, coords.join(","))
            } else {
                // Multi-part → MultiLineString
                let lines: Vec<String> = parts
                    .iter()
                    .map(|part| {
                        let coords: Vec<String> = part.iter().map(|p| format!("[{},{}]", p[0], p[1])).collect();
                        format!("[{}]", coords.join(","))
                    })
                    .collect();
                format!(r#"{{"type":"MultiLineString","coordinates":[{}]}}"#, lines.join(","))
            }
        }
        Shape::Polygon { rings } => {
            if rings.len() == 1 {
                // Single ring → Polygon with one ring
                let coords: Vec<String> = rings[0].iter().map(|p| format!("[{},{}]", p[0], p[1])).collect();
                format!(r#"{{"type":"Polygon","coordinates":[[{}]]}}"#, coords.join(","))
            } else {
                // Multiple rings → Polygon with exterior + holes
                let rings_json: Vec<String> = rings
                    .iter()
                    .map(|ring| {
                        let coords: Vec<String> = ring.iter().map(|p| format!("[{},{}]", p[0], p[1])).collect();
                        format!("[{}]", coords.join(","))
                    })
                    .collect();
                format!(r#"{{"type":"Polygon","coordinates":[{}]}}"#, rings_json.join(","))
            }
        }
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Export a Shapefile's features as KML.
pub fn export_kml(shp: &Shapefile, output_path: &Path, document_name: &str) -> Result<(), String> {
    let placemarks: Vec<String> = shp
        .features
        .iter()
        .map(|f| feature_to_kml_placemark(f))
        .collect();

    let kml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <name>{}</name>
    {}
  </Document>
</kml>"#,
        escape_xml(document_name),
        placemarks.join("\n    ")
    );

    fs::write(output_path, kml).map_err(|e| format!("writing KML: {e}"))
}

fn feature_to_kml_placemark(feature: &ShapefileFeature) -> String {
    let geometry = shape_to_kml_geometry(&feature.geometry);
    let name = feature.attributes.get("NAME").or_else(|| feature.attributes.get("Name")).cloned().unwrap_or_default();
    let description: Vec<String> = feature
        .attributes
        .iter()
        .map(|(k, v)| format!("{}: {}", escape_xml(k), escape_xml(v)))
        .collect();
    format!(
        "<Placemark><name>{}</name><description><![CDATA[{}]]></description>{}</Placemark>",
        escape_xml(&name),
        description.join("<br>"),
        geometry
    )
}

fn shape_to_kml_geometry(shape: &Shape) -> String {
    match shape {
        Shape::Point { x, y } => {
            format!("<Point><coordinates>{},{}</coordinates></Point>", x, y)
        }
        Shape::MultiPoint { points } => {
            let coords: Vec<String> = points.iter().map(|p| format!("{},{}", p[0], p[1])).collect();
            format!("<MultiGeometry><Point><coordinates>{}</coordinates></Point></MultiGeometry>", coords.join(" "))
        }
        Shape::Polyline { parts } => {
            let lines: Vec<String> = parts
                .iter()
                .map(|part| {
                    let coords: Vec<String> = part.iter().map(|p| format!("{},{}", p[0], p[1])).collect();
                    format!("<LineString><coordinates>{}</coordinates></LineString>", coords.join(" "))
                })
                .collect();
            if lines.len() == 1 {
                lines[0].clone()
            } else {
                format!("<MultiGeometry>{}</MultiGeometry>", lines.join(""))
            }
        }
        Shape::Polygon { rings } => {
            let rings_xml: Vec<String> = rings
                .iter()
                .map(|ring| {
                    let coords: Vec<String> = ring.iter().map(|p| format!("{},{}", p[0], p[1])).collect();
                    format!("<LinearRing><coordinates>{}</coordinates></LinearRing>", coords.join(" "))
                })
                .collect();
            let outer = rings_xml.first().cloned().unwrap_or_default();
            let inner = &rings_xml[1..];
            if inner.is_empty() {
                format!("<Polygon><outerBoundaryIs>{}</outerBoundaryIs></Polygon>", outer)
            } else {
                let inner_xml: String = inner.iter().map(|r| format!("<innerBoundaryIs>{}</innerBoundaryIs>", r)).collect();
                format!("<Polygon><outerBoundaryIs>{}</outerBoundaryIs>{}</Polygon>", outer, inner_xml)
            }
        }
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::shapefile::{Shape, ShapefileFeature, ShapeType};
    use std::collections::HashMap;

    fn make_test_shapefile() -> Shapefile {
        let mut attrs1 = HashMap::new();
        attrs1.insert("NAME".to_string(), "Point A".to_string());
        attrs1.insert("ID".to_string(), "1".to_string());

        let mut attrs2 = HashMap::new();
        attrs2.insert("NAME".to_string(), "Line B".to_string());
        attrs2.insert("ID".to_string(), "2".to_string());

        Shapefile {
            shape_type: ShapeType::Point,
            features: vec![
                ShapefileFeature {
                    geometry: Shape::Point { x: 100.0, y: 200.0 },
                    attributes: attrs1,
                },
                ShapefileFeature {
                    geometry: Shape::Polyline {
                        parts: vec![vec![[0.0, 0.0], [10.0, 10.0], [20.0, 0.0]]],
                    },
                    attributes: attrs2,
                },
            ],
            bounds: (0.0, 0.0, 100.0, 200.0),
        }
    }

    #[test]
    fn test_export_geojson_point() {
        let shp = Shapefile {
            shape_type: ShapeType::Point,
            features: vec![ShapefileFeature {
                geometry: Shape::Point { x: 1.0, y: 2.0 },
                attributes: HashMap::new(),
            }],
            bounds: (0.0, 0.0, 1.0, 2.0),
        };
        let dir = std::env::temp_dir();
        let path = dir.join("test_export.geojson");
        export_geojson(&shp, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("FeatureCollection"));
        assert!(content.contains("Point"));
        assert!(content.contains("[1.0,2.0]"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_export_kml_point() {
        let shp = Shapefile {
            shape_type: ShapeType::Point,
            features: vec![ShapefileFeature {
                geometry: Shape::Point { x: 1.0, y: 2.0 },
                attributes: HashMap::new(),
            }],
            bounds: (0.0, 0.0, 1.0, 2.0),
        };
        let dir = std::env::temp_dir();
        let path = dir.join("test_export.kml");
        export_kml(&shp, &path, "Test KML").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("<?xml"));
        assert!(content.contains("<kml"));
        assert!(content.contains("<Point>"));
        assert!(content.contains("1.0,2.0"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_export_geojson_polygon() {
        let shp = Shapefile {
            shape_type: ShapeType::Polygon,
            features: vec![ShapefileFeature {
                geometry: Shape::Polygon {
                    rings: vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]],
                },
                attributes: HashMap::new(),
            }],
            bounds: (0.0, 0.0, 10.0, 10.0),
        };
        let dir = std::env::temp_dir();
        let path = dir.join("test_polygon.geojson");
        export_geojson(&shp, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("Polygon"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_escape_json() {
        assert_eq!(escape_json("hello"), "hello");
        assert_eq!(escape_json("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_json("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
    }
}
