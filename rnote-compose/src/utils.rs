use p2d::bounding_volume::AABB;
use piet::RenderContext;
use rand::{Rng, SeedableRng};

const XML_HEADER_REGEX: &str = r#"<\?xml[^\?>]*\?>"#;
const SVG_ROOT_REGEX: &str = r#"<svg[^>]*>|<[^/svg]*/svg>"#;

/// Check if a xml header is present
pub fn check_xml_header(svg: &str) -> bool {
    let re = regex::Regex::new(XML_HEADER_REGEX).unwrap();
    re.is_match(svg)
}

/// Adds a xml header to the &str
pub fn add_xml_header(svg: &str) -> String {
    let re = regex::Regex::new(XML_HEADER_REGEX).unwrap();
    if !re.is_match(svg) {
        String::from(r#"<?xml version="1.0" standalone="no"?>"#) + "\n" + svg
    } else {
        String::from(svg)
    }
}

/// Removes the xml header from the &str, if present
pub fn remove_xml_header(svg: &str) -> String {
    let re = regex::Regex::new(XML_HEADER_REGEX).unwrap();
    String::from(re.replace_all(svg, ""))
}

/// Checks is a svg root is present
pub fn check_svg_root(svg: &str) -> bool {
    let re = regex::Regex::new(SVG_ROOT_REGEX).unwrap();
    re.is_match(svg)
}

/// Wraps a svg str with a svg root element and its attributes
pub fn wrap_svg_root(
    data: &str,
    bounds: Option<AABB>,
    viewbox: Option<AABB>,
    preserve_aspectratio: bool,
) -> String {
    let (x, y, width, height) = if let Some(bounds) = bounds {
        let x = format!("{:.3}", bounds.mins[0]);
        let y = format!("{:.3}", bounds.mins[1]);
        let width = format!("{:.3}", bounds.extents()[0]);
        let height = format!("{:.3}", bounds.extents()[1]);

        (x, y, width, height)
    } else {
        (
            String::from("0"),
            String::from("0"),
            String::from("100%"),
            String::from("100%"),
        )
    };

    let viewbox = if let Some(viewbox) = viewbox {
        format!(
            "{:.3} {:.3} {:.3} {:.3}",
            viewbox.mins[0],
            viewbox.mins[1],
            viewbox.extents()[0],
            viewbox.extents()[1]
        )
    } else {
        String::from("")
    };
    let preserve_aspectratio = if preserve_aspectratio {
        String::from("xMidyMid")
    } else {
        String::from("none")
    };

    let svg_root = svg::node::element::SVG::new()
        .set("xmlns", "http://www.w3.org/2000/svg")
        .set("xmlns:svg", "http://www.w3.org/2000/svg")
        .set("xmlns:xlink", "http://www.w3.org/1999/xlink")
        .set("x", x.as_str())
        .set("y", y.as_str())
        .set("width", width.as_str())
        .set("height", height.as_str())
        .set("viewBox", viewbox.as_str())
        .set("preserveAspectRatio", preserve_aspectratio.as_str())
        .add(svg::node::Text::new(data));

    // unwrapping because we know its a valid Svg
    svg_node_to_string(&svg_root).unwrap()
}

/// Strips the svg root element, if present
pub fn strip_svg_root(svg: &str) -> String {
    let re = regex::Regex::new(SVG_ROOT_REGEX).unwrap();
    String::from(re.replace_all(svg, ""))
}

/// Converting a svg::Node to a String
pub fn svg_node_to_string<N>(node: &N) -> Result<String, anyhow::Error>
where
    N: svg::Node,
{
    let mut document_buffer = Vec::<u8>::new();
    svg::write(&mut document_buffer, node)?;
    Ok(String::from_utf8(document_buffer)?)
}

/// A new random number generator with the pcg64 algorithm. Used for seedable, reproducable random numbers.
pub fn new_rng_default_pcg64(seed: Option<u64>) -> rand_pcg::Pcg64 {
    if let Some(seed) = seed {
        rand_pcg::Pcg64::seed_from_u64(seed)
    } else {
        rand_pcg::Pcg64::from_entropy()
    }
}

/// generates a alphanumeric random prefix for svg ids. used to avoid id collisions.
pub fn random_id_prefix() -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>()
}

/// returns a new seed by generating a random value seeded from the old seed
pub fn seed_advance(seed: u64) -> u64 {
    let mut rng = rand_pcg::Pcg64::seed_from_u64(seed);
    rng.gen()
}

/// Generates a svg string from the piet context
pub fn piet_svg_cx_to_svg(mut cx: piet_svg::RenderContext) -> Result<String, anyhow::Error> {
    let mut data: Vec<u8> = vec![];
    cx.write(&mut data)?;

    cx.finish()
        .map_err(|e| anyhow::anyhow!("cx.finish() failed in svg_cx_to_svg() with Err {}", e))?;

    let svg_data = crate::utils::strip_svg_root(String::from_utf8(data)?.as_str());

    Ok(svg_data)
}
