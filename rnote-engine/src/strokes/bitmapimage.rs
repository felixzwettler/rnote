use super::strokebehaviour::GeneratedStrokeImages;
use super::StrokeBehaviour;
use crate::render;
use crate::DrawBehaviour;
use rnote_compose::color;
use rnote_compose::helpers::{AABBHelpers, Affine2Helpers};
use rnote_compose::shapes::Rectangle;
use rnote_compose::shapes::ShapeBehaviour;
use rnote_compose::transform::Transform;
use rnote_compose::transform::TransformBehaviour;

use anyhow::Context;
use gtk4::cairo;
use p2d::bounding_volume::{BoundingVolume, AABB};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename = "bitmapimage")]
pub struct BitmapImage {
    /// The bounds field of the image should not be used to determine the stroke bounds. Use rectangle.bounds() instead.
    #[serde(rename = "image")]
    pub image: render::Image,
    #[serde(rename = "rectangle")]
    pub rectangle: Rectangle,
}

impl Default for BitmapImage {
    fn default() -> Self {
        Self {
            image: render::Image::default(),
            rectangle: Rectangle::default(),
        }
    }
}

impl StrokeBehaviour for BitmapImage {
    fn gen_svg(&self) -> Result<render::Svg, anyhow::Error> {
        let bounds = self.bounds();
        let mut cx = piet_svg::RenderContext::new_no_text(kurbo::Size::new(
            bounds.extents()[0],
            bounds.extents()[1],
        ));

        self.draw(&mut cx, 1.0)?;
        let svg_data = rnote_compose::utils::piet_svg_cx_to_svg(cx)?;

        Ok(render::Svg { svg_data, bounds })
    }

    fn gen_images(
        &self,
        viewport: AABB,
        image_scale: f64,
    ) -> Result<GeneratedStrokeImages, anyhow::Error> {
        let bounds = self.bounds();

        if viewport.contains(&bounds) {
            Ok(GeneratedStrokeImages::Full(vec![
                render::Image::gen_with_piet(
                    |piet_cx| self.draw(piet_cx, image_scale),
                    bounds,
                    image_scale,
                )?,
            ]))
        } else {
            Ok(GeneratedStrokeImages::Partial {
                images: vec![render::Image::gen_with_piet(
                    |piet_cx| self.draw(piet_cx, image_scale),
                    viewport,
                    image_scale,
                )?],
                viewport,
            })
        }
    }
}

impl DrawBehaviour for BitmapImage {
    fn draw(&self, cx: &mut impl piet::RenderContext, _image_scale: f64) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{}", e))?;

        let piet_image_format = piet::ImageFormat::try_from(self.image.memory_format)?;

        cx.transform(self.rectangle.transform.affine.to_kurbo());

        let piet_image = cx
            .make_image(
                self.image.pixel_width as usize,
                self.image.pixel_height as usize,
                &self.image.data,
                piet_image_format,
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let dest_rect = self.rectangle.cuboid.local_aabb().to_kurbo_rect();
        cx.draw_image(&piet_image, dest_rect, piet::InterpolationMode::Bilinear);

        cx.restore().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }
}

impl ShapeBehaviour for BitmapImage {
    fn bounds(&self) -> AABB {
        self.rectangle.bounds()
    }

    fn hitboxes(&self) -> Vec<AABB> {
        vec![self.bounds()]
    }
}

impl TransformBehaviour for BitmapImage {
    fn translate(&mut self, offset: nalgebra::Vector2<f64>) {
        self.rectangle.translate(offset);
    }

    fn rotate(&mut self, angle: f64, center: nalgebra::Point2<f64>) {
        self.rectangle.rotate(angle, center);
    }

    fn scale(&mut self, scale: na::Vector2<f64>) {
        self.rectangle.scale(scale);
    }
}

impl BitmapImage {
    /// The default offset in surface coords when importing a bitmap image
    pub const IMPORT_OFFSET_DEFAULT: na::Vector2<f64> = na::vector![32.0, 32.0];

    pub fn import_from_image_bytes(
        bytes: &[u8],
        pos: na::Vector2<f64>,
    ) -> Result<Self, anyhow::Error> {
        let mut image = render::Image::try_from_encoded_bytes(bytes)?;
        // Ensure we are in rgba8-remultiplied format, to be able to draw to piet
        image.convert_to_rgba8pre()?;

        let size = na::vector![f64::from(image.pixel_width), f64::from(image.pixel_height)];

        let rectangle = Rectangle {
            cuboid: p2d::shape::Cuboid::new(size * 0.5),
            transform: Transform::new_w_isometry(na::Isometry2::new(pos + size * 0.5, 0.0)),
        };

        Ok(Self { image, rectangle })
    }

    pub fn import_from_pdf_bytes(
        to_be_read: &[u8],
        pos: na::Vector2<f64>,
        page_width: Option<i32>,
    ) -> Result<Vec<Self>, anyhow::Error> {
        let doc = poppler::Document::from_data(to_be_read, None)?;

        let mut images = Vec::new();

        for i in 0..doc.n_pages() {
            if let Some(page) = doc.page(i) {
                let intrinsic_size = page.size();

                let (width, height, zoom) = if let Some(page_width) = page_width {
                    let zoom = f64::from(page_width) / intrinsic_size.0;

                    (page_width, (intrinsic_size.1 * zoom).round() as i32, zoom)
                } else {
                    (
                        intrinsic_size.0.round() as i32,
                        intrinsic_size.1.round() as i32,
                        1.0,
                    )
                };

                let x = pos[0];
                let y = pos[1]
                    + f64::from(i)
                        * (f64::from(height) + f64::from(Self::IMPORT_OFFSET_DEFAULT[1]) * 0.5);

                let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width, height)
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "create ImageSurface with dimensions ({}, {}) failed, {}",
                            width,
                            height,
                            e
                        )
                    })?;

                {
                    let cx = cairo::Context::new(&surface).context("new cairo::Context failed")?;
                    cx.scale(zoom, zoom);

                    // Set margin to white
                    cx.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                    cx.paint()?;

                    page.render(&cx);

                    cx.scale(1.0 / zoom, 1.0 / zoom);

                    // Draw outline around page
                    cx.set_source_rgba(
                        color::GNOME_REDS[4].as_rgba().0,
                        color::GNOME_REDS[4].as_rgba().1,
                        color::GNOME_REDS[4].as_rgba().2,
                        1.0,
                    );

                    let line_width = 1.0;
                    cx.set_line_width(line_width);
                    cx.rectangle(
                        line_width * 0.5,
                        line_width * 0.5,
                        f64::from(width) - line_width,
                        f64::from(height) - line_width,
                    );
                    cx.stroke()?;
                }

                let mut png_data: Vec<u8> = Vec::new();
                surface.write_to_png(&mut png_data)?;

                images.push(Self::import_from_image_bytes(&png_data, na::vector![x, y])?);
            }
        }

        Ok(images)
    }
}
