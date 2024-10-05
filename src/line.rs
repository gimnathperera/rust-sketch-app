use geo::Simplify;

pub type Line = Vec<(f32,f32,u32)>;

pub fn simplify_line(line: &Line) -> Line {
    if line.len() < 2 {
	return line.to_vec();
    }
    let color = line[0].2;
    let linestring : geo::LineString =
	line.iter()
	.map(| (x, y, _) | (*x as f64, *y as f64 ))
	.collect();
    let linestring = linestring.simplify(&4.0);
    linestring.0.iter()
	.map(| c | (c.x as f32, c.y as f32, color))
	.collect()
}

