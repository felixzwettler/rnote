use crate::RnoteAppWindow;
use gtk4::{
    gdk, glib, glib::clone, prelude::*, subclass::prelude::*, CompositeTemplate, CssProvider,
    GestureClick, GestureLongPress, Image, Widget,
};
use once_cell::sync::Lazy;
use std::cell::RefCell;

use super::WorkspaceListEntry;

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/workspacerow.ui")]
    pub struct WorkspaceRow {
        pub entry: RefCell<WorkspaceListEntry>,

        #[template_child]
        pub folder_image: TemplateChild<Image>,

        pub css: CssProvider,
    }

    impl Default for WorkspaceRow {
        fn default() -> Self {
            Self {
                entry: RefCell::new(WorkspaceListEntry::default()),
                folder_image: TemplateChild::<Image>::default(),

                css: CssProvider::new(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WorkspaceRow {
        const NAME: &'static str = "WorkspaceRow";
        type Type = super::WorkspaceRow;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WorkspaceRow {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.set_css_classes(&["workspacerow"]);

            obj.style_context()
                .add_provider(&self.css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }

        fn dispose(&self, obj: &Self::Type) {
            while let Some(child) = obj.first_child() {
                child.unparent();
            }
        }
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "entry",
                    "entry",
                    "entry",
                    WorkspaceListEntry::static_type(),
                    glib::ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "entry" => {
                    let entry = value
                        .get::<WorkspaceListEntry>()
                        .expect("The value needs to be of type `WorkspaceListEntry`.");

                    entry.connect_notify_local(
                        Some("dir"),
                        clone!(@strong obj => move |_, _| {
                            obj.imp().update_apearance();
                        }),
                    );

                    entry.connect_notify_local(
                        Some("color"),
                        clone!(@strong obj => move |_, _| {
                            obj.imp().update_apearance();
                        }),
                    );

                    entry.connect_notify_local(
                        Some("name"),
                        clone!(@strong obj => move |_, _| {
                            obj.imp().update_apearance();
                        }),
                    );

                    self.entry.replace(entry);
                    self.update_apearance();
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "entry" => self.entry.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for WorkspaceRow {}

    impl WorkspaceRow {
        fn update_apearance(&self) {
            let dir = self.entry.borrow().dir();
            let color = self.entry.borrow().color();
            let name = self.entry.borrow().name();

            self.instance()
                .set_tooltip_text(Some(format!("{}\n{}", name, dir).as_str()));

            let custom_css = format!(
                "
.workspacerow {{
    color: rgba({0}, {1}, {2}, {3:.3});
    transition: color 0.15s ease-out;
}}
            ",
                (color.red() * 255.0) as i32,
                (color.green() * 255.0) as i32,
                (color.blue() * 255.0) as i32,
                (color.alpha() * 1000.0).round() / 1000.0
            );

            self.css.load_from_data(custom_css.as_bytes());
        }
    }
}

glib::wrapper! {
    pub struct WorkspaceRow(ObjectSubclass<imp::WorkspaceRow>)
        @extends gtk4::Widget;
}

impl Default for WorkspaceRow {
    fn default() -> Self {
        Self::new(WorkspaceListEntry::default())
    }
}

impl WorkspaceRow {
    pub fn new(entry: WorkspaceListEntry) -> Self {
        glib::Object::new(&[("entry", &entry.to_value())]).expect("Failed to create `WorkspaceRow`")
    }

    pub fn entry(&self) -> WorkspaceListEntry {
        self.property::<WorkspaceListEntry>("entry")
    }

    pub fn set_entry(&self, entry: WorkspaceListEntry) {
        self.set_property("entry", entry.to_value());
    }

    pub fn folder_image(&self) -> Image {
        self.imp().folder_image.clone()
    }

    pub fn init(&self, appwindow: &RnoteAppWindow) {
        let rightclick_gesture = GestureClick::builder()
            .name("rightclick_gesture")
            .button(gdk::BUTTON_SECONDARY)
            .build();
        self.add_controller(&rightclick_gesture);
        rightclick_gesture.connect_pressed(
            clone!(@weak appwindow => move |_rightclick_gesture, _n_press, _x, _y| {
                adw::prelude::ActionGroupExt::activate_action(&appwindow, "edit-workspace", None);
            }),
        );

        let longpress_gesture = GestureLongPress::builder()
            .name("longpress_gesture")
            .touch_only(true)
            .build();
        self.add_controller(&longpress_gesture);
        longpress_gesture.group_with(&rightclick_gesture);

        longpress_gesture.connect_pressed(
            clone!(@weak appwindow => move |_rightclick_gesture, _x, _y| {
                adw::prelude::ActionGroupExt::activate_action(&appwindow, "edit-workspace", None);
            }),
        );
    }
}
