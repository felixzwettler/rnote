pub mod appactions;

mod imp {
    use std::{
        cell::{Cell, RefCell},
        path,
    };

    use adw::subclass::prelude::AdwApplicationImpl;
    use gtk4::{gio, glib, prelude::*, subclass::prelude::*};
    use once_cell::sync::Lazy;

    use crate::ui::canvas::ExpandMode;
    use crate::{
        compose::textured::TexturedDotsDistribution,
        config,
        pens::PenStyle,
        sheet::format::MeasureUnit,
        sheet::{background::PatternStyle, format::PredefinedFormat},
        ui::{
            appmenu::AppMenu, appwindow::RnoteAppWindow, canvas::Canvas, canvasmenu::CanvasMenu,
            colorpicker::colorsetter::ColorSetter, colorpicker::ColorPicker,
            mainheader::MainHeader, penssidebar::brushpage::BrushPage,
            penssidebar::eraserpage::EraserPage, penssidebar::markerpage::MarkerPage,
            penssidebar::selectorpage::SelectorPage, penssidebar::shaperpage::ShaperPage,
            penssidebar::toolspage::ToolsPage, penssidebar::PensSideBar,
            selectionmodifier::modifiernode::ModifierNode, selectionmodifier::SelectionModifier,
            settingspanel::SettingsPanel, unitentry::UnitEntry, workspacebrowser::filerow::FileRow,
            workspacebrowser::WorkspaceBrowser,
        },
        utils,
    };

    #[derive(Debug)]
    pub struct RnoteApp {
        pub input_file: RefCell<Option<gio::File>>,
        pub output_file: RefCell<Option<gio::File>>,
        pub unsaved_changes: Cell<bool>,
    }

    impl Default for RnoteApp {
        fn default() -> Self {
            Self {
                input_file: RefCell::new(None),
                output_file: RefCell::new(None),
                unsaved_changes: Cell::new(false),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RnoteApp {
        const NAME: &'static str = "RnoteApp";
        type Type = super::RnoteApp;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for RnoteApp {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                // Any unsaved changes of the current application state
                vec![glib::ParamSpecBoolean::new(
                    "unsaved-changes",
                    "unsaved-changes",
                    "unsaved-changes",
                    false,
                    glib::ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "unsaved-changes" => self.unsaved_changes.get().to_value(),
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
                "unsaved-changes" => {
                    let unsaved_changes: bool =
                        value.get().expect("The value needs to be of type `bool`.");
                    self.unsaved_changes.replace(unsaved_changes);
                }
                _ => unimplemented!(),
            }
        }
    }

    impl ApplicationImpl for RnoteApp {
        fn activate(&self, app: &Self::Type) {
            // Custom buildable Widgets need to register
            RnoteAppWindow::static_type();
            Canvas::static_type();
            ExpandMode::static_type();
            ColorPicker::static_type();
            ColorSetter::static_type();
            SelectionModifier::static_type();
            ModifierNode::static_type();
            CanvasMenu::static_type();
            SettingsPanel::static_type();
            AppMenu::static_type();
            MainHeader::static_type();
            PensSideBar::static_type();
            MarkerPage::static_type();
            BrushPage::static_type();
            ShaperPage::static_type();
            EraserPage::static_type();
            SelectorPage::static_type();
            ToolsPage::static_type();
            PenStyle::static_type();
            WorkspaceBrowser::static_type();
            FileRow::static_type();
            PredefinedFormat::static_type();
            MeasureUnit::static_type();
            PatternStyle::static_type();
            UnitEntry::static_type();
            TexturedDotsDistribution::static_type();

            // Load the resources
            app.set_resource_base_path(Some(config::APP_IDPATH));
            let res = gio::Resource::load(path::Path::new(config::RESOURCES_FILE))
                .expect("Could not load gresource file");
            gio::resources_register(&res);

            // init gstreamer
            if let Err(e) = gst::init() {
                log::error!("failed to initialize gstreamer. Err `{}`. Aborting.", e);
                return;
            }

            let appwindow = RnoteAppWindow::new(app.upcast_ref::<gtk4::Application>());
            appwindow.init();

            // setup the app
            app.setup_actions();
            app.setup_action_accels();

            // Everything else before showing
            app.init_misc(&appwindow);

            appwindow.show();
        }

        fn open(&self, application: &Self::Type, files: &[gio::File], _hint: &str) {
            for file in files {
                match utils::FileType::lookup_file_type(file) {
                    utils::FileType::UnknownFile => {
                        log::warn!("tried to open unsupported file type");
                    }
                    _ => {
                        *self.input_file.borrow_mut() = Some(file.clone());
                    }
                };
            }
            application.activate();
        }
    }

    impl GtkApplicationImpl for RnoteApp {}
    impl AdwApplicationImpl for RnoteApp {}
}

use gtk4::{gio, glib, prelude::*, subclass::prelude::*};

use crate::config;
use crate::ui::appwindow::RnoteAppWindow;

glib::wrapper! {
    pub struct RnoteApp(ObjectSubclass<imp::RnoteApp>)
        @extends gio::Application, gtk4::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for RnoteApp {
    fn default() -> Self {
        Self::new()
    }
}

impl RnoteApp {
    pub fn new() -> Self {
        glib::Object::new(&[
            ("application-id", &config::APP_ID),
            ("flags", &gio::ApplicationFlags::HANDLES_OPEN),
        ])
        .expect("failed to create RnoteApp")
    }

    pub fn input_file(&self) -> Option<gio::File> {
        imp::RnoteApp::from_instance(self)
            .input_file
            .borrow()
            .clone()
    }

    pub fn set_input_file(&self, input_file: Option<gio::File>) {
        *imp::RnoteApp::from_instance(self).input_file.borrow_mut() = input_file;
    }

    pub fn output_file(&self) -> Option<gio::File> {
        imp::RnoteApp::from_instance(self)
            .output_file
            .borrow()
            .clone()
    }

    pub fn set_output_file(&self, output_file: Option<&gio::File>, appwindow: &RnoteAppWindow) {
        appwindow.mainheader().set_title_for_file(output_file);
        *imp::RnoteApp::from_instance(self).output_file.borrow_mut() = output_file.cloned();
    }

    pub fn unsaved_changes(&self) -> bool {
        self.property::<bool>("unsaved-changes")
    }

    pub fn set_unsaved_changes(&self, unsaved_changes: bool) {
        self.set_property("unsaved-changes", unsaved_changes.to_value());
    }

    // Anything that needs to be done right before showing the appwindow
    pub fn init_misc(&self, appwindow: &RnoteAppWindow) {
        appwindow.canvas().resize_sheet_to_fit_strokes();
        appwindow.canvas().return_to_origin_page();
    }
}
