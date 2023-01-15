use super::chrono_comp::StrokeLayer;
use super::{StrokeKey, StrokeStore};
use crate::strokes::{BrushStroke, Stroke};
use crate::WidgetFlags;

use p2d::bounding_volume::{Aabb, BoundingVolume};
use rnote_compose::shapes::ShapeBehaviour;
use rnote_compose::PenPath;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename = "trash_component")]
pub struct TrashComponent {
    #[serde(rename = "trashed")]
    pub trashed: bool,
}

impl Default for TrashComponent {
    fn default() -> Self {
        Self { trashed: false }
    }
}

/// Systems that are related trashing
impl StrokeStore {
    /// Rebuilds the slotmap with default trash components from the keys returned from the primary map, stroke_components.
    pub fn rebuild_trash_components_slotmap(&mut self) {
        self.trash_components = Arc::new(slotmap::SecondaryMap::new());
        self.stroke_components.keys().for_each(|key| {
            Arc::make_mut(&mut self.trash_components)
                .insert(key, Arc::new(TrashComponent::default()));
        });
    }

    pub fn can_trash(&self, key: StrokeKey) -> bool {
        self.trash_components.get(key).is_some()
    }

    pub fn trashed(&self, key: StrokeKey) -> Option<bool> {
        if let Some(trash_comp) = self.trash_components.get(key) {
            Some(trash_comp.trashed)
        } else {
            log::debug!(
                "get trash_comp in trashed() returned None for stroke with key {:?}",
                key
            );
            None
        }
    }

    pub fn set_trashed(&mut self, key: StrokeKey, trash: bool) {
        if let Some(trash_comp) = Arc::make_mut(&mut self.trash_components)
            .get_mut(key)
            .map(Arc::make_mut)
        {
            trash_comp.trashed = trash;

            self.update_chrono_to_last(key);
        } else {
            log::debug!(
                "get trash_comp in set_trashed() returned None for stroke with key {:?}",
                key
            );
        }
    }

    pub fn set_trashed_keys(&mut self, keys: &[StrokeKey], trash: bool) {
        keys.iter().for_each(|&key| {
            self.set_selected(key, false);
            self.set_trashed(key, trash);
            self.update_chrono_to_last(key);
        });
    }

    pub fn trashed_keys_unordered(&self) -> Vec<StrokeKey> {
        self.stroke_components
            .keys()
            .filter(|&key| self.trashed(key).unwrap_or(false))
            .collect()
    }

    pub fn remove_trashed_strokes(&mut self) {
        for key in self.trashed_keys_unordered() {
            self.remove_stroke(key);
        }
    }

    /// trash strokes that collide with the given bounds
    pub fn trash_colliding_strokes(&mut self, eraser_bounds: Aabb, viewport: Aabb) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();

        self.stroke_keys_as_rendered_intersecting_bounds(viewport)
            .into_iter()
            .for_each(|key| {
                let mut trash_current_stroke = false;

                if let Some(stroke) = self.stroke_components.get(key) {
                    match stroke.as_ref() {
                        Stroke::BrushStroke(_) | Stroke::ShapeStroke(_) => {
                            // First check if eraser even intersects stroke bounds, avoiding unnecessary work
                            if eraser_bounds.intersects(&stroke.bounds()) {
                                for hitbox in stroke.hitboxes().into_iter() {
                                    if eraser_bounds.intersects(&hitbox) {
                                        trash_current_stroke = true;

                                        break;
                                    }
                                }
                            }
                        }
                        // Ignore other strokes when trashing with the Eraser
                        Stroke::TextStroke(_) | Stroke::VectorImage(_) | Stroke::BitmapImage(_) => {
                        }
                    }
                }

                if trash_current_stroke {
                    widget_flags.merge(self.record(Instant::now()));
                    self.set_trashed(key, true);
                }
            });

        widget_flags
    }

    /// remove colliding stroke segments with the given bounds. The stroke is then split. For strokes that don't have segments, trash the entire stroke.
    /// Returns the keys of all created or modified strokes.
    /// returned strokes need to update their rendering.
    pub fn split_colliding_strokes(
        &mut self,
        eraser_bounds: Aabb,
        viewport: Aabb,
    ) -> Vec<StrokeKey> {
        let mut modified_keys = vec![];

        let new_strokes = self
            .stroke_keys_as_rendered_intersecting_bounds(viewport)
            .into_iter()
            .flat_map(|key| {
                let Some(stroke) = Arc::make_mut(&mut self.stroke_components)
                    .get_mut(key)
                    .map(Arc::make_mut) else {
                    return vec![]
                };

                let Some(chrono_comp) = self.chrono_components.get(key) else {
                    return vec![]
                };

                let mut new_strokes = vec![];
                let mut trash_current_stroke = false;
                let stroke_bounds = stroke.bounds();

                match stroke {
                    Stroke::BrushStroke(brushstroke) => {
                        if eraser_bounds.intersects(&stroke_bounds) {
                            let mut splitted = Vec::new();

                            let mut hits = brushstroke
                                .path
                                .hittest(&eraser_bounds, brushstroke.style.stroke_width() * 0.5)
                                .into_iter();

                            if let Some(first_hit) = hits.next() {
                                let mut prev = first_hit;
                                for hit in hits {
                                    let split = &brushstroke.path.segments[prev..hit];

                                    // skip splits that don't have at least two segments (one's end as path start, one additional)
                                    if split.len() > 1 {
                                        splitted.push(split.to_vec());
                                    }

                                    prev = hit;
                                }

                                // Catch the last
                                let last_split = &brushstroke.path.segments[prev..];
                                if last_split.len() > 1 {
                                    splitted.push(last_split.to_vec());
                                }

                                for next_split in splitted {
                                    let mut next_split_iter = next_split.into_iter();
                                    let next_start = next_split_iter.next().unwrap().end();

                                    new_strokes.push((
                                        Stroke::BrushStroke(BrushStroke::from_penpath(
                                            PenPath::new_w_segments(next_start, next_split_iter),
                                            brushstroke.style.clone(),
                                        )),
                                        chrono_comp.layer,
                                    ));
                                }

                                let first_split = &brushstroke.path.segments[..first_hit];
                                // Modify the original stroke at the end.
                                // We keep the start, so we only need at least one segment
                                if !first_split.is_empty() {
                                    brushstroke.replace_path(PenPath::new_w_segments(
                                        brushstroke.path.start,
                                        first_split.to_vec(),
                                    ));

                                    modified_keys.push(key);
                                } else {
                                    trash_current_stroke = true;
                                }
                            }
                        }
                    }
                    Stroke::ShapeStroke(_) => {
                        if eraser_bounds.intersects(&stroke_bounds) {
                            for hitbox_elem in stroke.hitboxes().iter() {
                                if eraser_bounds.intersects(hitbox_elem) {
                                    trash_current_stroke = true;
                                }
                            }
                        }
                    }
                    // Ignore other strokes when trashing with the Eraser
                    Stroke::TextStroke(_) | Stroke::VectorImage(_) | Stroke::BitmapImage(_) => {}
                }

                if trash_current_stroke {
                    self.set_trashed(key, true);
                }

                new_strokes
            })
            .collect::<Vec<(Stroke, StrokeLayer)>>();

        modified_keys.append(
            &mut new_strokes
                .into_iter()
                .map(|(new_stroke, layer)| self.insert_stroke(new_stroke, Some(layer)))
                .collect(),
        );

        modified_keys
    }
}
