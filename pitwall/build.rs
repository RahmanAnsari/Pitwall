use std::fs;
use std::path::Path;

// Sector colors (F1 broadcast style)
const SECTOR1_COLOR: &str = "#e10600"; // red
const SECTOR2_COLOR: &str = "#00d2ff"; // cyan
const SECTOR3_COLOR: &str = "#eab308"; // yellow
const FALLBACK_COLOR: &str = "#ffffff"; // white (no sector data)

fn main() {
    let geojson_dir = Path::new("circuits/geojson");
    let svg_dir = Path::new("circuits/svg");

    if !geojson_dir.exists() {
        return;
    }

    fs::create_dir_all(svg_dir).expect("failed to create circuits/svg");
    println!("cargo::rerun-if-changed=circuits/geojson");

    for entry in fs::read_dir(geojson_dir).expect("failed to read geojson dir") {
        let entry = entry.expect("failed to read dir entry");
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("geojson") {
            continue;
        }

        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        let contents = fs::read_to_string(&path).expect("failed to read geojson file");

        let svg = geojson_to_svg(&contents);
        let out_path = svg_dir.join(format!("{}.svg", stem));
        fs::write(&out_path, svg).expect("failed to write svg");
    }
}

fn geojson_to_svg(json_str: &str) -> String {
    let coords = extract_linestring_coordinates(json_str);

    if coords.is_empty() {
        return String::from("<svg xmlns=\"http://www.w3.org/2000/svg\"/>");
    }

    // Extract sector split points
    let start_finish = extract_point_feature(json_str, "Start/Finish Line");
    let sector1_split = extract_point_feature(json_str, "Sector 1 Split");
    let sector2_split = extract_point_feature(json_str, "Sector 2 Split");

    // Compute normalized coordinates
    let lngs: Vec<f64> = coords.iter().map(|(lng, _)| *lng).collect();
    let lats: Vec<f64> = coords.iter().map(|(_, lat)| *lat).collect();

    let min_lng = lngs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_lng = lngs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_lat = lats.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_lat = lats.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let mid_lat_rad = ((min_lat + max_lat) / 2.0).to_radians();
    let cos_lat = mid_lat_rad.cos();

    let width = (max_lng - min_lng) * cos_lat;
    let height = max_lat - min_lat;
    let scale = width.max(height);

    if scale == 0.0 {
        return String::from("<svg xmlns=\"http://www.w3.org/2000/svg\"/>");
    }

    // Add padding so the thick stroke doesn't clip at edges
    // Must be at least half the thickest stroke width (0.028/2 = 0.014) plus margin
    let padding = 0.04;
    let svg_w = width / scale + padding * 2.0;
    let svg_h = height / scale + padding * 2.0;

    // If the track is taller than wide, rotate by swapping x/y coordinates
    let landscape = svg_h > svg_w;
    let (final_w, final_h) = if landscape { (svg_h, svg_w) } else { (svg_w, svg_h) };

    let normalize = |lng: f64, lat: f64| -> (f64, f64) {
        let x = (lng - min_lng) * cos_lat / scale + padding;
        let y = (max_lat - lat) / scale + padding;
        if landscape {
            // Rotate 90°: new_x = old_y, new_y = svg_w - old_x
            (y, svg_w - x)
        } else {
            (x, y)
        }
    };

    let normalized: Vec<(f64, f64)> = coords.iter().map(|(lng, lat)| normalize(*lng, *lat)).collect();

    // Generate smooth SVG path for the full track (use lower tension for closed loop)
    let full_path_d = smooth_path_with_tension(&normalized, true, 0.15);

    // Build sector paths or fallback
    let mut start_finish_marker = String::new();
    let sector_paths = if let (Some(sf), Some(s1), Some(s2)) = (start_finish, sector1_split, sector2_split) {
        let sf_idx = find_closest_index(&coords, sf);
        let s1_idx = find_closest_index(&coords, s1);
        let s2_idx = find_closest_index(&coords, s2);

        // Get the normalized start/finish position for the marker
        let sf_pos = normalized[sf_idx];
        start_finish_marker = format_checkered_flag(sf_pos.0, sf_pos.1);

        let len = normalized.len();
        let fwd_dist = if s1_idx >= sf_idx { s1_idx - sf_idx } else { len - sf_idx + s1_idx };
        let bwd_dist = if sf_idx >= s1_idx { sf_idx - s1_idx } else { len - s1_idx + sf_idx };

        let (sector1_points, sector2_points, sector3_points) = if fwd_dist <= bwd_dist {
            (
                get_wrapped_slice_fwd(&normalized, sf_idx, s1_idx),
                get_wrapped_slice_fwd(&normalized, s1_idx, s2_idx),
                get_wrapped_slice_fwd(&normalized, s2_idx, sf_idx),
            )
        } else {
            (
                get_wrapped_slice_bwd(&normalized, sf_idx, s1_idx),
                get_wrapped_slice_bwd(&normalized, s1_idx, s2_idx),
                get_wrapped_slice_bwd(&normalized, s2_idx, sf_idx),
            )
        };

        vec![
            format_smooth_path(&sector1_points, SECTOR1_COLOR),
            format_smooth_path(&sector2_points, SECTOR2_COLOR),
            format_smooth_path(&sector3_points, SECTOR3_COLOR),
        ]
    } else {
        vec![format_smooth_path(&normalized, FALLBACK_COLOR)]
    };

    // Layer 1: Dark thick track surface (gives road width)
    // Layer 2: Slightly less thick dark border
    // Layer 3: Colored sector lines on top
    let track_bg = format!(
        "  <path d=\"{}\" stroke=\"#3d3d5c\" stroke-width=\"0.028\" />",
        full_path_d
    );
    let track_border = format!(
        "  <path d=\"{}\" stroke=\"#2a2a40\" stroke-width=\"0.020\" />",
        full_path_d
    );

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w:.5} {h:.5}\" fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n{bg}\n{border}\n{sectors}\n{marker}\n</svg>",
        w = final_w,
        h = final_h,
        bg = track_bg,
        border = track_border,
        sectors = sector_paths.join("\n"),
        marker = start_finish_marker
    )
}

fn format_smooth_path(points: &[(f64, f64)], color: &str) -> String {
    let d = smooth_path_with_tension(points, false, 0.25);
    format!("  <path d=\"{}\" stroke=\"{}\" stroke-width=\"0.012\" />", d, color)
}

/// Generate a small checkered flag icon at the given position.
/// Positioned above the track line (offset upward).
fn format_checkered_flag(cx: f64, cy: f64) -> String {
    // 4x4 checkered pattern, offset above the point
    let size = 0.020;
    let cell = size / 4.0;
    let x0 = cx - size / 2.0;
    let y0 = cy - size - 0.018; // move above the line (negative Y = up in SVG)

    let mut rects = String::new();
    for row in 0..4 {
        for col in 0..4 {
            let is_dark = (row + col) % 2 == 0;
            let fill = if is_dark { "#ffffff" } else { "#000000" };
            let rx = x0 + col as f64 * cell;
            let ry = y0 + row as f64 * cell;
            rects.push_str(&format!(
                "    <rect x=\"{:.5}\" y=\"{:.5}\" width=\"{:.5}\" height=\"{:.5}\" fill=\"{}\" />\n",
                rx, ry, cell, cell, fill
            ));
        }
    }

    format!("  <g>\n{}</g>", rects)
}

/// Generate a smooth SVG path using Catmull-Rom to cubic Bezier conversion.
/// If `closed` is true, the path wraps smoothly at the endpoints.
fn smooth_path_with_tension(points: &[(f64, f64)], closed: bool, tension: f64) -> String {
    let n = points.len();
    if n < 2 {
        return String::new();
    }
    if n == 2 {
        return format!("M{:.5},{:.5} L{:.5},{:.5}", points[0].0, points[0].1, points[1].0, points[1].1);
    }

    let mut d = format!("M{:.5},{:.5}", points[0].0, points[0].1);

    for i in 0..n - 1 {
        let p0 = if i == 0 {
            if closed { points[n - 2] } else { points[0] }
        } else {
            points[i - 1]
        };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 >= n {
            if closed { points[(i + 2) % n] } else { points[n - 1] }
        } else {
            points[i + 2]
        };

        // Catmull-Rom to cubic bezier control points
        let cp1x = p1.0 + (p2.0 - p0.0) * tension / 3.0;
        let cp1y = p1.1 + (p2.1 - p0.1) * tension / 3.0;
        let cp2x = p2.0 - (p3.0 - p1.0) * tension / 3.0;
        let cp2y = p2.1 - (p3.1 - p1.1) * tension / 3.0;

        d.push_str(&format!(
            " C{:.5},{:.5} {:.5},{:.5} {:.5},{:.5}",
            cp1x, cp1y, cp2x, cp2y, p2.0, p2.1
        ));
    }

    d
}

/// Get a slice of points going forward (increasing indices), wrapping around if needed.
fn get_wrapped_slice_fwd(points: &[(f64, f64)], start: usize, end: usize) -> Vec<(f64, f64)> {
    let len = points.len();
    let mut result = Vec::new();
    let mut i = start;
    loop {
        result.push(points[i % len]);
        if i % len == end % len {
            break;
        }
        i += 1;
    }
    result
}

/// Get a slice of points going backward (decreasing indices), wrapping around if needed.
fn get_wrapped_slice_bwd(points: &[(f64, f64)], start: usize, end: usize) -> Vec<(f64, f64)> {
    let len = points.len();
    let mut result = Vec::new();
    let mut i = start;
    loop {
        result.push(points[i % len]);
        if i % len == end % len {
            break;
        }
        i = if i == 0 { len - 1 } else { i - 1 };
    }
    result
}

/// Find the index of the coordinate closest to the given point.
fn find_closest_index(coords: &[(f64, f64)], target: (f64, f64)) -> usize {
    let mut best_idx = 0;
    let mut best_dist = f64::INFINITY;

    for (i, (lng, lat)) in coords.iter().enumerate() {
        let d = (lng - target.0).powi(2) + (lat - target.1).powi(2);
        if d < best_dist {
            best_dist = d;
            best_idx = i;
        }
    }

    best_idx
}

/// Extract a Point feature's coordinates by matching a substring in its properties.
fn extract_point_feature(json_str: &str, name: &str) -> Option<(f64, f64)> {
    // Find the feature containing this name
    let search = format!("\"{}\"", name);
    let name_pos = json_str.find(&search)?;

    // Find "coordinates" after this name (within the same feature)
    let rest = &json_str[name_pos..];
    let coord_marker = "\"coordinates\"";
    let coord_pos = rest.find(coord_marker)?;
    let after_coord = &rest[coord_pos + coord_marker.len()..];

    // Find the [ that starts the coordinate pair
    let bracket_pos = after_coord.find('[')?;
    let after_bracket = &after_coord[bracket_pos + 1..];

    // Find the closing ]
    let end_bracket = after_bracket.find(']')?;
    let pair_str = &after_bracket[..end_bracket];

    let parts: Vec<&str> = pair_str.split(',').collect();
    if parts.len() >= 2 {
        let lng = parts[0].trim().parse::<f64>().ok()?;
        let lat = parts[1].trim().parse::<f64>().ok()?;
        Some((lng, lat))
    } else {
        None
    }
}

/// Extract the first LineString coordinates from the GeoJSON.
fn extract_linestring_coordinates(json_str: &str) -> Vec<(f64, f64)> {
    let mut coords = Vec::new();

    // Find first "LineString" to ensure we get the track, not a point
    let ls_marker = "\"LineString\"";
    let ls_pos = match json_str.find(ls_marker) {
        Some(i) => i,
        None => return coords,
    };

    // Find "coordinates" after the LineString type
    let rest = &json_str[ls_pos..];
    let coord_marker = "\"coordinates\"";
    let coord_pos = match rest.find(coord_marker) {
        Some(i) => i + coord_marker.len(),
        None => return coords,
    };

    let after_coord = &rest[coord_pos..];

    // Find the outer opening '['
    let outer_start = match after_coord.find('[') {
        Some(i) => i,
        None => return coords,
    };

    let arr_str = &after_coord[outer_start..];
    let bytes = arr_str.as_bytes();
    let mut i = 1; // skip outer '['

    while i < bytes.len() {
        if bytes[i] == b'[' {
            let inner_start = i + 1;
            let mut end = inner_start;
            while end < bytes.len() && bytes[end] != b']' {
                end += 1;
            }
            if end >= bytes.len() {
                break;
            }
            let pair_str = &arr_str[inner_start..end];
            let parts: Vec<&str> = pair_str.split(',').collect();
            if parts.len() >= 2 {
                if let (Ok(lng), Ok(lat)) = (
                    parts[0].trim().parse::<f64>(),
                    parts[1].trim().parse::<f64>(),
                ) {
                    coords.push((lng, lat));
                }
            }
            i = end + 1;
        } else if bytes[i] == b']' {
            break;
        } else {
            i += 1;
        }
    }

    coords
}
