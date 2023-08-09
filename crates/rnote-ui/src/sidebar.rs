// Imports
use crate::{RnAppWindow, RnSettingsPanel, RnWorkspaceBrowser};
use gtk4::{
    glib, glib::clone, prelude::*, subclass::prelude::*, Button, CompositeTemplate, Widget,
};

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate, Default)]
    #[template(resource = "/com/github/flxzt/rnote/ui/sidebar.ui")]
    pub(crate) struct RnSidebar {
        #[template_child]
        pub(crate) headerbar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub(crate) close_button: TemplateChild<Button>,
        #[template_child]
        pub(crate) sidebar_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub(crate) workspacebrowser: TemplateChild<RnWorkspaceBrowser>,
        #[template_child]
        pub(crate) settings_panel: TemplateChild<RnSettingsPanel>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RnSidebar {
        const NAME: &'static str = "RnSidebar";
        type Type = super::RnSidebar;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RnSidebar {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn dispose(&self) {
            self.dispose_template();
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for RnSidebar {}
}

glib::wrapper! {
    pub(crate) struct RnSidebar(ObjectSubclass<imp::RnSidebar>)
        @extends gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl Default for RnSidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl RnSidebar {
    pub(crate) fn new() -> Self {
        glib::Object::new()
    }

    pub(crate) fn headerbar(&self) -> adw::HeaderBar {
        self.imp().headerbar.get()
    }

    pub(crate) fn close_button(&self) -> Button {
        self.imp().close_button.get()
    }

    pub(crate) fn sidebar_stack(&self) -> adw::ViewStack {
        self.imp().sidebar_stack.get()
    }

    pub(crate) fn workspacebrowser(&self) -> RnWorkspaceBrowser {
        self.imp().workspacebrowser.get()
    }

    pub(crate) fn settings_panel(&self) -> RnSettingsPanel {
        self.imp().settings_panel.get()
    }

    pub(crate) fn init(&self, appwindow: &RnAppWindow) {
        let imp = self.imp();

        imp.workspacebrowser.get().init(appwindow);
        imp.settings_panel.get().init(appwindow);

        appwindow
            .split_view()
            .bind_property("collapsed", &imp.close_button.get(), "visible")
            .sync_create()
            .build();

        imp.close_button
            .connect_clicked(clone!(@weak appwindow => move |_| {
                appwindow.split_view().set_show_sidebar(false);
            }));
    }
}
