// Imports
use crate::engine::EngineView;
use crate::utils::GrapheneRectHelpers;
use gtk4::{graphene, prelude::*};
use p2d::bounding_volume::Aabb;
use piet::RenderContext;
use rnote_compose::helpers::{AabbHelpers, Affine2Helpers};

/// Trait for types that can draw themselves on the document.
///
/// In the coordinate space of the document
pub trait DrawOnDocBehaviour {
    /// Bounds on the document.
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<Aabb>;
    /// Draw itself on the document.
    ///
    /// The implementors are expected to save/restore the drawing context.
    fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()>;

    /// Draw itself on the snapshot.
    ///
    /// The snapshot is expected to be untransformed in surface coordinate space.
    fn draw_on_doc_to_gtk_snapshot(
        &self,
        snapshot: &gtk4::Snapshot,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        snapshot.save();

        if let Some(bounds) = self.bounds_on_doc(engine_view) {
            let viewport = engine_view.camera.viewport();

            // Restrict to viewport as maximum bounds. Else cairo will panic for very large bounds
            let bounds = bounds.clamp(None, Some(viewport));
            // Transform the bounds into surface coords
            let mut bounds_transformed = bounds
                .scale(engine_view.camera.total_zoom())
                .translate(-engine_view.camera.offset)
                .ceil();

            bounds_transformed.ensure_positive();
            bounds_transformed.assert_valid()?;

            let cairo_cx =
                snapshot.append_cairo(&graphene::Rect::from_p2d_aabb(bounds_transformed));
            let mut piet_cx = piet_cairo::CairoRenderContext::new(&cairo_cx);

            // Transform to document coordinate space
            piet_cx.transform(engine_view.camera.transform().to_kurbo());

            self.draw_on_doc(&mut piet_cx, engine_view)?;
        }

        snapshot.restore();
        Ok(())
    }
}

/// Trait for types that can draw themselves on a [piet::RenderContext].
pub trait DrawBehaviour {
    /// Draw itself.
    /// The implementors are expected to save/restore the drawing context.
    ///
    /// `image_scale` is the scale-factor of generated images within the type.
    /// The content should not be zoomed by it!
    fn draw(&self, cx: &mut impl piet::RenderContext, image_scale: f64) -> anyhow::Result<()>;
}
