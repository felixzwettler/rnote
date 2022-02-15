use crate::compose::smooth::SmoothOptions;
use crate::compose::{self, curves, geometry, smooth};
use crate::{
    drawbehaviour::DrawBehaviour, pens::marker::Marker, render, strokes::strokestyle::Element,
};
use p2d::bounding_volume::{BoundingVolume, AABB};
use rand::{Rng, SeedableRng};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use svg::node::element::path;

use crate::compose::transformable::Transformable;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename = "markerstroke")]
pub struct MarkerStroke {
    #[serde(rename = "elements")]
    pub elements: Vec<Element>,
    #[serde(rename = "marker")]
    pub options: SmoothOptions,
    #[serde(rename = "bounds")]
    pub bounds: AABB,
    #[serde(skip)]
    pub hitbox: Vec<AABB>,
}

impl Default for MarkerStroke {
    fn default() -> Self {
        Self {
            elements: vec![],
            options: SmoothOptions::default(),
            bounds: geometry::aabb_new_zero(),
            hitbox: vec![],
        }
    }
}

impl DrawBehaviour for MarkerStroke {
    fn bounds(&self) -> AABB {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: AABB) {
        self.bounds = bounds;
    }

    fn gen_bounds(&self) -> Option<AABB> {
        if let Some(&first) = self.elements.iter().peekable().peek() {
            let mut bounds = AABB::new_invalid();
            bounds.take_point(na::Point2::from(first.inputdata.pos()));

            bounds.merge(
                &self
                    .elements
                    .par_iter()
                    .zip(self.elements.par_iter().skip(1))
                    .zip(self.elements.par_iter().skip(2))
                    .zip(self.elements.par_iter().skip(3))
                    .filter_map(|(((first, second), third), forth)| {
                        let width = self.options.width;

                        let mut bounds = AABB::new_invalid();

                        if let Some(cubbez) = curves::gen_cubbez_w_catmull_rom(
                            first.inputdata.pos(),
                            second.inputdata.pos(),
                            third.inputdata.pos(),
                            forth.inputdata.pos(),
                        ) {
                            // Bounds are definitely inside the polygon of the control points. (Could be improved with the second derivative of the bezier curve)
                            bounds.take_point(na::Point2::from(cubbez.start));
                            bounds.take_point(na::Point2::from(cubbez.cp1));
                            bounds.take_point(na::Point2::from(cubbez.cp2));
                            bounds.take_point(na::Point2::from(cubbez.end));
                        } else if let Some(line) =
                            curves::gen_line(second.inputdata.pos(), third.inputdata.pos())
                        {
                            bounds.take_point(na::Point2::from(line.start));
                            bounds.take_point(na::Point2::from(line.end));
                        } else {
                            return None;
                        }

                        bounds.loosen(width + 1.0);

                        Some(bounds)
                    })
                    .reduce(AABB::new_invalid, |i, next| i.merged(&next)),
            );
            Some(bounds)
        } else {
            None
        }
    }

    fn gen_svgs(&self, offset: na::Vector2<f64>) -> Result<Vec<render::Svg>, anyhow::Error> {
        let svg_root = false;

        let svgs: Vec<render::Svg> = self
            .elements
            .iter()
            .zip(self.elements.iter().skip(1))
            .zip(self.elements.iter().skip(2))
            .zip(self.elements.iter().skip(3))
            .filter_map(|(((first, second), third), forth)| {
                Self::gen_svg_elem(
                    &self.options,
                    (first, second, third, forth),
                    offset,
                    svg_root,
                )
            })
            .collect();

        Ok(svgs)
    }
}

impl Transformable for MarkerStroke {
    fn translate(&mut self, offset: nalgebra::Vector2<f64>) {
        self.elements.iter_mut().for_each(|element| {
            element.inputdata.set_pos(element.inputdata.pos() + offset);
        });
        self.update_geometry();
    }
    fn rotate(&mut self, angle: f64, center: nalgebra::Point2<f64>) {
        let mut isometry = na::Isometry2::identity();
        isometry.append_rotation_wrt_point_mut(&na::UnitComplex::new(angle), &center);

        self.elements.iter_mut().for_each(|element| {
            element
                .inputdata
                .set_pos((isometry * na::Point2::from(element.inputdata.pos())).coords);
        });
        self.update_geometry();
    }
    fn scale(&mut self, scale: nalgebra::Vector2<f64>) {
        let center = self.bounds.center().coords;

        self.elements.iter_mut().for_each(|element| {
            element
                .inputdata
                .set_pos(((element.inputdata.pos() - center).component_mul(&scale)) + center);
        });
        self.update_geometry();
    }
}

impl MarkerStroke {
    pub const HITBOX_DEFAULT: f64 = 10.0;

    pub fn new(element: Element, marker: &Marker) -> Self {
        let seed = Some(rand_pcg::Pcg64::from_entropy().gen());

        let elements = Vec::with_capacity(4);
        let bounds = AABB::new(
            na::point![element.inputdata.pos()[0], element.inputdata.pos()[1]],
            na::point![element.inputdata.pos()[0], element.inputdata.pos()[1]],
        );
        let hitbox: Vec<AABB> = Vec::new();

        let mut options = marker.options;
        options.seed = seed;

        let mut markerstroke = Self {
            elements,
            options,
            bounds,
            hitbox,
        };

        // Pushing with push_elem() instead pushing vector, because bounds are getting updated there too
        markerstroke.push_elem(element);

        markerstroke
    }

    pub fn new_w_elements(
        mut elements: impl Iterator<Item = Element>,
        marker: &Marker,
    ) -> Option<Self> {
        if let Some(first) = elements.next() {
            let mut markerstroke = Self::new(first, marker);

            for element in elements {
                markerstroke.elements.push(element);
            }
            markerstroke.update_geometry();

            Some(markerstroke)
        } else {
            None
        }
    }

    pub fn push_elem(&mut self, element: Element) {
        self.elements.push(element);

        self.update_bounds_to_last_elem();
    }

    pub fn pop_elem(&mut self) -> Option<Element> {
        let element = self.elements.pop();

        self.update_geometry();
        element
    }

    pub fn update_geometry(&mut self) {
        if let Some(new_bounds) = self.gen_bounds() {
            self.set_bounds(new_bounds);
        }
        self.hitbox = self.gen_hitbox();
    }

    fn update_bounds_to_last_elem(&mut self) {
        // Making sure bounds are always outside of coord + width
        if let Some(last) = self.elements.last() {
            self.bounds.merge(&AABB::new(
                na::Point2::from(
                    last.inputdata.pos() - na::vector![self.options.width, self.options.width],
                ),
                na::Point2::from(
                    last.inputdata.pos() + na::vector![self.options.width, self.options.width],
                ),
            ));
        }
    }

    fn gen_hitbox(&self) -> Vec<AABB> {
        let mut hitbox: Vec<AABB> = Vec::with_capacity(self.elements.len() as usize);
        let mut elements_iter = self.elements.iter().peekable();
        while let Some(first) = elements_iter.next() {
            let second = if let Some(&second) = elements_iter.peek() {
                Some(second)
            } else {
                None
            };

            hitbox.push(self.gen_hitbox_for_elems(first, second));
        }

        hitbox
    }

    fn gen_hitbox_for_elems(&self, first: &Element, second: Option<&Element>) -> AABB {
        let width = self.options.width;

        let first = first.inputdata.pos();
        if let Some(second) = second {
            let second = second.inputdata.pos();

            let delta = second - first;
            let marker_x = if delta[0] < 0.0 { -width } else { width };
            let marker_y = if delta[1] < 0.0 { -width } else { width };

            geometry::aabb_new_positive(
                na::Point2::from(first - na::vector![marker_x / 2.0, marker_y / 2.0]),
                na::Point2::from(first + delta + na::vector![marker_x / 2.0, marker_y / 2.0]),
            )
        } else {
            geometry::aabb_new_positive(
                na::Point2::from(
                    first
                        - na::vector![
                            (Self::HITBOX_DEFAULT + width) / 2.0,
                            (Self::HITBOX_DEFAULT + width / 2.0)
                        ],
                ),
                na::Point2::from(
                    first + na::vector![Self::HITBOX_DEFAULT + width, Self::HITBOX_DEFAULT + width],
                ),
            )
        }
    }

    pub fn gen_svg_elem(
        options: &SmoothOptions,
        elements: (&Element, &Element, &Element, &Element),
        offset: na::Vector2<f64>,
        svg_root: bool,
    ) -> Option<render::Svg> {
        let mut commands = Vec::new();
        let width = options.width;

        let mut bounds = AABB::new_invalid();

        if let Some(mut cubbez) = curves::gen_cubbez_w_catmull_rom(
            elements.0.inputdata.pos(),
            elements.1.inputdata.pos(),
            elements.2.inputdata.pos(),
            elements.3.inputdata.pos(),
        ) {
            cubbez.start += offset;
            cubbez.cp1 += offset;
            cubbez.cp2 += offset;
            cubbez.end += offset;

            // Bounds are definitely inside the polygon of the control points. (Could be improved with the second derivative of the bezier curve)
            bounds.take_point(na::Point2::from(cubbez.start));
            bounds.take_point(na::Point2::from(cubbez.cp1));
            bounds.take_point(na::Point2::from(cubbez.cp2));
            bounds.take_point(na::Point2::from(cubbez.end));

            commands.append(&mut smooth::compose_cubbez(cubbez, true, options));
        } else if let Some(mut line) =
            curves::gen_line(elements.1.inputdata.pos(), elements.2.inputdata.pos())
        {
            line.start += offset;
            line.end += offset;

            bounds.take_point(na::Point2::from(line.start));
            bounds.take_point(na::Point2::from(line.end));

            commands.append(&mut smooth::compose_line(line, true, options));
        } else {
            return None;
        }

        bounds.loosen(width + 1.0);

        let color = options
            .stroke_color
            .map_or(String::from(""), |color| color.to_css_color());

        let path = svg::node::element::Path::new()
            .set("stroke", color)
            .set("stroke-width", width)
            .set("stroke-linejoin", "round")
            .set("stroke-linecap", "round")
            .set("fill", "none")
            .set("d", path::Data::from(commands));

        let mut svg_data = compose::svg_node_to_string(&path)
            .map_err(|e| {
                anyhow::anyhow!(
                    "node_to_string() failed in gen_svg_elem() of markerstroke with Err `{}`",
                    e
                )
            })
            .ok()?;

        if svg_root {
            svg_data = compose::wrap_svg_root(&svg_data, Some(bounds), Some(bounds), true);
        }
        Some(render::Svg { svg_data, bounds })
    }
}
