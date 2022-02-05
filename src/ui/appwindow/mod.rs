pub mod appsettings;
pub mod appwindowactions;

mod imp {
    use std::cell::RefCell;
    use std::{cell::Cell, rc::Rc};

    use adw::{prelude::*, subclass::prelude::*};
    use gtk4::{
        gdk, glib, glib::clone, subclass::prelude::*, Box, CompositeTemplate, CssProvider,
        FileChooserNative, Grid, Inhibit, PackType, ScrolledWindow, StyleContext, ToggleButton,
    };
    use gtk4::{gio, GestureDrag, PropagationPhase, Separator, Revealer};
    use once_cell::sync::Lazy;

    use crate::audioplayer::RnoteAudioPlayer;
    use crate::{
        app::RnoteApp, config, ui::canvas::Canvas, ui::dialogs, ui::mainheader::MainHeader,
        ui::penssidebar::PensSideBar, ui::settingspanel::SettingsPanel,
        ui::workspacebrowser::WorkspaceBrowser,
    };

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/appwindow.ui")]
    pub struct RnoteAppWindow {
        pub app_settings: gio::Settings,
        pub audioplayer: Rc<RefCell<RnoteAudioPlayer>>,
        pub filechoosernative: Rc<RefCell<Option<FileChooserNative>>>,

        pub righthanded: Cell<bool>,
        pub pen_sounds: Cell<bool>,

        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub main_grid: TemplateChild<Grid>,
        #[template_child]
        pub canvas_scroller: TemplateChild<ScrolledWindow>,
        #[template_child]
        pub canvas: TemplateChild<Canvas>,
        #[template_child]
        pub settings_panel: TemplateChild<SettingsPanel>,
        #[template_child]
        pub sidebar_scroller: TemplateChild<ScrolledWindow>,
        #[template_child]
        pub sidebar_grid: TemplateChild<Grid>,
        #[template_child]
        pub sidebar_sep: TemplateChild<Separator>,
        #[template_child]
        pub flap: TemplateChild<adw::Flap>,
        #[template_child]
        pub flap_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub flap_header: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub flap_resizer: TemplateChild<gtk4::Box>,
        #[template_child]
        pub flap_resizer_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub workspacebrowser: TemplateChild<WorkspaceBrowser>,
        #[template_child]
        pub flapreveal_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        pub flap_menus_box: TemplateChild<Box>,
        #[template_child]
        pub mainheader: TemplateChild<MainHeader>,
        #[template_child]
        pub narrow_pens_toggles_revealer: TemplateChild<Revealer>,
        #[template_child]
        pub narrow_marker_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        pub narrow_brush_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        pub narrow_shaper_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        pub narrow_eraser_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        pub narrow_selector_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        pub narrow_tools_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        pub penssidebar: TemplateChild<PensSideBar>,
    }

    impl Default for RnoteAppWindow {
        fn default() -> Self {
            Self {
                app_settings: gio::Settings::new(config::APP_ID),
                audioplayer: Rc::new(RefCell::new(RnoteAudioPlayer::default())),
                filechoosernative: Rc::new(RefCell::new(None)),

                righthanded: Cell::new(true),
                pen_sounds: Cell::new(true),

                toast_overlay: TemplateChild::<adw::ToastOverlay>::default(),
                main_grid: TemplateChild::<Grid>::default(),
                canvas_scroller: TemplateChild::<ScrolledWindow>::default(),
                canvas: TemplateChild::<Canvas>::default(),
                settings_panel: TemplateChild::<SettingsPanel>::default(),
                sidebar_scroller: TemplateChild::<ScrolledWindow>::default(),
                sidebar_grid: TemplateChild::<Grid>::default(),
                sidebar_sep: TemplateChild::<Separator>::default(),
                flap: TemplateChild::<adw::Flap>::default(),
                flap_box: TemplateChild::<gtk4::Box>::default(),
                flap_header: TemplateChild::<adw::HeaderBar>::default(),
                flap_resizer: TemplateChild::<gtk4::Box>::default(),
                flap_resizer_box: TemplateChild::<gtk4::Box>::default(),
                workspacebrowser: TemplateChild::<WorkspaceBrowser>::default(),
                flapreveal_toggle: TemplateChild::<ToggleButton>::default(),
                flap_menus_box: TemplateChild::<Box>::default(),
                mainheader: TemplateChild::<MainHeader>::default(),
                narrow_pens_toggles_revealer: TemplateChild::<Revealer>::default(),
                narrow_marker_toggle: TemplateChild::<ToggleButton>::default(),
                narrow_brush_toggle: TemplateChild::<ToggleButton>::default(),
                narrow_shaper_toggle: TemplateChild::<ToggleButton>::default(),
                narrow_eraser_toggle: TemplateChild::<ToggleButton>::default(),
                narrow_selector_toggle: TemplateChild::<ToggleButton>::default(),
                narrow_tools_toggle: TemplateChild::<ToggleButton>::default(),
                penssidebar: TemplateChild::<PensSideBar>::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RnoteAppWindow {
        const NAME: &'static str = "RnoteAppWindow";
        type Type = super::RnoteAppWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RnoteAppWindow {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let _windowsettings = obj.settings();

            if config::PROFILE == "devel" {
                obj.add_css_class("devel");
            }

            // Load the application css
            let css = CssProvider::new();
            css.load_from_resource((String::from(config::APP_IDPATH) + "ui/custom.css").as_str());

            let display = gdk::Display::default().unwrap();
            StyleContext::add_provider_for_display(
                &display,
                &css,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );

            self.setup_flap(obj);

            // pens narrow toggles
            self.narrow_marker_toggle.connect_toggled(clone!(@weak obj as appwindow => move |narrow_marker_toggle| {
                if narrow_marker_toggle.is_active() {
                    adw::prelude::ActionGroupExt::activate_action(&appwindow, "current-pen", Some(&"marker_style".to_variant()));
                }
            }));

            self.narrow_brush_toggle.connect_toggled(clone!(@weak obj as appwindow => move |narrow_brush_toggle| {
                if narrow_brush_toggle.is_active() {
                    adw::prelude::ActionGroupExt::activate_action(&appwindow, "current-pen", Some(&"brush_style".to_variant()));
                }
            }));

            self.narrow_shaper_toggle.connect_toggled(clone!(@weak obj as appwindow => move |narrow_shaper_toggle| {
                if narrow_shaper_toggle.is_active() {
                    adw::prelude::ActionGroupExt::activate_action(&appwindow, "current-pen", Some(&"shaper_style".to_variant()));
                }
            }));

            self.narrow_eraser_toggle.connect_toggled(clone!(@weak obj as appwindow => move |narrow_eraser_toggle| {
                if narrow_eraser_toggle.is_active() {
                    adw::prelude::ActionGroupExt::activate_action(&appwindow, "current-pen", Some(&"eraser_style".to_variant()));
                }
            }));

            self.narrow_selector_toggle.connect_toggled(clone!(@weak obj as appwindow => move |narrow_selector_toggle| {
                if narrow_selector_toggle.is_active() {
                    adw::prelude::ActionGroupExt::activate_action(&appwindow, "current-pen", Some(&"selector_style".to_variant()));
                }
            }));

            self.narrow_tools_toggle.connect_toggled(clone!(@weak obj as appwindow => move |narrow_tools_toggle| {
                if narrow_tools_toggle.is_active() {
                    adw::prelude::ActionGroupExt::activate_action(&appwindow, "current-pen", Some(&"tools_style".to_variant()));
                }
            }));
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    // Pen sounds
                    glib::ParamSpecBoolean::new(
                        "righthanded",
                        "righthanded",
                        "righthanded",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    // Pen sounds
                    glib::ParamSpecBoolean::new(
                        "pen-sounds",
                        "pen-sounds",
                        "pen-sounds",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "righthanded" => self.righthanded.get().to_value(),
                "pen-sounds" => self.pen_sounds.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "righthanded" => {
                    let righthanded = value
                        .get::<bool>()
                        .expect("The value needs to be of type `bool`.");

                    self.righthanded.replace(righthanded);
                }
                "pen-sounds" => {
                    let pen_sounds = value
                        .get::<bool>()
                        .expect("The value needs to be of type `bool`.");

                    self.pen_sounds.replace(pen_sounds);
                    self.audioplayer.borrow_mut().set_enabled(pen_sounds);
                }
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for RnoteAppWindow {}

    impl WindowImpl for RnoteAppWindow {
        // Save window state right before the window will be closed
        fn close_request(&self, obj: &Self::Type) -> Inhibit {
            // Save current sheet
            if obj
                .application()
                .unwrap()
                .downcast::<RnoteApp>()
                .unwrap()
                .unsaved_changes()
            {
                dialogs::dialog_quit_save(obj);
            } else {
                obj.close();
            }

            // Inhibit (Overwrite) the default handler. This handler is then responsible for destoying the window.
            Inhibit(true)
        }
    }

    impl ApplicationWindowImpl for RnoteAppWindow {}
    impl AdwWindowImpl for RnoteAppWindow {}
    impl AdwApplicationWindowImpl for RnoteAppWindow {}

    impl RnoteAppWindow {
        // Setting up the sidebar flap
        fn setup_flap(&self, obj: &super::RnoteAppWindow) {
            let flap = self.flap.get();
            let flap_box = self.flap_box.get();
            let flap_resizer = self.flap_resizer.get();
            let flap_resizer_box = self.flap_resizer_box.get();
            let workspace_headerbar = self.flap_header.get();
            let flapreveal_toggle = self.flapreveal_toggle.get();

            flap.set_locked(true);
            flap.set_fold_policy(adw::FlapFoldPolicy::Auto);

            let expanded_revealed = Rc::new(Cell::new(flap.reveals_flap()));

            self.flapreveal_toggle
                .bind_property("active", &flap, "reveal-flap")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.flapreveal_toggle.connect_toggled(
                clone!(@weak flap, @strong expanded_revealed => move |flapreveal_toggle| {
                    flap.set_reveal_flap(flapreveal_toggle.is_active());
                    if !flap.is_folded() {
                        expanded_revealed.set(flapreveal_toggle.is_active());
                    }
                }),
            );

            self.flap
                .connect_folded_notify(clone!(@weak obj as appwindow, @strong expanded_revealed, @weak flapreveal_toggle, @weak workspace_headerbar => move |flap| {
                    if appwindow.mainheader().appmenu().parent().is_some() {
                        appwindow.mainheader().appmenu().unparent();
                    }
                    if flap.reveals_flap() && !flap.is_folded() {
                        appwindow.flap_menus_box().append(&appwindow.mainheader().appmenu());
                    } else {
                        appwindow.mainheader().menus_box().append(&appwindow.mainheader().appmenu());
                    }

                    if flap.is_folded() {
                        flapreveal_toggle.set_active(false);
                    } else if expanded_revealed.get() || flap.reveals_flap() {
                        expanded_revealed.set(true);
                        flapreveal_toggle.set_active(true);
                    }

                    if flap.flap_position() == PackType::Start {
                        workspace_headerbar.set_show_start_title_buttons(flap.reveals_flap());
                        workspace_headerbar.set_show_end_title_buttons(false);
                    } else if flap.flap_position() == PackType::End {
                        workspace_headerbar.set_show_start_title_buttons(false);
                        workspace_headerbar.set_show_end_title_buttons(flap.reveals_flap());
                    }
                }));

            self.flap
                .connect_reveal_flap_notify(clone!(@weak obj as appwindow, @weak workspace_headerbar => move |flap| {
                    if appwindow.mainheader().appmenu().parent().is_some() {
                        appwindow.mainheader().appmenu().unparent();
                    }
                    if flap.reveals_flap() && !flap.is_folded() {
                        appwindow.flap_menus_box().append(&appwindow.mainheader().appmenu());
                    } else {
                        appwindow.mainheader().menus_box().append(&appwindow.mainheader().appmenu());
                    }

                    if flap.flap_position() == PackType::Start {
                        workspace_headerbar.set_show_start_title_buttons(flap.reveals_flap());
                        workspace_headerbar.set_show_end_title_buttons(false);
                    } else if flap.flap_position() == PackType::End {
                        workspace_headerbar.set_show_start_title_buttons(false);
                        workspace_headerbar.set_show_end_title_buttons(flap.reveals_flap());
                    }
                }));

            self.flap.connect_flap_position_notify(
                clone!(@weak flap_resizer_box, @weak flap_resizer, @weak flap_box, @weak workspace_headerbar, @strong expanded_revealed => move |flap| {
                    if flap.flap_position() == PackType::Start {
                        workspace_headerbar.set_show_start_title_buttons(flap.reveals_flap());
                        workspace_headerbar.set_show_end_title_buttons(false);

                        flap_resizer_box.reorder_child_after(&flap_resizer, Some(&flap_box));
                    } else if flap.flap_position() == PackType::End {
                        workspace_headerbar.set_show_start_title_buttons(false);
                        workspace_headerbar.set_show_end_title_buttons(flap.reveals_flap());

                        flap_resizer_box.reorder_child_after(&flap_box, Some(&flap_resizer));
                    }
                }),
            );

            // Resizing the flap contents
            let resizer_drag_gesture = GestureDrag::builder()
                .name("resizer_drag_gesture")
                .propagation_phase(PropagationPhase::Capture)
                .build();
            self.flap_resizer.add_controller(&resizer_drag_gesture);

            // Dirty hack to stop resizing when it is switching from non-folded to folded or vice versa (else gtk crashes)
            let prev_folded = Rc::new(Cell::new(self.flap.get().is_folded()));

            resizer_drag_gesture.connect_drag_begin(clone!(@strong prev_folded, @weak flap, @weak flap_box => move |_resizer_drag_gesture, _x , _y| {
                    prev_folded.set(flap.is_folded());
            }));

            resizer_drag_gesture.connect_drag_update(clone!(@weak obj, @strong prev_folded, @weak flap, @weak flap_box, @weak flapreveal_toggle => move |_resizer_drag_gesture, x , _y| {
                if flap.is_folded() == prev_folded.get() {
                    // Set BEFORE new width request
                    prev_folded.set(flap.is_folded());

                    let new_width = if flap.flap_position() == PackType::Start {
                        flap_box.width() + x.ceil() as i32
                    } else {
                        flap_box.width() - x.floor() as i32
                    };
                    if new_width > 0 && new_width < obj.mainheader().width() - 64 {
                        flap_box.set_width_request(new_width);
                    }
                } else if flap.is_folded() {
                    flapreveal_toggle.set_active(true);
                }
            }));

            self.flap_resizer.set_cursor(
                gdk::Cursor::from_name(
                    "col-resize",
                    gdk::Cursor::from_name("default", None).as_ref(),
                )
                .as_ref(),
            );
        }
    }
}

use std::{
    cell::{Cell, RefCell},
    path::Path,
    rc::Rc,
};

use adw::prelude::*;
use gtk4::Revealer;
use gtk4::{
    gdk, gio, glib, glib::clone, subclass::prelude::*, Application, Box, EventControllerScroll,
    EventControllerScrollFlags, EventSequenceState, FileChooserNative, GestureDrag, GestureZoom,
    Grid, IconTheme, Inhibit, PropagationPhase, ScrolledWindow, Separator, ToggleButton,
};

use crate::{
    app::RnoteApp,
    audioplayer::RnoteAudioPlayer,
    config,
    strokes::{bitmapimage::BitmapImage, vectorimage::VectorImage},
    strokesstate::{StateTask, StrokesState},
    ui::canvas::Canvas,
    ui::penssidebar::PensSideBar,
    ui::settingspanel::SettingsPanel,
    ui::workspacebrowser::WorkspaceBrowser,
    ui::{dialogs, mainheader::MainHeader},
    utils,
};

// The renderer as a global singleton

glib::wrapper! {
    pub struct RnoteAppWindow(ObjectSubclass<imp::RnoteAppWindow>)
        @extends gtk4::Widget, gtk4::Window, adw::Window, gtk4::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl RnoteAppWindow {
    pub const CANVAS_ZOOMGESTURE_THRESHOLD: f64 = 0.005; // Sets the delta threshold (eg. 0.01 = 1% ) when to update the canvas when doing a zoom gesture
    pub const CANVAS_ZOOM_SCROLL_STEP: f64 = 0.1; // Sets the canvas zoom scroll step in % for one unit of the event controller delta

    pub fn new(app: &Application) -> Self {
        glib::Object::new(&[("application", app)]).expect("Failed to create `RnoteAppWindow`.")
    }

    /// Called to close the window
    pub fn close(&self) {
        // Saving all state
        if let Err(err) = self.save_to_settings() {
            log::error!("Failed to save appwindow to settings, with Err `{}`", &err);
        }

        // Setting all gstreamer pipelines state to Null
        self.audioplayer().borrow_mut().set_states_null();

        // Closing the state tasks channel receiver
        if let Some(tasks_tx) = self
            .canvas()
            .sheet()
            .borrow()
            .strokes_state
            .tasks_tx
            .as_ref()
        {
            let _ = tasks_tx.send(StateTask::Quit);
        }

        if let Some(source) = self
            .canvas()
            .sheet()
            .borrow_mut()
            .strokes_state
            .channel_source
            .take()
        {
            source.destroy();
        }

        self.destroy();
    }

    pub fn app_settings(&self) -> gio::Settings {
        self.imp().app_settings.clone()
    }

    pub fn filechoosernative(&self) -> Rc<RefCell<Option<FileChooserNative>>> {
        imp::RnoteAppWindow::from_instance(self)
            .filechoosernative
            .clone()
    }

    pub fn toast_overlay(&self) -> adw::ToastOverlay {
        imp::RnoteAppWindow::from_instance(self).toast_overlay.get()
    }

    pub fn main_grid(&self) -> Grid {
        imp::RnoteAppWindow::from_instance(self).main_grid.get()
    }

    pub fn canvas_scroller(&self) -> ScrolledWindow {
        imp::RnoteAppWindow::from_instance(self)
            .canvas_scroller
            .get()
    }

    pub fn canvas(&self) -> Canvas {
        imp::RnoteAppWindow::from_instance(self).canvas.get()
    }

    pub fn settings_panel(&self) -> SettingsPanel {
        imp::RnoteAppWindow::from_instance(self)
            .settings_panel
            .get()
    }

    pub fn sidebar_scroller(&self) -> ScrolledWindow {
        imp::RnoteAppWindow::from_instance(self)
            .sidebar_scroller
            .get()
    }

    pub fn sidebar_grid(&self) -> Grid {
        imp::RnoteAppWindow::from_instance(self).sidebar_grid.get()
    }

    pub fn sidebar_sep(&self) -> Separator {
        imp::RnoteAppWindow::from_instance(self).sidebar_sep.get()
    }

    pub fn flap_header(&self) -> adw::HeaderBar {
        imp::RnoteAppWindow::from_instance(self).flap_header.get()
    }

    pub fn workspacebrowser(&self) -> WorkspaceBrowser {
        imp::RnoteAppWindow::from_instance(self)
            .workspacebrowser
            .get()
    }

    pub fn flap(&self) -> adw::Flap {
        imp::RnoteAppWindow::from_instance(self).flap.get()
    }

    pub fn flapreveal_toggle(&self) -> ToggleButton {
        imp::RnoteAppWindow::from_instance(self)
            .flapreveal_toggle
            .get()
    }

    pub fn flap_menus_box(&self) -> Box {
        imp::RnoteAppWindow::from_instance(self)
            .flap_menus_box
            .get()
    }

    pub fn mainheader(&self) -> MainHeader {
        imp::RnoteAppWindow::from_instance(self).mainheader.get()
    }

    pub fn narrow_pens_toggles_revealer(&self) -> Revealer {
        imp::RnoteAppWindow::from_instance(self)
            .narrow_pens_toggles_revealer
            .get()
    }

    pub fn narrow_marker_toggle(&self) -> ToggleButton {
        imp::RnoteAppWindow::from_instance(self).narrow_marker_toggle.get()
    }

    pub fn narrow_brush_toggle(&self) -> ToggleButton {
        imp::RnoteAppWindow::from_instance(self).narrow_brush_toggle.get()
    }

    pub fn narrow_shaper_toggle(&self) -> ToggleButton {
        imp::RnoteAppWindow::from_instance(self).narrow_shaper_toggle.get()
    }

    pub fn narrow_eraser_toggle(&self) -> ToggleButton {
        imp::RnoteAppWindow::from_instance(self).narrow_eraser_toggle.get()
    }

    pub fn narrow_selector_toggle(&self) -> ToggleButton {
        imp::RnoteAppWindow::from_instance(self).narrow_selector_toggle.get()
    }

    pub fn narrow_tools_toggle(&self) -> ToggleButton {
        imp::RnoteAppWindow::from_instance(self).narrow_tools_toggle.get()
    }

    pub fn penssidebar(&self) -> PensSideBar {
        imp::RnoteAppWindow::from_instance(self).penssidebar.get()
    }

    pub fn audioplayer(&self) -> Rc<RefCell<RnoteAudioPlayer>> {
        imp::RnoteAppWindow::from_instance(self).audioplayer.clone()
    }

    pub fn save_window_size(&self) -> Result<(), anyhow::Error> {
        let mut width = self.width();
        let mut height = self.height();

        // Window would grow without subtracting this size. Why? I dont know
        width -= 122;
        height -= 122;

        self.app_settings().set_int("window-width", width)?;
        self.app_settings().set_int("window-height", height)?;
        self.app_settings()
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    pub fn load_window_size(&self) {
        let width = self.app_settings().int("window-width");
        let height = self.app_settings().int("window-height");
        let is_maximized = self.app_settings().boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    // Must be called after application is associated with it else it fails
    pub fn init(&self) {
        if let Err(e) = self.imp().audioplayer.borrow_mut().init(self) {
            log::error!("failed to init audio_player with Err {}", e);
        }
        self.imp().workspacebrowser.get().init(self);
        self.imp().settings_panel.get().init(self);
        self.imp().mainheader.get().init(self);
        self.imp().mainheader.get().canvasmenu().init(self);
        self.imp().mainheader.get().appmenu().init(self);
        self.imp().penssidebar.get().init(self);
        self.imp().penssidebar.get().marker_page().init(self);
        self.imp().penssidebar.get().brush_page().init(self);
        self.imp().penssidebar.get().shaper_page().init(self);
        self.imp().penssidebar.get().eraser_page().init(self);
        self.imp().penssidebar.get().selector_page().init(self);
        self.imp().penssidebar.get().tools_page().init(self);
        self.imp().canvas.get().init(self);
        StrokesState::init(self);
        self.imp().canvas.get().selection_modifier().init(self);

        // add icon theme resource path because automatic lookup does not work in the devel build.
        let app_icon_theme = IconTheme::for_display(&self.display());
        app_icon_theme.add_resource_path((String::from(config::APP_IDPATH) + "icons").as_str());

        // zoom scrolling with <ctrl> + scroll
        let canvas_zoom_scroll_controller = EventControllerScroll::builder()
            .name("canvas_zoom_scroll_controller")
            .propagation_phase(PropagationPhase::Capture)
            .flags(EventControllerScrollFlags::VERTICAL)
            .build();

        canvas_zoom_scroll_controller.connect_scroll(clone!(@weak self as appwindow => @default-return Inhibit(false), move |zoom_scroll_controller, _dx, dy| {
            let total_zoom = appwindow.canvas().total_zoom();
            if zoom_scroll_controller.current_event_state() == gdk::ModifierType::CONTROL_MASK {
                let delta = dy * Self::CANVAS_ZOOM_SCROLL_STEP * total_zoom;
                let new_zoom = total_zoom - delta;

                // the sheet position BEFORE zooming
                let sheet_center_pos = appwindow.canvas().transform_canvas_coords_to_sheet_coords(
                    na::vector![
                        f64::from(appwindow.canvas_scroller().width()) * 0.5,
                        f64::from(appwindow.canvas_scroller().height()) * 0.5
                    ]);

                appwindow.canvas().zoom_temporarily_then_scale_to_after_timeout(new_zoom, Canvas::ZOOM_TIMEOUT_TIME);

                // Reposition scroller center to the previous sheet position
                appwindow.canvas().center_around_coord_on_sheet(sheet_center_pos);
                // Stop event propagation
                Inhibit(true)
            } else {
                Inhibit(false)
            }
        }));
        self.canvas_scroller()
            .add_controller(&canvas_zoom_scroll_controller);

        // Move Canvas with touch gesture
        let canvas_touch_drag_gesture = GestureDrag::builder()
            .name("canvas_touch_drag_gesture")
            .touch_only(true)
            .propagation_phase(PropagationPhase::Bubble)
            .build();

        let touch_drag_start_x = Rc::new(Cell::new(0.0));
        let touch_drag_start_y = Rc::new(Cell::new(0.0));

        canvas_touch_drag_gesture.connect_drag_begin(clone!(@strong touch_drag_start_x, @strong touch_drag_start_y, @weak self as appwindow => move |_canvas_touch_drag_gesture, _x, _y| {
            touch_drag_start_x.set(appwindow.canvas().hadjustment().unwrap().value());
            touch_drag_start_y.set(appwindow.canvas().vadjustment().unwrap().value());
        }));
        canvas_touch_drag_gesture.connect_drag_update(clone!(@strong touch_drag_start_x, @strong touch_drag_start_y, @weak self as appwindow => move |_canvas_touch_drag_gesture, x, y| {
            appwindow.canvas().hadjustment().unwrap().set_value(touch_drag_start_x.get() - x);
            appwindow.canvas().vadjustment().unwrap().set_value(touch_drag_start_y.get() - y);
        }));
        self.canvas_scroller()
            .add_controller(&canvas_touch_drag_gesture);

        // Move Canvas with middle mouse button
        let canvas_mouse_middle_gesture = GestureDrag::builder()
            .name("canvas_mouse_drag_gesture")
            .button(gdk::BUTTON_MIDDLE)
            .propagation_phase(PropagationPhase::Capture)
            .build();
        self.canvas_scroller()
            .add_controller(&canvas_mouse_middle_gesture);

        let mouse_drag_start_x = Rc::new(Cell::new(0.0));
        let mouse_drag_start_y = Rc::new(Cell::new(0.0));

        canvas_mouse_middle_gesture.connect_drag_begin(clone!(@strong mouse_drag_start_x, @strong mouse_drag_start_y, @weak self as appwindow => move |_canvas_mouse_drag_gesture, _x, _y| {
            mouse_drag_start_x.set(appwindow.canvas().hadjustment().unwrap().value());
            mouse_drag_start_y.set(appwindow.canvas().vadjustment().unwrap().value());
        }));
        canvas_mouse_middle_gesture.connect_drag_update(clone!(@strong mouse_drag_start_x, @strong mouse_drag_start_y, @weak self as appwindow => move |_canvas_mouse_drag_gesture, x, y| {
            appwindow.canvas().hadjustment().unwrap().set_value(mouse_drag_start_x.get() - x);
            appwindow.canvas().vadjustment().unwrap().set_value(mouse_drag_start_y.get() - y);
        }));

        // Move Canvas by dragging in empty area
        let canvas_mouse_drag_empty_area_gesture = GestureDrag::builder()
            .name("canvas_mouse_drag_gesture")
            .button(gdk::BUTTON_PRIMARY)
            .propagation_phase(PropagationPhase::Bubble)
            .build();
        self.canvas_scroller()
            .add_controller(&canvas_mouse_drag_empty_area_gesture);

        let mouse_drag_start_x = Rc::new(Cell::new(0.0));
        let mouse_drag_start_y = Rc::new(Cell::new(0.0));

        canvas_mouse_drag_empty_area_gesture.connect_drag_begin(clone!(@strong mouse_drag_start_x, @strong mouse_drag_start_y, @weak self as appwindow => move |_canvas_mouse_drag_gesture, _x, _y| {
            mouse_drag_start_x.set(appwindow.canvas().hadjustment().unwrap().value());
            mouse_drag_start_y.set(appwindow.canvas().vadjustment().unwrap().value());
        }));
        canvas_mouse_drag_empty_area_gesture.connect_drag_update(clone!(@strong mouse_drag_start_x, @strong mouse_drag_start_y, @weak self as appwindow => move |_canvas_mouse_drag_gesture, x, y| {
            appwindow.canvas().hadjustment().unwrap().set_value(mouse_drag_start_x.get() - x);
            appwindow.canvas().vadjustment().unwrap().set_value(mouse_drag_start_y.get() - y);
        }));

        // Canvas gesture zooming with preview and dragging
        let canvas_zoom_gesture = GestureZoom::builder()
            .name("canvas_zoom_gesture")
            .propagation_phase(PropagationPhase::Capture)
            .build();
        self.canvas_scroller().add_controller(&canvas_zoom_gesture);

        let prev_zoom = Rc::new(Cell::new(1_f64));
        let zoom_begin = Rc::new(Cell::new(1_f64));
        let new_zoom = Rc::new(Cell::new(self.canvas().zoom()));
        let zoomgesture_canvasscroller_start_pos = Rc::new(Cell::new((0.0, 0.0)));
        let zoomgesture_bbcenter_start: Rc<Cell<Option<(f64, f64)>>> = Rc::new(Cell::new(None));

        canvas_zoom_gesture.connect_begin(clone!(
            @strong zoom_begin,
            @strong prev_zoom,
            @strong new_zoom,
            @strong zoomgesture_canvasscroller_start_pos,
            @strong zoomgesture_bbcenter_start,
            @weak self as appwindow => move |canvas_zoom_gesture, _eventsequence| {
                canvas_zoom_gesture.set_state(EventSequenceState::Claimed);

                zoom_begin.set(appwindow.canvas().zoom());
                new_zoom.set(appwindow.canvas().zoom());
                prev_zoom.set(1.0);

                zoomgesture_canvasscroller_start_pos.set(
                    (
                        appwindow.canvas().hadjustment().unwrap().value(),
                        appwindow.canvas().vadjustment().unwrap().value()
                    )
                );
                if let Some(bbcenter) = canvas_zoom_gesture.bounding_box_center() {
                    zoomgesture_bbcenter_start.set(Some(
                        bbcenter
                    ));
                }
        }));

        canvas_zoom_gesture.connect_scale_changed(
            clone!(@strong zoom_begin, @strong new_zoom, @strong prev_zoom, @strong zoomgesture_canvasscroller_start_pos, @strong zoomgesture_bbcenter_start, @weak self as appwindow => move |canvas_zoom_gesture, zoom| {
                let new_zoom = if zoom_begin.get() * zoom > Canvas::ZOOM_MAX || zoom_begin.get() * zoom < Canvas::ZOOM_MIN {
                    prev_zoom.get()
                } else {
                    new_zoom.set(zoom_begin.get() * zoom);

                    appwindow.canvas().zoom_temporarily_then_scale_to_after_timeout(new_zoom.get(), Canvas::ZOOM_TIMEOUT_TIME);

                    prev_zoom.set(zoom);
                    zoom
                };

                if let Some(bbcenter) = canvas_zoom_gesture.bounding_box_center() {
                    if let Some(bbcenter_start) = zoomgesture_bbcenter_start.get() {
                        let bbcenter_delta = (
                            bbcenter.0 - bbcenter_start.0 * new_zoom,
                            bbcenter.1 - bbcenter_start.1 * new_zoom
                        );

                        appwindow.canvas().hadjustment().unwrap().set_value(
                            zoomgesture_canvasscroller_start_pos.get().0 * new_zoom - bbcenter_delta.0
                        );
                        appwindow.canvas().vadjustment().unwrap().set_value(
                            zoomgesture_canvasscroller_start_pos.get().1 * new_zoom - bbcenter_delta.1
                        );
                    } else {
                        // Setting the start position if connect_scale_start didn't set it
                        zoomgesture_bbcenter_start.set(Some(
                            bbcenter
                        ));
                    }
                }
            }),
        );

        canvas_zoom_gesture.connect_cancel(
            clone!(@strong zoomgesture_bbcenter_start, @weak self as appwindow => move |canvas_zoom_gesture, _eventsequence| {
                zoomgesture_bbcenter_start.set(None);

                canvas_zoom_gesture.set_state(EventSequenceState::Denied);
            }),
        );

        canvas_zoom_gesture.connect_end(
            clone!(@strong new_zoom, @strong zoomgesture_bbcenter_start, @weak self as appwindow => move |canvas_zoom_gesture, _eventsequence| {
                zoomgesture_bbcenter_start.set(None);
                appwindow.canvas().zoom_to(new_zoom.get());

                canvas_zoom_gesture.set_state(EventSequenceState::Denied);
            }),
        );

        // Gesture Grouping
        canvas_mouse_middle_gesture.group_with(&canvas_touch_drag_gesture);
        canvas_mouse_drag_empty_area_gesture.group_with(&canvas_touch_drag_gesture);
        canvas_zoom_gesture.group_with(&canvas_touch_drag_gesture);

        // actions and settings AFTER widget callback declarations
        self.setup_actions();
        self.setup_action_accels();
        self.setup_settings();

        if let Err(e) = self.load_settings() {
            log::debug!("failed to load appwindow settings with Err `{}`", e);
        }

        // Loading in input file, if Some
        if let Some(input_file) = self
            .application()
            .unwrap()
            .downcast::<RnoteApp>()
            .unwrap()
            .input_file()
        {
            if self
                .application()
                .unwrap()
                .downcast::<RnoteApp>()
                .unwrap()
                .unsaved_changes()
            {
                dialogs::dialog_open_overwrite(self);
            } else if let Err(e) = self.load_in_file(&input_file, None) {
                log::error!("failed to load in input file, {}", e);
            }
        }
    }

    pub fn open_file_w_dialogs(&self, file: &gio::File, target_pos: Option<na::Vector2<f64>>) {
        let app = self.application().unwrap().downcast::<RnoteApp>().unwrap();
        match utils::FileType::lookup_file_type(file) {
            utils::FileType::RnoteFile | utils::FileType::XoppFile => {
                // Setting input file to hand it to the open overwrite dialog
                app.set_input_file(Some(file.clone()));

                if app.unsaved_changes() {
                    dialogs::dialog_open_overwrite(self);
                } else if let Err(e) = self.load_in_file(file, target_pos) {
                    log::error!(
                        "failed to load in file with FileType::RnoteFile | FileType::XoppFile, {}",
                        e
                    );
                }
            }
            utils::FileType::VectorImageFile
            | utils::FileType::BitmapImageFile
            | utils::FileType::PdfFile => {
                if let Err(e) = self.load_in_file(file, target_pos) {
                    log::error!("failed to load in file with FileType::VectorImageFile / FileType::BitmapImageFile / FileType::Pdf, {}", e);
                }
            }
            utils::FileType::Folder => {
                if let Some(path) = file.path() {
                    self.workspacebrowser().set_primary_path(Some(&path));
                }
            }
            utils::FileType::UnknownFile => {
                log::warn!("tried to open unsupported file type.");
            }
        }
    }

    /// Loads in a file of any supported type into the current sheet.
    pub fn load_in_file(
        &self,
        file: &gio::File,
        target_pos: Option<na::Vector2<f64>>,
    ) -> Result<(), anyhow::Error> {
        let main_cx = glib::MainContext::default();
        let app = self.application().unwrap().downcast::<RnoteApp>().unwrap();
        let file = file.clone();

        match utils::FileType::lookup_file_type(&file) {
            utils::FileType::RnoteFile => {
                main_cx.spawn_local(clone!(@weak self as appwindow => async move {
                    let result = file.load_bytes_future().await;
                    if let Ok((file_bytes, _)) = result {
                        if let Err(e) = appwindow.load_in_rnote_bytes(file_bytes, file.path()) {
                            log::error!(
                                "load_in_rnote_bytes() failed in load_in_file() with Err {}",
                                e
                            );
                        }
                    }
                }));
            }
            utils::FileType::XoppFile => {
                main_cx.spawn_local(clone!(@weak self as appwindow => async move {
                    let result = file.load_bytes_future().await;
                    if let Ok((file_bytes, _)) = result {
                        if let Err(e) = appwindow.load_in_xopp_bytes(file_bytes, file.path()) {
                            log::error!(
                                "load_in_xopp_bytes() failed in load_in_file() with Err {}",
                                e
                            );
                        }
                    }
                }));
            }
            utils::FileType::VectorImageFile => {
                main_cx.spawn_local(clone!(@weak self as appwindow => async move {
                    let result = file.load_bytes_future().await;
                    if let Ok((file_bytes, _)) = result {
                        if let Err(e) = appwindow.load_in_vectorimage_bytes(file_bytes, target_pos) {
                            log::error!(
                                "load_in_rnote_bytes() failed in load_in_file() with Err {}",
                                e
                            );
                        }
                    }
                }));
            }
            utils::FileType::BitmapImageFile => {
                main_cx.spawn_local(clone!(@weak self as appwindow => async move {
                    let result = file.load_bytes_future().await;
                    if let Ok((file_bytes, _)) = result {
                        if let Err(e) = appwindow.load_in_bitmapimage_bytes(file_bytes, target_pos) {
                            log::error!(
                                "load_in_rnote_bytes() failed in load_in_file() with Err {}",
                                e
                            );
                        }
                    }
                }));
            }
            utils::FileType::PdfFile => {
                main_cx.spawn_local(clone!(@weak self as appwindow => async move {
                    let result = file.load_bytes_future().await;
                    if let Ok((file_bytes, _)) = result {
                        if let Err(e) = appwindow.load_in_pdf_bytes(file_bytes, target_pos) {
                            log::error!(
                                "load_in_rnote_bytes() failed in load_in_file() with Err {}",
                                e
                            );
                        }
                    }
                }));
            }
            utils::FileType::Folder => {
                log::warn!("tried to open folder as sheet.");
            }
            utils::FileType::UnknownFile => {
                log::warn!("tried to open a unsupported file type.");
                app.set_input_file(None);
            }
        }

        Ok(())
    }

    pub fn load_in_rnote_bytes<P>(
        &self,
        bytes: glib::Bytes,
        path: Option<P>,
    ) -> Result<(), anyhow::Error>
    where
        P: AsRef<Path>,
    {
        let app = self.application().unwrap().downcast::<RnoteApp>().unwrap();
        self.canvas()
            .sheet()
            .borrow_mut()
            .open_sheet_from_rnote_bytes(bytes)?;

        // Loading the sheet properties into the format settings panel
        self.settings_panel().refresh_for_sheet(self);

        self.canvas().set_unsaved_changes(false);
        app.set_input_file(None);
        if let Some(path) = path {
            let file = gio::File::for_path(path);
            app.set_output_file(Some(&file), self);
        }

        self.canvas().set_unsaved_changes(false);
        self.canvas().set_empty(false);
        self.canvas().regenerate_background(false);
        self.canvas().regenerate_content(true, true);

        self.canvas()
            .selection_modifier()
            .update_state(&self.canvas());

        adw::prelude::ActionGroupExt::activate_action(self, "refresh-ui-for-sheet", None);

        Ok(())
    }

    pub fn load_in_xopp_bytes<P>(
        &self,
        bytes: glib::Bytes,
        _path: Option<P>,
    ) -> Result<(), anyhow::Error>
    where
        P: AsRef<Path>,
    {
        let app = self.application().unwrap().downcast::<RnoteApp>().unwrap();
        self.canvas()
            .sheet()
            .borrow_mut()
            .open_from_xopp_bytes(bytes)?;

        // Loading the sheet properties into the format settings panel
        self.settings_panel().refresh_for_sheet(self);

        app.set_input_file(None);
        app.set_output_file(None, self);

        self.canvas().set_unsaved_changes(true);
        self.canvas().set_empty(false);
        self.canvas().regenerate_background(false);
        self.canvas().regenerate_content(true, true);

        self.canvas()
            .selection_modifier()
            .update_state(&self.canvas());

        adw::prelude::ActionGroupExt::activate_action(self, "refresh-ui-for-sheet", None);

        Ok(())
    }

    pub fn load_in_vectorimage_bytes(
        &self,
        bytes: glib::Bytes,
        target_pos: Option<na::Vector2<f64>>,
    ) -> Result<(), anyhow::Error> {
        let app = self.application().unwrap().downcast::<RnoteApp>().unwrap();

        let pos = target_pos.unwrap_or_else(|| {
            self.canvas()
                .transform_canvas_coords_to_sheet_coords(na::vector![
                    f64::from(self.canvas().sheet_margin() + VectorImage::OFFSET_X_DEFAULT),
                    f64::from(self.canvas().sheet_margin() + VectorImage::OFFSET_Y_DEFAULT)
                ])
        });
        let all_strokes = self
            .canvas()
            .sheet()
            .borrow()
            .strokes_state
            .keys_sorted_chrono();
        self.canvas()
            .sheet()
            .borrow_mut()
            .strokes_state
            .set_selected_keys(&all_strokes, false);

        self.canvas()
            .sheet()
            .borrow_mut()
            .strokes_state
            .insert_vectorimage_bytes_threaded(pos, bytes, self.canvas().renderer());

        app.set_input_file(None);
        self.canvas().set_unsaved_changes(true);
        self.canvas().set_empty(false);
        self.canvas().queue_draw();

        self.canvas()
            .selection_modifier()
            .update_state(&self.canvas());

        Ok(())
    }

    /// Target position is in the coordinate space of the sheet
    pub fn load_in_bitmapimage_bytes(
        &self,
        bytes: glib::Bytes,
        target_pos: Option<na::Vector2<f64>>,
    ) -> Result<(), anyhow::Error> {
        let app = self.application().unwrap().downcast::<RnoteApp>().unwrap();

        let pos = target_pos.unwrap_or_else(|| {
            self.canvas()
                .transform_canvas_coords_to_sheet_coords(na::vector![
                    f64::from(self.canvas().sheet_margin() + BitmapImage::OFFSET_X_DEFAULT),
                    f64::from(self.canvas().sheet_margin() + BitmapImage::OFFSET_Y_DEFAULT)
                ])
        });
        let all_strokes = self
            .canvas()
            .sheet()
            .borrow()
            .strokes_state
            .keys_sorted_chrono();
        self.canvas()
            .sheet()
            .borrow_mut()
            .strokes_state
            .set_selected_keys(&all_strokes, false);

        self.canvas()
            .sheet()
            .borrow_mut()
            .strokes_state
            .insert_bitmapimage_bytes_threaded(pos, bytes);

        app.set_input_file(None);

        self.canvas().set_unsaved_changes(true);
        self.canvas().set_empty(false);

        Ok(())
    }

    /// Target position is in the coordinate space of the sheet
    pub fn load_in_pdf_bytes(
        &self,
        bytes: glib::Bytes,
        target_pos: Option<na::Vector2<f64>>,
    ) -> Result<(), anyhow::Error> {
        let app = self.application().unwrap().downcast::<RnoteApp>().unwrap();

        let pos = target_pos.unwrap_or_else(|| {
            self.canvas()
                .transform_canvas_coords_to_sheet_coords(na::vector![
                    f64::from(self.canvas().sheet_margin() + BitmapImage::OFFSET_X_DEFAULT),
                    f64::from(self.canvas().sheet_margin() + BitmapImage::OFFSET_Y_DEFAULT)
                ])
        });
        let page_width = (f64::from(self.canvas().sheet().borrow().width)
            * (self.canvas().pdf_import_width() / 100.0))
            .round() as i32;

        let all_strokes = self
            .canvas()
            .sheet()
            .borrow()
            .strokes_state
            .keys_sorted_chrono();
        self.canvas()
            .sheet()
            .borrow_mut()
            .strokes_state
            .set_selected_keys(&all_strokes, false);

        if self.canvas().pdf_import_as_vector() {
            self.canvas()
                .sheet()
                .borrow_mut()
                .strokes_state
                .insert_pdf_bytes_as_vector_threaded(
                    pos,
                    Some(page_width),
                    bytes,
                    self.canvas().renderer(),
                );
        } else {
            self.canvas()
                .sheet()
                .borrow_mut()
                .strokes_state
                .insert_pdf_bytes_as_bitmap_threaded(pos, Some(page_width), bytes);
        }

        app.set_input_file(None);

        self.canvas().set_unsaved_changes(true);
        self.canvas().set_empty(false);

        Ok(())
    }
}
