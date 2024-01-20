use std::fmt::Write as _;

use crate::{ClosedPath, HORIZONTAL_FACTOR, Point, VERTICAL_FACTOR};


const SVG_NS_URI: &str = "http://www.w3.org/2000/svg";


pub(crate) fn assemble_svg(grid: Point, paths: &[ClosedPath]) -> String {
    let doc_package = sxd_document::Package::new();
    let doc = doc_package.as_document();

    let svg_elem = doc.create_element("svg");
    svg_elem.set_attribute_value("xmlns", SVG_NS_URI);
    doc.root().append_child(svg_elem);

    // use grid to determine dimensions
    let width = grid.x * HORIZONTAL_FACTOR;
    let height = grid.y * VERTICAL_FACTOR;
    svg_elem.set_attribute_value("width", &format!("{}", width));
    svg_elem.set_attribute_value("height", &format!("{}", height));

    let mut full_path_def = String::new();
    for path in paths {
        if path.points.len() == 0 {
            continue;
        }

        for (i, point) in path.points.iter().enumerate() {
            let spacing = if full_path_def.len() == 0 { "" } else { " " };
            let prefix = if i == 0 { "M" } else { "L" };
            write!(full_path_def, "{}{} {} {}", spacing, prefix, point.x, point.y).unwrap();
        }
        write!(full_path_def, " z").unwrap();
    }

    let path_elem = doc.create_element("path");
    path_elem.set_attribute_value("d", &full_path_def);
    svg_elem.append_child(path_elem);

    let mut ret = Vec::new();
    sxd_document::writer::format_document(&doc, &mut ret)
        .expect("failed to serialize XML");
    String::from_utf8(ret)
        .expect("XML serialized into something that is not UTF-8")
}
