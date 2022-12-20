pub mod export;
pub mod import;
pub mod rendering;
pub mod visual_debug;

// Re-Exports
pub use self::export::ExportPrefs;
use self::export::{SelectionExportFormat, SelectionExportPrefs};
pub use self::import::ImportPrefs;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::document::Layout;
use crate::pens::penholder::PenStyle;
use crate::pens::PenMode;
use crate::store::{ChronoComponent, StrokeKey};
use crate::strokes::strokebehaviour::GeneratedStrokeImages;
use crate::strokes::Stroke;
use crate::{render, AudioPlayer, WidgetFlags};
use crate::{Camera, Document, PenHolder, StrokeStore};
use rnote_compose::helpers::AABBHelpers;
use rnote_compose::penevents::{PenEvent, ShortcutKey};

use futures::channel::mpsc;
use gtk4::gsk;
use p2d::bounding_volume::{BoundingVolume, AABB};
use serde::{Deserialize, Serialize};
use slotmap::{HopSlotMap, SecondaryMap};

/// A view into the rest of the engine, excluding the penholder
#[allow(missing_debug_implementations)]
pub struct EngineView<'a> {
    pub tasks_tx: EngineTaskSender,
    pub doc: &'a Document,
    pub store: &'a StrokeStore,
    pub camera: &'a Camera,
    pub audioplayer: &'a Option<AudioPlayer>,
}

/// A mutable view into the rest of the engine, excluding the penholder
#[allow(missing_debug_implementations)]
pub struct EngineViewMut<'a> {
    pub tasks_tx: EngineTaskSender,
    pub doc: &'a mut Document,
    pub store: &'a mut StrokeStore,
    pub camera: &'a mut Camera,
    pub audioplayer: &'a mut Option<AudioPlayer>,
}

impl<'a> EngineViewMut<'a> {
    // converts itself to the immutable view
    pub fn as_im<'m>(&'m self) -> EngineView<'m> {
        EngineView::<'m> {
            tasks_tx: self.tasks_tx.clone(),
            doc: self.doc,
            store: self.store,
            camera: self.camera,
            audioplayer: self.audioplayer,
        }
    }
}

#[derive(Debug, Clone)]
/// A engine task, usually coming from a spawned thread and to be processed with `process_received_task()`.
pub enum EngineTask {
    /// Replace the images of the render_comp.
    /// Note that usually the state of the render component should be set **before** spawning a thread, generating images and sending this task,
    /// to avoid spawning large amounts of already outdated rendering tasks when checking the render component state on resize / zooming, etc.
    UpdateStrokeWithImages {
        key: StrokeKey,
        images: GeneratedStrokeImages,
    },
    /// Appends the images to the rendering of the stroke
    /// Note that usually the state of the render component should be set **before** spawning a thread, generating images and sending this task,
    /// to avoid spawning large amounts of already outdated rendering tasks when checking the render component state on resize / zooming, etc.
    AppendImagesToStroke {
        key: StrokeKey,
        images: GeneratedStrokeImages,
    },
    /// indicates that the application is quitting. Usually handled to quit the async loop which receives the tasks
    Quit,
}

#[allow(missing_debug_implementations)]
#[derive(Serialize, Deserialize)]
#[serde(default, rename = "engine_config")]
struct EngineConfig {
    #[serde(rename = "document")]
    document: serde_json::Value,
    #[serde(rename = "penholder")]
    penholder: serde_json::Value,
    #[serde(rename = "import_prefs")]
    import_prefs: serde_json::Value,
    #[serde(rename = "export_prefs")]
    export_prefs: serde_json::Value,
    #[serde(rename = "pen_sounds")]
    pen_sounds: serde_json::Value,
}

impl Default for EngineConfig {
    fn default() -> Self {
        let engine = RnoteEngine::default();

        Self {
            document: serde_json::to_value(&engine.document).unwrap(),
            penholder: serde_json::to_value(&engine.penholder).unwrap(),

            import_prefs: serde_json::to_value(engine.import_prefs).unwrap(),
            export_prefs: serde_json::to_value(engine.export_prefs).unwrap(),
            pen_sounds: serde_json::to_value(engine.pen_sounds).unwrap(),
        }
    }
}

// the engine snapshot, used when saving and loading to and from a file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default, rename = "engine_snapshot")]
pub struct EngineSnapshot {
    #[serde(rename = "document")]
    pub document: Document,
    #[serde(rename = "stroke_components")]
    pub stroke_components: Arc<HopSlotMap<StrokeKey, Arc<Stroke>>>,
    #[serde(rename = "chrono_components")]
    pub chrono_components: Arc<SecondaryMap<StrokeKey, Arc<ChronoComponent>>>,
    #[serde(rename = "chrono_counter")]
    pub chrono_counter: u32,
}

impl Default for EngineSnapshot {
    fn default() -> Self {
        Self {
            document: Document::default(),
            stroke_components: Arc::new(HopSlotMap::with_key()),
            chrono_components: Arc::new(SecondaryMap::new()),
            chrono_counter: 0,
        }
    }
}

pub type EngineTaskSender = mpsc::UnboundedSender<EngineTask>;
pub type EngineTaskReceiver = mpsc::UnboundedReceiver<EngineTask>;

/// The engine.
#[allow(missing_debug_implementations)]
#[derive(Serialize, Deserialize)]
#[serde(default, rename = "engine")]
pub struct RnoteEngine {
    #[serde(rename = "document")]
    pub document: Document,
    #[serde(rename = "penholder")]
    pub penholder: PenHolder,
    #[serde(rename = "store")]
    pub store: StrokeStore,
    #[serde(rename = "camera")]
    pub camera: Camera,

    #[serde(rename = "import_prefs")]
    pub import_prefs: ImportPrefs,
    #[serde(rename = "export_prefs")]
    pub export_prefs: ExportPrefs,
    #[serde(rename = "pen_sounds")]
    pen_sounds: bool,

    #[serde(skip)]
    pub audioplayer: Option<AudioPlayer>,
    #[serde(skip)]
    pub visual_debug: bool,
    #[serde(skip)]
    pub tasks_tx: EngineTaskSender,
    /// To be taken out into a loop which processes the receiver stream. The received tasks should be processed with process_received_task()
    #[serde(skip)]
    pub tasks_rx: Option<EngineTaskReceiver>,
    // Background rendering
    #[serde(skip)]
    pub background_tile_image: Option<render::Image>,
    #[serde(skip)]
    background_rendernodes: Vec<gsk::RenderNode>,
}

impl Default for RnoteEngine {
    fn default() -> Self {
        let (tasks_tx, tasks_rx) = futures::channel::mpsc::unbounded::<EngineTask>();

        Self {
            document: Document::default(),
            penholder: PenHolder::default(),
            store: StrokeStore::default(),
            camera: Camera::default(),

            import_prefs: ImportPrefs::default(),
            export_prefs: ExportPrefs::default(),
            pen_sounds: false,

            audioplayer: None,
            visual_debug: false,
            tasks_tx,
            tasks_rx: Some(tasks_rx),
            background_tile_image: None,
            background_rendernodes: Vec::default(),
        }
    }
}

impl RnoteEngine {
    pub fn tasks_tx(&self) -> EngineTaskSender {
        self.tasks_tx.clone()
    }

    /// Gets the EngineView
    pub fn view(&self) -> EngineView {
        EngineView {
            tasks_tx: self.tasks_tx.clone(),
            doc: &self.document,
            store: &self.store,
            camera: &self.camera,
            audioplayer: &self.audioplayer,
        }
    }

    /// Gets the EngineViewMut
    pub fn view_mut(&mut self) -> EngineViewMut {
        EngineViewMut {
            tasks_tx: self.tasks_tx.clone(),
            doc: &mut self.document,
            store: &mut self.store,
            camera: &mut self.camera,
            audioplayer: &mut self.audioplayer,
        }
    }

    /// whether pen sounds are enabled
    pub fn pen_sounds(&self) -> bool {
        self.pen_sounds
    }

    /// enables / disables the pen sounds.
    /// If pen sound should be enabled, the rnote data dir must be provided.
    pub fn set_pen_sounds(&mut self, pen_sounds: bool, data_dir: Option<PathBuf>) {
        self.pen_sounds = pen_sounds;

        if pen_sounds {
            if let Some(data_dir) = data_dir {
                // Only create and init a new audioplayer if it does not already exists
                if self.audioplayer.is_none() {
                    self.audioplayer = match AudioPlayer::new_init(data_dir) {
                        Ok(audioplayer) => Some(audioplayer),
                        Err(e) => {
                            log::error!("creating a new audioplayer failed, Err: {e:?}");
                            None
                        }
                    }
                }
            }
        } else {
            self.audioplayer.take();
        }
    }

    /// Takes a snapshot of the current state
    pub fn take_snapshot(&self) -> EngineSnapshot {
        let mut store_history_entry = self.store.history_entry_from_current_state();

        // Remove all trashed strokes
        let trashed_keys = store_history_entry
            .trash_components
            .iter()
            .filter_map(|(key, trash_comp)| if trash_comp.trashed { Some(key) } else { None })
            .collect::<Vec<StrokeKey>>();

        for key in trashed_keys {
            Arc::make_mut(&mut Arc::make_mut(&mut store_history_entry).stroke_components)
                .remove(key);
        }

        let snapshot = EngineSnapshot {
            document: self.document.clone(),
            stroke_components: Arc::clone(&store_history_entry.stroke_components),
            chrono_components: Arc::clone(&store_history_entry.chrono_components),
            chrono_counter: store_history_entry.chrono_counter,
        };

        snapshot
    }

    /// imports a engine snapshot. A loaded strokes store should always be imported with this method.
    /// the store then needs to update its rendering
    pub fn import_snapshot(&mut self, snapshot: &EngineSnapshot) {
        self.document = snapshot.document.clone();
        self.store.import_from_snapshot(&snapshot);

        self.update_pens_states();
    }

    /// records the current store state and saves it as a history entry.
    pub fn record(&mut self, _now: Instant) -> WidgetFlags {
        self.store.record()
    }

    /// Undo the latest changes
    pub fn undo(&mut self, now: Instant) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();
        let current_pen_style = self.penholder.current_style_w_override();

        if current_pen_style != PenStyle::Selector {
            widget_flags.merge_with_other(self.handle_pen_event(PenEvent::Cancel, None, now));
        }

        widget_flags.merge_with_other(self.store.undo());

        if !self.store.selection_keys_unordered().is_empty() {
            widget_flags.merge_with_other(
                self.penholder
                    .force_style_override_without_sideeffects(None),
            );
            widget_flags.merge_with_other(
                self.penholder
                    .force_style_without_sideeffects(PenStyle::Selector),
            );
        }

        self.resize_autoexpand();
        self.update_pens_states();
        if let Err(e) = self.update_rendering_current_viewport() {
            log::error!("failed to update rendering for current viewport while undo, Err: {e:?}");
        }

        widget_flags.redraw = true;

        widget_flags
    }

    /// redo the latest changes
    pub fn redo(&mut self, now: Instant) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();
        let current_pen_style = self.penholder.current_style_w_override();

        if current_pen_style != PenStyle::Selector {
            widget_flags.merge_with_other(self.handle_pen_event(PenEvent::Cancel, None, now));
        }

        widget_flags.merge_with_other(self.store.redo());

        if !self.store.selection_keys_unordered().is_empty() {
            widget_flags.merge_with_other(
                self.penholder
                    .force_style_override_without_sideeffects(None),
            );
            widget_flags.merge_with_other(
                self.penholder
                    .force_style_without_sideeffects(PenStyle::Selector),
            );
        }

        self.resize_autoexpand();
        self.update_pens_states();
        if let Err(e) = self.update_rendering_current_viewport() {
            log::error!("failed to update rendering for current viewport while redo, Err: {e:?}");
        }

        widget_flags.redraw = true;

        widget_flags
    }

    // Clears the store
    pub fn clear(&mut self) {
        self.store.clear();
        self.update_pens_states();
    }

    /// processes the received task from tasks_rx.
    /// Returns widget flags to indicate what needs to be updated in the UI.
    /// An example how to use it:
    /// ```rust, ignore
    /// let main_cx = glib::MainContext::default();

    /// main_cx.spawn_local(clone!(@strong canvas, @strong appwindow => async move {
    ///            let mut task_rx = canvas.engine().borrow_mut().store.tasks_rx.take().unwrap();

    ///           loop {
    ///              if let Some(task) = task_rx.next().await {
    ///                    let widget_flags = canvas.engine().borrow_mut().process_received_task(task);
    ///                    if appwindow.handle_widget_flags(widget_flags) {
    ///                         break;
    ///                    }
    ///                }
    ///            }
    ///        }));
    /// ```
    /// Processes a received store task. Usually called from a receiver loop which polls tasks_rx.
    pub fn process_received_task(&mut self, task: EngineTask) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();

        match task {
            EngineTask::UpdateStrokeWithImages { key, images } => {
                if let Err(e) = self.store.replace_rendering_with_images(key, images) {
                    log::error!("replace_rendering_with_images() in process_received_task() failed with Err: {e:?}");
                }

                widget_flags.redraw = true;
            }
            EngineTask::AppendImagesToStroke { key, images } => {
                if let Err(e) = self.store.append_rendering_images(key, images) {
                    log::error!(
                        "append_rendering_images() in process_received_task() failed with Err: {e:?}"
                    );
                }

                widget_flags.redraw = true;
            }
            EngineTask::Quit => {
                widget_flags.quit = true;
            }
        }

        widget_flags
    }

    /// handle an pen event
    pub fn handle_pen_event(
        &mut self,
        event: PenEvent,
        pen_mode: Option<PenMode>,
        now: Instant,
    ) -> WidgetFlags {
        self.penholder.handle_pen_event(
            event,
            pen_mode,
            now,
            &mut EngineViewMut {
                tasks_tx: self.tasks_tx(),
                doc: &mut self.document,
                store: &mut self.store,
                camera: &mut self.camera,
                audioplayer: &mut self.audioplayer,
            },
        )
    }

    /// Handle a pressed shortcut key
    pub fn handle_pen_pressed_shortcut_key(
        &mut self,
        shortcut_key: ShortcutKey,
        now: Instant,
    ) -> WidgetFlags {
        self.penholder.handle_pressed_shortcut_key(
            shortcut_key,
            now,
            &mut EngineViewMut {
                tasks_tx: self.tasks_tx(),
                doc: &mut self.document,
                store: &mut self.store,
                camera: &mut self.camera,
                audioplayer: &mut self.audioplayer,
            },
        )
    }

    /// change the pen style
    pub fn change_pen_style(&mut self, new_style: PenStyle, now: Instant) -> WidgetFlags {
        self.penholder.change_style(
            new_style,
            now,
            &mut EngineViewMut {
                tasks_tx: self.tasks_tx(),
                doc: &mut self.document,
                store: &mut self.store,
                camera: &mut self.camera,
                audioplayer: &mut self.audioplayer,
            },
        )
    }

    /// change the pen style override
    pub fn change_pen_style_override(
        &mut self,
        new_style_override: Option<PenStyle>,
        now: Instant,
    ) -> WidgetFlags {
        self.penholder.change_style_override(
            new_style_override,
            now,
            &mut EngineViewMut {
                tasks_tx: self.tasks_tx(),
                doc: &mut self.document,
                store: &mut self.store,
                camera: &mut self.camera,
                audioplayer: &mut self.audioplayer,
            },
        )
    }

    /// change the pen mode. Relevant for stylus input
    pub fn change_pen_mode(&mut self, pen_mode: PenMode, now: Instant) -> WidgetFlags {
        self.penholder.change_pen_mode(
            pen_mode,
            now,
            &mut EngineViewMut {
                tasks_tx: self.tasks_tx(),
                doc: &mut self.document,
                store: &mut self.store,
                camera: &mut self.camera,
                audioplayer: &mut self.audioplayer,
            },
        )
    }

    // Generates bounds for each page on the document which contains content
    pub fn pages_bounds_w_content(&self) -> Vec<AABB> {
        let doc_bounds = self.document.bounds();
        let keys = self.store.stroke_keys_as_rendered();

        let strokes_bounds = self.store.strokes_bounds(&keys);

        let pages_bounds = doc_bounds
            .split_extended_origin_aligned(na::vector![
                self.document.format.width,
                self.document.format.height
            ])
            .into_iter()
            .filter(|page_bounds| {
                // Filter the pages out that doesn't intersect with any stroke
                strokes_bounds
                    .iter()
                    .any(|stroke_bounds| stroke_bounds.intersects(page_bounds))
            })
            .collect::<Vec<AABB>>();

        if pages_bounds.is_empty() {
            // If no page has content, return the origin page
            vec![AABB::new(
                na::point![0.0, 0.0],
                na::point![self.document.format.width, self.document.format.height],
            )]
        } else {
            pages_bounds
        }
    }

    /// Generates bounds which contain all pages on the doc with content extended to fit the format.
    pub fn bounds_w_content_extended(&self) -> Option<AABB> {
        let pages_bounds = self.pages_bounds_w_content();

        if pages_bounds.is_empty() {
            return None;
        }

        Some(
            pages_bounds
                .into_iter()
                .fold(AABB::new_invalid(), |prev, next| prev.merged(&next)),
        )
    }

    /// the current document layout
    pub fn doc_layout(&self) -> Layout {
        self.document.layout()
    }

    pub fn set_doc_layout(&mut self, layout: Layout) {
        self.document.set_layout(layout, &self.store, &self.camera);
    }

    /// resizes the doc to the format and to fit all strokes
    /// Document background rendering then needs to be updated.
    pub fn resize_to_fit_strokes(&mut self) {
        self.document
            .resize_to_fit_strokes(&self.store, &self.camera);
    }

    /// resize the doc when in autoexpanding layouts. called e.g. when finishing a new stroke
    /// Document background rendering then needs to be updated.
    pub fn resize_autoexpand(&mut self) {
        self.document.resize_autoexpand(&self.store, &self.camera);
    }

    /// Updates the camera and expands doc dimensions with offset
    /// Document background rendering then needs to be updated.
    pub fn update_camera_offset(&mut self, new_offset: na::Vector2<f64>) {
        self.camera.offset = new_offset;

        match self.document.layout() {
            Layout::FixedSize => {
                // Does not resize in fixed size mode, use resize_doc_to_fit_strokes() for it.
            }
            Layout::ContinuousVertical => {
                self.document
                    .resize_doc_continuous_vertical_layout(&self.store);
            }
            Layout::Infinite => {
                // only expand, don't resize to fit strokes
                self.document
                    .expand_doc_infinite_layout(self.camera.viewport());
            }
        }
    }

    /// Updates pens state with the current engine state.
    /// needs to be called when the engine state was changed outside of pen events. ( e.g. trash all strokes, set strokes selected, etc. )
    pub fn update_pens_states(&mut self) {
        self.penholder.update_internal_state(&EngineView {
            tasks_tx: self.tasks_tx(),
            doc: &self.document,
            store: &self.store,
            camera: &self.camera,
            audioplayer: &self.audioplayer,
        });
    }

    /// Fetches clipboard content from current state.
    /// Returns (the content, mime_type)
    #[allow(clippy::type_complexity)]
    pub fn fetch_clipboard_content(
        &self,
    ) -> anyhow::Result<(Option<(Vec<u8>, String)>, WidgetFlags)> {
        let export_bytes = self.export_selection(Some(SelectionExportPrefs {
            with_background: true,
            export_format: SelectionExportFormat::Svg,
            ..Default::default()
        }));

        // First try exporting the selection as svg
        if let Some(selection_bytes) = futures::executor::block_on(async { export_bytes.await? })? {
            return Ok((
                Some((selection_bytes, String::from("image/svg+xml"))),
                WidgetFlags::default(),
            ));
        }

        // else fetch from pen
        self.penholder.fetch_clipboard_content(&EngineView {
            tasks_tx: self.tasks_tx(),
            doc: &self.document,
            store: &self.store,
            camera: &self.camera,
            audioplayer: &self.audioplayer,
        })
    }

    /// Cuts clipboard content from current state.
    /// Returns (the content, mime_type)
    #[allow(clippy::type_complexity)]
    pub fn cut_clipboard_content(
        &mut self,
    ) -> anyhow::Result<(Option<(Vec<u8>, String)>, WidgetFlags)> {
        /*
        // FIXME: Until svg import is broken, we don't want users being able to cut the selection without the possibility to insert it again.

                let export_bytes = self.export_selection(Some(SelectionExportPrefs {
                    with_background: true,
                    export_format: SelectionExportFormat::Svg,
                    ..Default::default()
                }));

                // First try exporting the selection as svg
                if let Some(selection_bytes) = futures::executor::block_on(async { export_bytes.await? })? {
                    return Ok(Some((selection_bytes, String::from("image/svg+xml"))));
                }
         */

        // else fetch from pen
        self.penholder.cut_clipboard_content(&mut EngineViewMut {
            tasks_tx: self.tasks_tx(),
            doc: &mut self.document,
            store: &mut self.store,
            camera: &mut self.camera,
            audioplayer: &mut self.audioplayer,
        })
    }
}
