use crate::RnoteAppWindow;
use gettextrs::gettext;
use gtk4::pango;
use gtk4::{
    gdk, gio, glib, glib::clone, prelude::*, subclass::prelude::*, Align, Button,
    CompositeTemplate, DragSource, Entry, GestureClick, GestureLongPress, Grid, Image, Label,
    MenuButton, Popover, PopoverMenu, PositionType, Widget,
};
use once_cell::sync::Lazy;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/filerow.ui")]
    pub struct FileRow {
        pub current_file: RefCell<Option<gio::File>>,
        pub drag_source: DragSource,
        pub action_group: gio::SimpleActionGroup,

        #[template_child]
        pub file_image: TemplateChild<Image>,
        #[template_child]
        pub file_label: TemplateChild<Label>,
        #[template_child]
        pub menubutton_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub menubutton: TemplateChild<MenuButton>,
        #[template_child]
        pub popovermenu: TemplateChild<PopoverMenu>,
    }

    impl Default for FileRow {
        fn default() -> Self {
            let drag_source = DragSource::builder()
                .name("workspacebrowser-file-drag-source")
                .actions(gdk::DragAction::COPY)
                .build();

            Self {
                action_group: gio::SimpleActionGroup::new(),
                current_file: RefCell::new(None),
                drag_source,
                file_image: TemplateChild::<Image>::default(),
                file_label: TemplateChild::<Label>::default(),
                menubutton_box: TemplateChild::<gtk4::Box>::default(),
                menubutton: TemplateChild::<MenuButton>::default(),
                popovermenu: TemplateChild::<PopoverMenu>::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileRow {
        const NAME: &'static str = "FileRow";
        type Type = super::FileRow;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FileRow {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.set_widget_name("filerow");

            Self::setup_input(obj);
        }

        fn dispose(&self, obj: &Self::Type) {
            while let Some(child) = obj.first_child() {
                child.unparent();
            }
        }
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "current-file",
                    "current-file",
                    "current-file",
                    Option::<gio::File>::static_type(),
                    glib::ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "current-file" => {
                    let current_file = value
                        .get::<Option<gio::File>>()
                        .expect("The value needs to be of type `Option<gio::File>`.");
                    self.current_file.replace(current_file);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "current-file" => self.current_file.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for FileRow {}

    impl FileRow {
        fn setup_input(obj: &super::FileRow) {
            obj.add_controller(&obj.imp().drag_source);

            let rightclick_gesture = GestureClick::builder()
                .name("rightclick_gesture")
                .button(gdk::BUTTON_SECONDARY)
                .build();
            obj.add_controller(&rightclick_gesture);
            rightclick_gesture.connect_pressed(
                clone!(@weak obj => move |_rightclick_gesture, _n_press, _x, _y| {
                    obj.imp().popovermenu.popup();
                }),
            );

            let longpress_gesture = GestureLongPress::builder()
                .name("longpress_gesture")
                .touch_only(true)
                .build();
            obj.add_controller(&longpress_gesture);
            longpress_gesture.group_with(&rightclick_gesture);

            longpress_gesture.connect_pressed(
                clone!(@weak obj => move |_rightclick_gesture, _x, _y| {
                    obj.imp().popovermenu.popup();
                }),
            );
        }
    }
}

glib::wrapper! {
    pub struct FileRow(ObjectSubclass<imp::FileRow>)
        @extends gtk4::Widget;
}

impl Default for FileRow {
    fn default() -> Self {
        Self::new()
    }
}

impl FileRow {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create `FileRow`")
    }

    pub fn current_file(&self) -> Option<gio::File> {
        self.property::<Option<gio::File>>("current-file")
    }

    pub fn set_current_file(&self, current_file: Option<gio::File>) {
        self.set_property("current-file", current_file.to_value());
    }

    pub fn action_group(&self) -> gio::SimpleActionGroup {
        self.imp().action_group.clone()
    }

    pub fn file_image(&self) -> Image {
        self.imp().file_image.clone()
    }

    pub fn file_label(&self) -> Label {
        self.imp().file_label.clone()
    }

    pub fn drag_source(&self) -> DragSource {
        self.imp().drag_source.clone()
    }

    pub fn menubutton_box(&self) -> gtk4::Box {
        self.imp().menubutton_box.get()
    }

    pub fn init(&self, appwindow: &RnoteAppWindow) {
        self.setup_actions(appwindow);
    }

    fn setup_actions(&self, appwindow: &RnoteAppWindow) {
        // Actions
        self.insert_action_group("filerow", Some(&self.imp().action_group));

        let action_open_file = gio::SimpleAction::new("open-file", None);
        self.imp().action_group.add_action(&action_open_file);
        let action_rename_file = gio::SimpleAction::new("rename-file", None);
        self.imp().action_group.add_action(&action_rename_file);
        let action_trash_file = gio::SimpleAction::new("trash-file", None);
        self.imp().action_group.add_action(&action_trash_file);

        // Open file
        action_open_file.connect_activate(
            clone!(@weak self as filerow, @weak appwindow => move |_action_open_file, _| {
                    if let Some(current_file) = filerow.current_file() {
                                if let Err(e) = appwindow.load_in_file(&current_file, None) {
                                    log::error!("failed to load_in_file() from filerow, {}", e);
                                }
                    }
            }),
        );

        // Rename file
        action_rename_file.connect_activate(clone!(@weak self as filerow => move |_action_rename_file, _| {
                if let Some(current_file) = filerow.current_file() {
                    if let Some(current_path) = current_file.path() {
                        if let Some(parent_path) = current_path.parent().map(|parent_path| parent_path.to_path_buf()) {
                            let current_name = current_path.file_name().map(|current_file_name| current_file_name.to_string_lossy().to_string()).unwrap_or_else(|| String::from(""));

                            let rename_entry = Entry::builder()
                                .text(current_name.as_str())
                                .build();

                            let rename_cancel_button = Button::builder().halign(Align::Start).label(&gettext("Cancel")).build();

                            let rename_apply_button = Button::builder().halign(Align::End).label(&gettext("Apply")).build();
                            rename_apply_button.style_context().add_class("suggested-action");

                            let rename_label = Label::builder().margin_bottom(12).halign(Align::Center).label(&gettext("Rename")).width_chars(24).ellipsize(pango::EllipsizeMode::End).build();
                            rename_label.style_context().add_class("title-4");

                            let rename_grid = Grid::builder().margin_top(6).margin_bottom(6).column_spacing(18).row_spacing(6).build();
                            rename_grid.attach(&rename_label, 0, 0, 2, 1);
                            rename_grid.attach(&rename_entry, 0, 1, 2, 1);
                            rename_grid.attach(&rename_cancel_button, 0, 2, 1, 1);
                            rename_grid.attach(&rename_apply_button, 1, 2, 1, 1);

                            let rename_popover = Popover::builder().autohide(true).has_arrow(true).position(PositionType::Bottom).build();
                            rename_popover.set_child(Some(&rename_grid));
                            filerow.menubutton_box().append(&rename_popover);

                            let parent_path_1 = parent_path.clone();
                            rename_entry.connect_text_notify(clone!(@weak rename_apply_button => move |rename_entry| {
                                let new_file_path = parent_path_1.join(rename_entry.text().to_string());
                                let new_file = gio::File::for_path(new_file_path);

                                // Disable apply button to prevent overwrites when file already exists
                                rename_apply_button.set_sensitive(!new_file.query_exists(None::<&gio::Cancellable>));
                            }));

                            rename_cancel_button.connect_clicked(clone!(@weak rename_popover => move |_| {
                                rename_popover.popdown();
                            }));

                            rename_apply_button.connect_clicked(clone!(@weak rename_popover, @weak rename_entry => move |_| {
                                let new_file_path = parent_path.join(rename_entry.text().to_string());
                                let new_file = gio::File::for_path(new_file_path);

                                if new_file.query_exists(None::<&gio::Cancellable>) {
                                    // Should have been caught earlier, but making sure
                                    log::error!("file already exists");
                                } else {
                                    if let Err(e) = current_file.move_(&new_file, gio::FileCopyFlags::NONE, None::<&gio::Cancellable>, None) {
                                        log::error!("rename file failed with Err {}", e);
                                    }

                                    rename_popover.popdown();
                                }
                            }));

                            rename_popover.popup();
                        }
                    }
                }
            }));

        // Trash file
        action_trash_file.connect_activate(clone!(@weak self as filerow => move |_action_trash_file, _| {
                if let Some(current_file) = filerow.current_file() {
                    current_file.trash_async(glib::PRIORITY_DEFAULT, None::<&gio::Cancellable>, clone!(@weak filerow => move |res| {
                        if let Err(e) = res {
                            log::error!("filerow trash file failed with Err {}", e);
                        } else {
                            filerow.set_current_file(None);
                        }
                    }));
                }
            }));
    }
}
