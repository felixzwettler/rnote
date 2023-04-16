mod canvaslayout;
pub(crate) mod imexport;
mod input;

// Re-exports
pub(crate) use canvaslayout::RnCanvasLayout;

// Imports
use futures::StreamExt;
use gettextrs::gettext;
use gtk4::{
    gdk, gio, glib, glib::clone, graphene, prelude::*, subclass::prelude::*, AccessibleRole,
    Adjustment, DropTarget, EventControllerKey, EventControllerLegacy, IMMulticontext, Inhibit,
    PropagationPhase, Scrollable, ScrollablePolicy, Widget,
};
use once_cell::sync::Lazy;
use p2d::bounding_volume::Aabb;
use rnote_compose::helpers::AabbHelpers;
use rnote_compose::penevents::PenState;
use rnote_engine::utils::GrapheneRectHelpers;
use rnote_engine::Document;
use rnote_engine::{RnoteEngine, WidgetFlags};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::RnCanvasWrapper;
use crate::{config, RnAppWindow};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, glib::Boxed)]
#[boxed_type(name = "WidgetFlagsBoxed")]
struct WidgetFlagsBoxed(WidgetFlags);

#[derive(Debug, Default)]
pub(crate) struct Handlers {
    pub(crate) hadjustment: Option<glib::SignalHandlerId>,
    pub(crate) vadjustment: Option<glib::SignalHandlerId>,
    pub(crate) zoom_timeout: Option<glib::SourceId>,
    pub(crate) tab_page_output_file: Option<glib::Binding>,
    pub(crate) tab_page_unsaved_changes: Option<glib::Binding>,
    pub(crate) appwindow_output_file: Option<glib::SignalHandlerId>,
    pub(crate) appwindow_scalefactor: Option<glib::SignalHandlerId>,
    pub(crate) appwindow_unsaved_changes: Option<glib::SignalHandlerId>,
    pub(crate) appwindow_touch_drawing: Option<glib::Binding>,
    pub(crate) appwindow_regular_cursor: Option<glib::Binding>,
    pub(crate) appwindow_drawing_cursor: Option<glib::Binding>,
    pub(crate) appwindow_drop_target: Option<glib::SignalHandlerId>,
    pub(crate) appwindow_zoom_changed: Option<glib::SignalHandlerId>,
    pub(crate) appwindow_handle_widget_flags: Option<glib::SignalHandlerId>,
}

mod imp {
    use super::*;

    #[allow(missing_debug_implementations)]
    pub(crate) struct RnCanvas {
        pub(crate) handlers: RefCell<Handlers>,

        pub(crate) hadjustment: RefCell<Option<Adjustment>>,
        pub(crate) vadjustment: RefCell<Option<Adjustment>>,
        pub(crate) hscroll_policy: Cell<ScrollablePolicy>,
        pub(crate) vscroll_policy: Cell<ScrollablePolicy>,
        pub(crate) regular_cursor: RefCell<gdk::Cursor>,
        pub(crate) regular_cursor_icon_name: RefCell<String>,
        pub(crate) drawing_cursor: RefCell<gdk::Cursor>,
        pub(crate) drawing_cursor_icon_name: RefCell<String>,
        pub(crate) pointer_controller: EventControllerLegacy,
        pub(crate) key_controller: EventControllerKey,
        pub(crate) key_controller_im_context: IMMulticontext,
        pub(crate) drop_target: DropTarget,
        pub(crate) drawing_cursor_enabled: Cell<bool>,

        pub(crate) engine: Rc<RefCell<RnoteEngine>>,

        pub(crate) output_file: RefCell<Option<gio::File>>,
        pub(crate) output_file_monitor: RefCell<Option<gio::FileMonitor>>,
        pub(crate) output_file_monitor_changed_handler: RefCell<Option<glib::SignalHandlerId>>,
        pub(crate) output_file_modified_toast_singleton: RefCell<Option<adw::Toast>>,
        pub(crate) output_file_expect_write: Cell<bool>,
        pub(crate) save_in_progress: Cell<bool>,
        pub(crate) unsaved_changes: Cell<bool>,
        pub(crate) empty: Cell<bool>,
        pub(crate) touch_drawing: Cell<bool>,
    }

    impl Default for RnCanvas {
        fn default() -> Self {
            let pointer_controller = EventControllerLegacy::builder()
                .name("pointer_controller")
                .propagation_phase(PropagationPhase::Bubble)
                .build();

            let key_controller = EventControllerKey::builder()
                .name("key_controller")
                .propagation_phase(PropagationPhase::Capture)
                .build();

            let key_controller_im_context = IMMulticontext::new();

            let drop_target = DropTarget::builder()
                .name("canvas_drop_target")
                .propagation_phase(PropagationPhase::Capture)
                .actions(gdk::DragAction::COPY)
                .build();

            // The order here is important: first files, then text
            drop_target.set_types(&[gio::File::static_type(), glib::types::Type::STRING]);

            let regular_cursor_icon_name = String::from("cursor-dot-medium");
            let regular_cursor = gdk::Cursor::from_texture(
                &gdk::Texture::from_resource(
                    (String::from(config::APP_IDPATH)
                        + "icons/scalable/actions/cursor-dot-medium.svg")
                        .as_str(),
                ),
                32,
                32,
                gdk::Cursor::from_name("default", None).as_ref(),
            );
            let drawing_cursor_icon_name = String::from("cursor-dot-small");
            let drawing_cursor = gdk::Cursor::from_texture(
                &gdk::Texture::from_resource(
                    (String::from(config::APP_IDPATH)
                        + "icons/scalable/actions/cursor-dot-small.svg")
                        .as_str(),
                ),
                32,
                32,
                gdk::Cursor::from_name("default", None).as_ref(),
            );

            let engine = RnoteEngine::default();

            Self {
                handlers: RefCell::new(Handlers::default()),

                hadjustment: RefCell::new(None),
                vadjustment: RefCell::new(None),
                hscroll_policy: Cell::new(ScrollablePolicy::Minimum),
                vscroll_policy: Cell::new(ScrollablePolicy::Minimum),
                regular_cursor: RefCell::new(regular_cursor),
                regular_cursor_icon_name: RefCell::new(regular_cursor_icon_name),
                drawing_cursor: RefCell::new(drawing_cursor),
                drawing_cursor_icon_name: RefCell::new(drawing_cursor_icon_name),
                pointer_controller,
                key_controller,
                key_controller_im_context,
                drop_target,
                drawing_cursor_enabled: Cell::new(false),

                engine: Rc::new(RefCell::new(engine)),

                output_file: RefCell::new(None),
                output_file_monitor: RefCell::new(None),
                output_file_monitor_changed_handler: RefCell::new(None),
                output_file_modified_toast_singleton: RefCell::new(None),
                output_file_expect_write: Cell::new(false),
                save_in_progress: Cell::new(false),
                unsaved_changes: Cell::new(false),
                empty: Cell::new(true),
                touch_drawing: Cell::new(false),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RnCanvas {
        const NAME: &'static str = "RnCanvas";
        type Type = super::RnCanvas;
        type ParentType = Widget;
        type Interfaces = (Scrollable,);

        fn class_init(klass: &mut Self::Class) {
            klass.set_accessible_role(AccessibleRole::Widget);
            klass.set_layout_manager_type::<RnCanvasLayout>();
        }

        fn new() -> Self {
            Self::default()
        }
    }

    impl ObjectImpl for RnCanvas {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.set_hexpand(false);
            obj.set_vexpand(false);
            // keyboard focus needed for typewriter
            obj.set_can_focus(true);
            obj.set_focusable(true);

            obj.set_cursor(Some(&*self.regular_cursor.borrow()));

            obj.add_controller(self.pointer_controller.clone());
            obj.add_controller(self.key_controller.clone());
            obj.add_controller(self.drop_target.clone());

            // receive and handling engine tasks
            glib::MainContext::default().spawn_local(
                clone!(@weak obj as canvas => async move {
                    let mut task_rx = canvas.engine().borrow_mut().regenerate_channel();

                    loop {
                        if let Some(task) = task_rx.next().await {
                            let (widget_flags, quit) = canvas.engine().borrow_mut().handle_engine_task(task);
                            canvas.emit_handle_widget_flags(widget_flags);

                            if quit {
                                break;
                            }
                        }
                    }
                }),
            );

            self.setup_input();
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    // this is nullable, so it can be used to represent Option<gio::File>
                    glib::ParamSpecObject::builder::<gio::File>("output-file").build(),
                    glib::ParamSpecBoolean::builder("unsaved-changes")
                        .default_value(false)
                        .build(),
                    glib::ParamSpecBoolean::builder("empty")
                        .default_value(true)
                        .build(),
                    glib::ParamSpecBoolean::builder("touch-drawing")
                        .default_value(false)
                        .build(),
                    glib::ParamSpecString::builder("regular-cursor")
                        .default_value(Some("cursor-dot-medium"))
                        .build(),
                    glib::ParamSpecString::builder("drawing-cursor")
                        .default_value(Some("cursor-dot-small"))
                        .build(),
                    // Scrollable properties
                    glib::ParamSpecOverride::for_interface::<Scrollable>("hscroll-policy"),
                    glib::ParamSpecOverride::for_interface::<Scrollable>("vscroll-policy"),
                    glib::ParamSpecOverride::for_interface::<Scrollable>("hadjustment"),
                    glib::ParamSpecOverride::for_interface::<Scrollable>("vadjustment"),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "output-file" => self.output_file.borrow().to_value(),
                "unsaved-changes" => self.unsaved_changes.get().to_value(),
                "empty" => self.empty.get().to_value(),
                "hadjustment" => self.hadjustment.borrow().to_value(),
                "vadjustment" => self.vadjustment.borrow().to_value(),
                "hscroll-policy" => self.hscroll_policy.get().to_value(),
                "vscroll-policy" => self.vscroll_policy.get().to_value(),
                "touch-drawing" => self.touch_drawing.get().to_value(),
                "regular-cursor" => self.regular_cursor_icon_name.borrow().to_value(),
                "drawing-cursor" => self.drawing_cursor_icon_name.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = self.obj();

            match pspec.name() {
                "output-file" => {
                    let output_file = value
                        .get::<Option<gio::File>>()
                        .expect("The value needs to be of type `Option<gio::File>`");
                    self.output_file.replace(output_file);
                }
                "unsaved-changes" => {
                    let unsaved_changes: bool =
                        value.get().expect("The value needs to be of type `bool`");
                    self.unsaved_changes.replace(unsaved_changes);
                }
                "empty" => {
                    let empty: bool = value.get().expect("The value needs to be of type `bool`");
                    self.empty.replace(empty);
                    if empty {
                        obj.set_unsaved_changes(false);
                    }
                }
                "hadjustment" => {
                    let hadj = value.get().unwrap();
                    self.set_hadjustment_prop(hadj);
                }
                "hscroll-policy" => {
                    let hscroll_policy = value.get().unwrap();
                    self.hscroll_policy.replace(hscroll_policy);
                }
                "vadjustment" => {
                    let vadj = value.get().unwrap();
                    self.set_vadjustment_prop(vadj);
                }
                "vscroll-policy" => {
                    let vscroll_policy = value.get().unwrap();
                    self.vscroll_policy.replace(vscroll_policy);
                }
                "touch-drawing" => {
                    let touch_drawing: bool =
                        value.get().expect("The value needs to be of type `bool`");
                    self.touch_drawing.replace(touch_drawing);
                }
                "regular-cursor" => {
                    let icon_name = value.get().unwrap();
                    self.regular_cursor_icon_name.replace(icon_name);

                    let cursor = gdk::Cursor::from_texture(
                        &gdk::Texture::from_resource(
                            (String::from(config::APP_IDPATH)
                                + &format!(
                                    "icons/scalable/actions/{}.svg",
                                    self.regular_cursor_icon_name.borrow()
                                ))
                                .as_str(),
                        ),
                        32,
                        32,
                        gdk::Cursor::from_name("default", None).as_ref(),
                    );

                    self.regular_cursor.replace(cursor);

                    obj.set_cursor(Some(&*self.regular_cursor.borrow()));
                }
                "drawing-cursor" => {
                    let icon_name = value.get().unwrap();
                    self.drawing_cursor_icon_name.replace(icon_name);

                    let cursor = gdk::Cursor::from_texture(
                        &gdk::Texture::from_resource(
                            (String::from(config::APP_IDPATH)
                                + &format!(
                                    "icons/scalable/actions/{}.svg",
                                    self.drawing_cursor_icon_name.borrow()
                                ))
                                .as_str(),
                        ),
                        32,
                        32,
                        gdk::Cursor::from_name("default", None).as_ref(),
                    );

                    self.drawing_cursor.replace(cursor);
                }
                _ => unimplemented!(),
            }
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: Lazy<Vec<glib::subclass::Signal>> = Lazy::new(|| {
                vec![
                    glib::subclass::Signal::builder("zoom-changed").build(),
                    glib::subclass::Signal::builder("handle-widget-flags")
                        .param_types([WidgetFlagsBoxed::static_type()])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for RnCanvas {
        // request_mode(), measure(), allocate() overrides happen in the CanvasLayout LayoutManager

        fn snapshot(&self, snapshot: &gtk4::Snapshot) {
            let obj = self.obj();

            if let Err(e) = || -> anyhow::Result<()> {
                let clip_bounds = if let Some(parent) = obj.parent() {
                    // unwrapping is fine, because its the parent
                    let (clip_x, clip_y) = parent.translate_coordinates(&*obj, 0.0, 0.0).unwrap();
                    Aabb::new_positive(
                        na::point![clip_x, clip_y],
                        na::point![f64::from(parent.width()), f64::from(parent.height())],
                    )
                } else {
                    obj.bounds()
                };
                // push the clip
                snapshot.push_clip(&graphene::Rect::from_p2d_aabb(clip_bounds));

                // Draw the entire engine
                self.engine
                    .borrow()
                    .draw_to_gtk_snapshot(snapshot, obj.bounds())?;

                // pop the clip
                snapshot.pop();
                Ok(())
            }() {
                log::error!("canvas snapshot() failed with Err: {e:?}");
            }
        }
    }

    impl ScrollableImpl for RnCanvas {}

    impl RnCanvas {
        fn setup_input(&self) {
            let obj = self.obj();

            // Pointer controller
            let pen_state = Cell::new(PenState::Up);
            self.pointer_controller.connect_event(clone!(@strong pen_state, @weak obj as canvas => @default-return Inhibit(false), move |_, event| {
                let (inhibit, new_state) = super::input::handle_pointer_controller_event(&canvas, event, pen_state.get());
                pen_state.set(new_state);
                inhibit
            }));

            // For unicode text the input is committed from the IM context, and won't trigger the key_pressed signal
            self.key_controller_im_context.connect_commit(
                clone!(@weak obj as canvas => move |_cx, text| {
                    super::input::handle_imcontext_text_commit(&canvas, text);
                }),
            );

            // Key controller
            self.key_controller.connect_key_pressed(clone!(@weak obj as canvas => @default-return Inhibit(false), move |_, key, _raw, modifier| {
                super::input::handle_key_controller_key_pressed(&canvas, key, modifier)
            }));
            /*
                       self.key_controller.connect_key_released(
                           clone!(@weak inst as canvas => move |_key_controller, _key, _raw, _modifier| {
                               //log::debug!("key released - key: {:?}, raw: {:?}, modifier: {:?}", key, raw, modifier);
                           }),
                       );
            */
        }

        fn set_hadjustment_prop(&self, hadj: Option<Adjustment>) {
            let obj = self.obj();

            if let Some(signal_id) = self.handlers.borrow_mut().hadjustment.take() {
                let old_adj = self.hadjustment.borrow().as_ref().unwrap().clone();
                old_adj.disconnect(signal_id);
            }

            if let Some(ref hadj) = hadj {
                let signal_id =
                    hadj.connect_value_changed(clone!(@weak obj as canvas => move |_| {
                        // this triggers a canvaslayout allocate() call, where the strokes rendering is updated based on some conditions
                        canvas.queue_resize();
                    }));

                self.handlers.borrow_mut().hadjustment.replace(signal_id);
            }
            self.hadjustment.replace(hadj);
        }

        fn set_vadjustment_prop(&self, vadj: Option<Adjustment>) {
            let obj = self.obj();

            if let Some(signal_id) = self.handlers.borrow_mut().vadjustment.take() {
                let old_adj = self.vadjustment.borrow().as_ref().unwrap().clone();
                old_adj.disconnect(signal_id);
            }

            if let Some(ref vadj) = vadj {
                let signal_id =
                    vadj.connect_value_changed(clone!(@weak obj as canvas => move |_| {
                        // this triggers a canvaslayout allocate() call, where the strokes rendering is updated based on some conditions
                        canvas.queue_resize();
                    }));

                self.handlers.borrow_mut().vadjustment.replace(signal_id);
            }
            self.vadjustment.replace(vadj);
        }
    }
}

glib::wrapper! {
    pub(crate) struct RnCanvas(ObjectSubclass<imp::RnCanvas>)
        @extends gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Scrollable;
}

impl Default for RnCanvas {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) static OUTPUT_FILE_NEW_TITLE: once_cell::sync::Lazy<String> =
    once_cell::sync::Lazy::new(|| gettext("New Document"));
pub(crate) static OUTPUT_FILE_NEW_SUBTITLE: once_cell::sync::Lazy<String> =
    once_cell::sync::Lazy::new(|| gettext("Draft"));

impl RnCanvas {
    // the zoom timeout time
    pub(crate) const ZOOM_TIMEOUT_TIME: Duration = Duration::from_millis(300);
    // Sets the canvas zoom scroll step in % for one unit of the event controller delta
    pub(crate) const ZOOM_SCROLL_STEP: f64 = 0.1;
    /// A small margin added to the document width, when zooming to fit document width
    pub(crate) const ZOOM_FIT_WIDTH_MARGIN: f64 = 32.0;

    pub(crate) fn new() -> Self {
        glib::Object::new()
    }

    #[allow(unused)]
    pub(crate) fn regular_cursor(&self) -> String {
        self.property::<String>("regular-cursor")
    }

    #[allow(unused)]
    pub(crate) fn set_regular_cursor(&self, regular_cursor: &str) {
        self.set_property("regular-cursor", regular_cursor.to_value());
    }

    #[allow(unused)]
    pub(crate) fn drawing_cursor(&self) -> String {
        self.property::<String>("drawing-cursor")
    }

    #[allow(unused)]
    pub(crate) fn set_drawing_cursor(&self, drawing_cursor: &str) {
        self.set_property("drawing-cursor", drawing_cursor.to_value());
    }

    #[allow(unused)]
    pub(crate) fn output_file(&self) -> Option<gio::File> {
        self.property::<Option<gio::File>>("output-file")
    }

    #[allow(unused)]
    pub(crate) fn output_file_expect_write(&self) -> bool {
        self.imp().output_file_expect_write.get()
    }

    #[allow(unused)]
    pub(crate) fn set_output_file_expect_write(&self, expect_write: bool) {
        self.imp().output_file_expect_write.set(expect_write);
    }

    #[allow(unused)]
    pub(crate) fn save_in_progress(&self) -> bool {
        self.imp().save_in_progress.get()
    }

    #[allow(unused)]
    pub(crate) fn set_save_in_progress(&self, save_in_progress: bool) {
        self.imp().save_in_progress.set(save_in_progress);
    }

    #[allow(unused)]
    pub(crate) fn set_output_file(&self, output_file: Option<gio::File>) {
        self.set_property("output-file", output_file.to_value());
    }

    #[allow(unused)]
    pub(crate) fn unsaved_changes(&self) -> bool {
        self.property::<bool>("unsaved-changes")
    }

    #[allow(unused)]
    pub(crate) fn set_unsaved_changes(&self, unsaved_changes: bool) {
        if self.imp().unsaved_changes.get() != unsaved_changes {
            self.set_property("unsaved-changes", unsaved_changes.to_value());
        }
    }

    #[allow(unused)]
    pub(crate) fn empty(&self) -> bool {
        self.property::<bool>("empty")
    }

    #[allow(unused)]
    pub(crate) fn set_empty(&self, empty: bool) {
        if self.imp().empty.get() != empty {
            self.set_property("empty", empty.to_value());
        }
    }

    #[allow(unused)]
    pub(crate) fn touch_drawing(&self) -> bool {
        self.property::<bool>("touch-drawing")
    }

    #[allow(unused)]
    pub(crate) fn set_touch_drawing(&self, touch_drawing: bool) {
        if self.imp().touch_drawing.get() != touch_drawing {
            self.set_property("touch-drawing", touch_drawing.to_value());
        }
    }

    #[allow(unused)]
    fn emit_zoom_changed(&self) {
        self.emit_by_name::<()>("zoom-changed", &[]);
    }

    #[allow(unused)]
    pub(super) fn emit_handle_widget_flags(&self, widget_flags: WidgetFlags) {
        self.emit_by_name::<()>("handle-widget-flags", &[&WidgetFlagsBoxed(widget_flags)]);
    }

    pub(crate) fn engine(&self) -> Rc<RefCell<RnoteEngine>> {
        self.imp().engine.clone()
    }

    pub(crate) fn set_text_preprocessing(&self, enable: bool) {
        if enable {
            self.imp()
                .key_controller
                .set_im_context(Some(&self.imp().key_controller_im_context));
        } else {
            self.imp()
                .key_controller
                .set_im_context(None::<&IMMulticontext>);
        }
    }

    pub(crate) fn save_engine_config(&self, settings: &gio::Settings) -> anyhow::Result<()> {
        let engine_config = self.engine().borrow().export_engine_config_as_json()?;
        Ok(settings.set_string("engine-config", engine_config.as_str())?)
    }

    pub(crate) fn load_engine_config(&self, settings: &gio::Settings) -> anyhow::Result<()> {
        // load engine config
        let engine_config = settings.string("engine-config");
        let widget_flags = match self
            .engine()
            .borrow_mut()
            .import_engine_config_from_json(&engine_config, crate::env::pkg_data_dir().ok())
        {
            Err(e) => {
                if engine_config.is_empty() {
                    // On first app startup the engine config is empty, so we don't log an error
                    log::debug!("did not load `engine-config` from settings, was empty");
                } else {
                    return Err(e);
                }
                None
            }
            Ok(widget_flags) => Some(widget_flags),
        };

        // Avoiding already borrowed
        if let Some(widget_flags) = widget_flags {
            self.emit_handle_widget_flags(widget_flags);
        }
        Ok(())
    }

    pub(crate) fn clear_output_file_monitor(&self) {
        if let Some(old_output_file_monitor) = self.imp().output_file_monitor.take() {
            if let Some(handler) = self.imp().output_file_monitor_changed_handler.take() {
                old_output_file_monitor.disconnect(handler);
            }

            old_output_file_monitor.cancel();
        }
    }

    pub(crate) fn dismiss_output_file_modified_toast(&self) {
        if let Some(output_file_modified_toast) =
            self.imp().output_file_modified_toast_singleton.take()
        {
            output_file_modified_toast.dismiss();
        }
    }

    /// Switches between the regular and the drawing cursor
    pub(crate) fn enable_drawing_cursor(&self, drawing_cursor: bool) {
        if drawing_cursor == self.imp().drawing_cursor_enabled.get() {
            return;
        };
        self.imp().drawing_cursor_enabled.set(drawing_cursor);

        if drawing_cursor {
            self.set_cursor(Some(&*self.imp().drawing_cursor.borrow()));
        } else {
            self.set_cursor(Some(&*self.imp().regular_cursor.borrow()));
        }
    }

    /// The document title for display. Can be used to get a string for the existing / a new save file.
    ///
    /// When there is no output-file, falls back to the "New document" string
    pub(crate) fn doc_title_display(&self) -> String {
        self.output_file()
            .map(|f| {
                f.basename()
                    .and_then(|t| Some(t.file_stem()?.to_string_lossy().to_string()))
                    .unwrap_or_else(|| gettext("- invalid file name -"))
            })
            .unwrap_or_else(|| OUTPUT_FILE_NEW_TITLE.to_string())
    }

    /// The document folder path for display. To get the actual path, use output-file
    ///
    /// When there is no output-file, falls back to the "Draft" string
    pub(crate) fn doc_folderpath_display(&self) -> String {
        self.output_file()
            .map(|f| {
                f.parent()
                    .and_then(|p| Some(p.path()?.display().to_string()))
                    .unwrap_or_else(|| gettext("- invalid folder path -"))
            })
            .unwrap_or_else(|| OUTPUT_FILE_NEW_SUBTITLE.to_string())
    }

    pub(crate) fn create_output_file_monitor(&self, file: &gio::File, appwindow: &RnAppWindow) {
        let new_monitor =
            match file.monitor_file(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE) {
                Ok(output_file_monitor) => output_file_monitor,
                Err(e) => {
                    self.clear_output_file_monitor();
                    log::error!(
                        "creating a file monitor for the new output file failed with Err: {e:?}"
                    );
                    return;
                }
            };

        let new_handler = new_monitor.connect_changed(
            glib::clone!(@weak self as canvas, @weak appwindow => move |_monitor, file, other_file, event| {
                let dispatch_toast_reload_modified_file = || {
                    canvas.set_unsaved_changes(true);

                    appwindow.overlays().dispatch_toast_w_button_singleton(
                        &gettext("Opened file was modified on disk"),
                        &gettext("Reload"),
                        clone!(@weak canvas, @weak appwindow => move |_reload_toast| {
                            glib::MainContext::default().spawn_local(clone!(@weak appwindow => async move {
                                appwindow.overlays().start_pulsing_progressbar();

                                if let Err(e) = canvas.reload_from_disk().await {
                                    appwindow.overlays().dispatch_toast_error(&gettext("Reloading .rnote file from disk failed"));
                                    log::error!("failed to reload current output file, {}", e);
                                }

                                appwindow.overlays().finish_progressbar();
                            }));
                        }),
                        0,
                    &mut canvas.imp().output_file_modified_toast_singleton.borrow_mut());
                };

                log::debug!("canvas with title: `{}` - output-file monitor emitted `changed` - file: {:?}, other_file: {:?}, event: {event:?}, expect_write: {}",
                    canvas.doc_title_display(),
                    file.path(),
                    other_file.map(|f| f.path()),
                    canvas.output_file_expect_write(),
                );

                match event {
                    gio::FileMonitorEvent::Changed => {
                        if canvas.output_file_expect_write() {
                            // => file has been modified due to own save, don't do anything.
                            canvas.set_output_file_expect_write(false);
                            return;
                        }

                        dispatch_toast_reload_modified_file();
                    },
                    gio::FileMonitorEvent::Renamed => {
                        if canvas.output_file_expect_write() {
                            // => file has been modified due to own save, don't do anything.
                            canvas.set_output_file_expect_write(false);
                            return;
                        }

                        // if previous file name was .goutputstream-<hash>, then the file has been replaced using gio.
                        if crate::utils::is_goutputstream_file(file) {
                            // => file has been modified, handle it the same as the Changed event.
                            dispatch_toast_reload_modified_file();
                        } else {
                            // => file has been renamed.

                            // other_file *should* never be none.
                            if other_file.is_none() {
                                canvas.set_unsaved_changes(true);
                            }

                            canvas.set_output_file(other_file.cloned());

                            appwindow.overlays().dispatch_toast_text(&gettext("Opened file was renamed on disk"))
                        }
                    },
                    gio::FileMonitorEvent::Deleted | gio::FileMonitorEvent::MovedOut => {
                        if canvas.output_file_expect_write() {
                            // => file has been modified due to own save, don't do anything.
                            canvas.set_output_file_expect_write(false);
                            return;
                        }

                        canvas.set_unsaved_changes(true);
                        canvas.set_output_file(None);

                        appwindow.overlays().dispatch_toast_text(&gettext("Opened file was moved or deleted on disk"));
                    },
                    _ => {},
                }

                // The expect_write flag can't be cleared after any event has been fired, because some actions emit multiple
                // events - not all of which are handled. The flag should stick around until a handled event has been blocked by it,
                // otherwise it will likely miss its purpose.
            }),
        );

        if let Some(old_monitor) = self
            .imp()
            .output_file_monitor
            .borrow_mut()
            .replace(new_monitor)
        {
            if let Some(old_handler) = self
                .imp()
                .output_file_monitor_changed_handler
                .borrow_mut()
                .replace(new_handler)
            {
                old_monitor.disconnect(old_handler);
            }

            old_monitor.cancel();
        }
    }

    /// Replaces and installs a new file monitor when there is an output file present
    fn reinstall_output_file_monitor(&self, appwindow: &RnAppWindow) {
        if let Some(output_file) = self.output_file() {
            self.create_output_file_monitor(&output_file, appwindow);
        } else {
            self.clear_output_file_monitor();
        }
    }

    /// Initializes for the given appwindow. Usually `init()` is only called once, but since this widget can be moved between appwindows through tabs,
    /// this function also disconnects and replaces all existing old connections
    pub(crate) fn init_reconnect(&self, appwindow: &RnAppWindow) {
        // Initial file monitor, (e.g. needed when reiniting the widget on a new appwindow)
        self.reinstall_output_file_monitor(appwindow);

        let appwindow_output_file = self.connect_notify_local(
            Some("output-file"),
            clone!(@weak appwindow => move |canvas, _pspec| {
                if let Some(output_file) = canvas.output_file(){
                    canvas.create_output_file_monitor(&output_file, &appwindow);
                } else {
                    canvas.clear_output_file_monitor();
                    canvas.dismiss_output_file_modified_toast();
                }

                appwindow.refresh_titles(&appwindow.active_tab());
            }),
        );

        // set scalefactor initially
        self.engine().borrow_mut().camera.scale_factor = f64::from(self.scale_factor());
        // and connect
        let appwindow_scalefactor =
            self.connect_notify_local(Some("scale-factor"), move |canvas, _pspec| {
                let scale_factor = f64::from(canvas.scale_factor());
                canvas.engine().borrow_mut().camera.scale_factor = scale_factor;

                let all_strokes = canvas.engine().borrow_mut().store.stroke_keys_unordered();
                canvas
                    .engine()
                    .borrow_mut()
                    .store
                    .set_rendering_dirty_for_strokes(&all_strokes);

                canvas.regenerate_background_pattern();
                canvas.update_engine_rendering();
            });

        // Update titles when there are changes
        let appwindow_unsaved_changes = self.connect_notify_local(
            Some("unsaved-changes"),
            clone!(@weak appwindow => move |_canvas, _pspec| {
                appwindow.refresh_titles(&appwindow.active_tab());
            }),
        );

        // one per-appwindow property for touch-drawing
        let appwindow_touch_drawing = appwindow
            .bind_property("touch-drawing", self, "touch_drawing")
            .sync_create()
            .build();

        // bind cursors
        let appwindow_regular_cursor = appwindow
            .settings_panel()
            .general_regular_cursor_picker()
            .bind_property("picked", self, "regular-cursor")
            .transform_to(|_, v: Option<String>| v)
            .sync_create()
            .build();

        let appwindow_drawing_cursor = appwindow
            .settings_panel()
            .general_drawing_cursor_picker()
            .bind_property("picked", self, "drawing-cursor")
            .transform_to(|_, v: Option<String>| v)
            .sync_create()
            .build();

        // Drop Target
        let appwindow_drop_target = self.imp().drop_target.connect_drop(
            clone!(@weak self as canvas, @weak appwindow => @default-return false, move |_drop_target, value, x, y| {
                let pos = (canvas.engine().borrow().camera.transform().inverse() *
                    na::point![x,y]).coords;

                if value.is::<gio::File>() {
                    appwindow.open_file_w_dialogs(value.get::<gio::File>().unwrap(), Some(pos), true);

                    return true;
                } else if value.is::<String>() {
                    if let Err(e) = canvas.load_in_text(value.get::<String>().unwrap(), Some(pos)) {
                        log::error!("failed to insert dropped in text, Err: {e:?}");
                    }
                }

                false
            }),
        );

        // update ui when zoom changes
        let appwindow_zoom_changed = self.connect_local("zoom-changed", false, clone!(@weak self as canvas, @weak appwindow => @default-return None, move |_| {
            let total_zoom = canvas.engine().borrow().camera.total_zoom();
            appwindow.mainheader().canvasmenu().zoom_reset_button().set_label(format!("{:.0}%", (100.0 * total_zoom).round()).as_str());
            None
        }));

        // handle widget flags
        let appwindow_handle_widget_flags = self.connect_local(
            "handle-widget-flags",
            false,
            clone!(@weak self as canvas, @weak appwindow => @default-return None, move |args| {
                // first argument is the widget
                let widget_flags = args[1].get::<WidgetFlagsBoxed>().unwrap().0;

                appwindow.handle_widget_flags(widget_flags, &canvas);
                None
            }),
        );

        // Replace old handlers
        let mut handlers = self.imp().handlers.borrow_mut();
        if let Some(old) = handlers
            .appwindow_output_file
            .replace(appwindow_output_file)
        {
            self.disconnect(old);
        }
        if let Some(old) = handlers
            .appwindow_scalefactor
            .replace(appwindow_scalefactor)
        {
            self.disconnect(old);
        }
        if let Some(old) = handlers
            .appwindow_unsaved_changes
            .replace(appwindow_unsaved_changes)
        {
            self.disconnect(old);
        }
        if let Some(old) = handlers
            .appwindow_touch_drawing
            .replace(appwindow_touch_drawing)
        {
            old.unbind();
        }
        if let Some(old) = handlers
            .appwindow_regular_cursor
            .replace(appwindow_regular_cursor)
        {
            old.unbind();
        }
        if let Some(old) = handlers
            .appwindow_drawing_cursor
            .replace(appwindow_drawing_cursor)
        {
            old.unbind();
        }
        if let Some(old) = handlers
            .appwindow_drop_target
            .replace(appwindow_drop_target)
        {
            self.imp().drop_target.disconnect(old);
        }
        if let Some(old) = handlers
            .appwindow_zoom_changed
            .replace(appwindow_zoom_changed)
        {
            self.disconnect(old);
        }
        if let Some(old) = handlers
            .appwindow_handle_widget_flags
            .replace(appwindow_handle_widget_flags)
        {
            self.disconnect(old);
        }
    }

    /// This disconnects all handlers with references to external objects, to prepare moving the widget to another appwindow.
    pub(crate) fn disconnect_handlers(&self, _appwindow: &RnAppWindow) {
        self.clear_output_file_monitor();

        let mut handlers = self.imp().handlers.borrow_mut();
        if let Some(old) = handlers.appwindow_output_file.take() {
            self.disconnect(old);
        }
        if let Some(old) = handlers.appwindow_scalefactor.take() {
            self.disconnect(old);
        }
        if let Some(old) = handlers.appwindow_unsaved_changes.take() {
            self.disconnect(old);
        }
        if let Some(old) = handlers.appwindow_touch_drawing.take() {
            old.unbind();
        }
        if let Some(old) = handlers.appwindow_regular_cursor.take() {
            old.unbind();
        }
        if let Some(old) = handlers.appwindow_drawing_cursor.take() {
            old.unbind();
        }
        if let Some(old) = handlers.appwindow_drop_target.take() {
            self.imp().drop_target.disconnect(old);
        }
        if let Some(old) = handlers.appwindow_zoom_changed.take() {
            self.disconnect(old);
        }
        if let Some(old) = handlers.appwindow_handle_widget_flags.take() {
            self.disconnect(old);
        }

        // tab page connections
        if let Some(old) = handlers.tab_page_output_file.take() {
            old.unbind();
        }
        if let Some(old) = handlers.tab_page_unsaved_changes.take() {
            old.unbind();
        }
    }

    /// When the widget is the child of a tab page, we want to connect their titles, icons, ..
    ///
    /// disconnects existing bindings / handlers to old tab pages.
    pub(crate) fn connect_to_tab_page(&self, page: &adw::TabPage) {
        // update the tab title whenever the canvas output file changes
        let tab_page_output_file = self
            .bind_property("output-file", page, "title")
            .sync_create()
            .transform_to(|b, _output_file: Option<gio::File>| {
                Some(
                    b.source()?
                        .downcast::<RnCanvas>()
                        .unwrap()
                        .doc_title_display(),
                )
            })
            .build();

        // display unsaved changes as icon
        let tab_page_unsaved_changes = self
            .bind_property("unsaved-changes", page, "icon")
            .transform_to(|_, from: bool| {
                Some(from.then_some(gio::ThemedIcon::new("dot-symbolic")))
            })
            .sync_create()
            .build();

        let mut handlers = self.imp().handlers.borrow_mut();
        if let Some(old) = handlers.tab_page_output_file.replace(tab_page_output_file) {
            old.unbind();
        }

        if let Some(old) = handlers
            .tab_page_unsaved_changes
            .replace(tab_page_unsaved_changes)
        {
            old.unbind();
        }
    }

    pub(crate) fn bounds(&self) -> Aabb {
        Aabb::new_positive(
            na::point![0.0, 0.0],
            na::point![f64::from(self.width()), f64::from(self.height())],
        )
    }

    /// gets the current scrollbar adjustment values
    pub(crate) fn adj_values(&self) -> na::Vector2<f64> {
        na::vector![
            self.hadjustment().unwrap().value(),
            self.vadjustment().unwrap().value()
        ]
    }

    /// updates the camera offset and scrollbar adjustment values
    pub(crate) fn update_camera_offset(&self, new_offset: na::Vector2<f64>) {
        // By setting new adjustment values, the callback connected to their `value` property is called,
        // Which is where the engine camera offset, size and the rendering is updated.
        self.hadjustment().unwrap().set_value(new_offset[0]);
        self.vadjustment().unwrap().set_value(new_offset[1]);
    }

    /// returns the current view center coords.
    /// used together with `center_view_around_coords`.
    pub(crate) fn current_view_center_coords(&self) -> na::Vector2<f64> {
        let wrapper = self
            .ancestor(RnCanvasWrapper::static_type())
            .unwrap()
            .downcast::<RnCanvasWrapper>()
            .unwrap();
        let wrapper_size = na::vector![wrapper.width() as f64, wrapper.height() as f64];
        let total_zoom = self.engine().borrow().camera.total_zoom();

        // we need to use the adj values here, because the camera transform doesn't get updated immediately.
        // (happens in the reallocation, which gets queued)
        (self.adj_values() + wrapper_size * 0.5) / total_zoom
    }

    /// centers the view around the given coords.
    /// used together with `current_view_center`.
    ///
    /// engine rendering then needs to be updated.
    pub(crate) fn center_view_around_coords(&self, coords: na::Vector2<f64>) {
        let wrapper = self
            .ancestor(RnCanvasWrapper::static_type())
            .unwrap()
            .downcast::<RnCanvasWrapper>()
            .unwrap();
        let wrapper_size = na::vector![wrapper.width() as f64, wrapper.height() as f64];
        let total_zoom = self.engine().borrow().camera.total_zoom();
        let new_offset = coords * total_zoom - wrapper_size * 0.5;

        self.update_camera_offset(new_offset);
    }

    /// Centering the view to the origin page
    ///
    /// engine rendering then needs to be updated.
    pub(crate) fn return_to_origin_page(&self) {
        let zoom = self.engine().borrow().camera.zoom();
        let Some(parent) = self.parent() else {
            log::debug!("self.parent() is None in `return_to_origin_page()");
            return
        };

        let new_offset =
            if self.engine().borrow().document.format.width * zoom <= f64::from(parent.width()) {
                na::vector![
                    (self.engine().borrow().document.format.width * 0.5 * zoom)
                        - f64::from(parent.width()) * 0.5,
                    -Document::SHADOW_WIDTH * zoom
                ]
            } else {
                // If the zoomed format width is larger than the displayed surface, we zoom to a fixed origin
                na::vector![
                    -Document::SHADOW_WIDTH * zoom,
                    -Document::SHADOW_WIDTH * zoom
                ]
            };

        self.update_camera_offset(new_offset);
    }

    /// zooms and regenerates the canvas and its contents to a new zoom
    /// is private, zooming from other parts of the app should always be done through the "zoom-to-value" action
    fn zoom_to(&self, new_zoom: f64) {
        // Remove the timeout if exists
        if let Some(source_id) = self.imp().handlers.borrow_mut().zoom_timeout.take() {
            source_id.remove();
        }

        self.engine().borrow_mut().camera.set_temporary_zoom(1.0);
        self.engine().borrow_mut().camera.set_zoom(new_zoom);

        let all_strokes = self.engine().borrow_mut().store.stroke_keys_unordered();
        self.engine()
            .borrow_mut()
            .store
            .set_rendering_dirty_for_strokes(&all_strokes);

        self.regenerate_background_pattern();
        self.update_engine_rendering();

        // We need to update the layout managers internal state after zooming
        self.layout_manager()
            .unwrap()
            .downcast::<RnCanvasLayout>()
            .unwrap()
            .update_state(self);
    }

    /// Zooms temporarily and then scale the canvas and its contents to a new zoom after a given time.
    /// Repeated calls to this function reset the timeout.
    /// should only be called from the "zoom-to-value" action.
    pub(crate) fn zoom_temporarily_then_scale_to_after_timeout(&self, new_zoom: f64) {
        if let Some(handler_id) = self.imp().handlers.borrow_mut().zoom_timeout.take() {
            handler_id.remove();
        }

        let old_perm_zoom = self.engine().borrow().camera.zoom();

        // Zoom temporarily
        let new_temp_zoom = new_zoom / old_perm_zoom;
        self.engine()
            .borrow_mut()
            .camera
            .set_temporary_zoom(new_temp_zoom);

        self.emit_zoom_changed();

        // In resize we render the strokes that came into view
        self.queue_resize();

        if let Some(source_id) = self.imp().handlers.borrow_mut().zoom_timeout.replace(
            glib::source::timeout_add_local_once(
                Self::ZOOM_TIMEOUT_TIME,
                clone!(@weak self as canvas => move || {

                    // After timeout zoom permanent
                    canvas.zoom_to(new_zoom);

                    // Removing the timeout id
                    let mut handlers = canvas.imp().handlers.borrow_mut();
                    if let Some(source_id) = handlers.zoom_timeout.take() {
                        source_id.remove();
                    }
                }),
            ),
        ) {
            source_id.remove();
        }
    }

    /// Updates the rendering of the background and strokes that are flagged for rerendering for the current viewport.
    /// To force the rerendering of the background pattern, call regenerate_background_pattern().
    /// To force the rerendering for all strokes in the current viewport, first flag their rendering as dirty.
    pub(crate) fn update_engine_rendering(&self) {
        // background rendering is updated in the layout manager
        self.queue_resize();

        // update content rendering
        self.engine()
            .borrow_mut()
            .update_content_rendering_current_viewport();

        self.queue_draw();
    }

    /// updates the background pattern and rendering for the current viewport.
    /// to be called for example when changing the background pattern or zoom.
    pub(crate) fn regenerate_background_pattern(&self) {
        if let Err(e) = self.engine().borrow_mut().background_regenerate_pattern() {
            log::error!("failed to regenerate background, {e:?}")
        };

        self.queue_draw();
    }
}
