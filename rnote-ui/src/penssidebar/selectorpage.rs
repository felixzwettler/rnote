use gtk4::{glib, glib::clone, prelude::*, subclass::prelude::*, CompositeTemplate, ToggleButton};
use rnote_engine::pens::selector::SelectorStyle;

use crate::appwindow::RnoteAppWindow;

mod imp {
    use super::*;

    #[derive(Default, Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/penssidebar/selectorpage.ui")]
    pub struct SelectorPage {
        #[template_child]
        pub selectorstyle_polygon_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        pub selectorstyle_rect_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        pub selectorstyle_single_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        pub selectorstyle_intersectingpath_toggle: TemplateChild<ToggleButton>,
        #[template_child]
        pub resize_lock_aspectratio_togglebutton: TemplateChild<ToggleButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SelectorPage {
        const NAME: &'static str = "SelectorPage";
        type Type = super::SelectorPage;
        type ParentType = gtk4::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SelectorPage {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn dispose(&self) {
            while let Some(child) = self.instance().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for SelectorPage {}
}

glib::wrapper! {
    pub struct SelectorPage(ObjectSubclass<imp::SelectorPage>)
        @extends gtk4::Widget;
}

impl Default for SelectorPage {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectorPage {
    pub fn new() -> Self {
        glib::Object::new(&[])
    }

    pub fn selectorstyle_polygon_toggle(&self) -> ToggleButton {
        self.imp().selectorstyle_polygon_toggle.get()
    }

    pub fn selectorstyle_rect_toggle(&self) -> ToggleButton {
        self.imp().selectorstyle_rect_toggle.get()
    }

    pub fn selectorstyle_single_toggle(&self) -> ToggleButton {
        self.imp().selectorstyle_single_toggle.get()
    }

    pub fn selectorstyle_intersectingpath_toggle(&self) -> ToggleButton {
        self.imp().selectorstyle_intersectingpath_toggle.get()
    }

    pub fn resize_lock_aspectratio_togglebutton(&self) -> ToggleButton {
        self.imp().resize_lock_aspectratio_togglebutton.get()
    }

    pub fn init(&self, appwindow: &RnoteAppWindow) {
        // selecting with Polygon / Rect toggles
        self.selectorstyle_polygon_toggle().connect_toggled(clone!(@weak appwindow => move |selectorstyle_polygon_toggle| {
            if selectorstyle_polygon_toggle.is_active() {
                appwindow.canvas().engine().borrow_mut().penholder.selector.style = SelectorStyle::Polygon;

                if let Err(e) = appwindow.save_engine_config() {
                    log::error!("saving engine config failed after changing selector style, Err `{}`", e);
                }
            }
        }));

        self.selectorstyle_rect_toggle().connect_toggled(clone!(@weak appwindow => move |selectorstyle_rect_toggle| {
            if selectorstyle_rect_toggle.is_active() {
                appwindow.canvas().engine().borrow_mut().penholder.selector.style = SelectorStyle::Rectangle;

                if let Err(e) = appwindow.save_engine_config() {
                    log::error!("saving engine config failed after changing selector style, Err `{}`", e);
                }
            }
        }));

        self.selectorstyle_single_toggle().connect_toggled(clone!(@weak appwindow => move |selectorstyle_single_toggle| {
            if selectorstyle_single_toggle.is_active() {
                appwindow.canvas().engine().borrow_mut().penholder.selector.style = SelectorStyle::Single;

                if let Err(e) = appwindow.save_engine_config() {
                    log::error!("saving engine config failed after changing selector style, Err `{}`", e);
                }
            }
        }));

        self.selectorstyle_intersectingpath_toggle().connect_toggled(clone!(@weak appwindow => move |selectorstyle_intersectingpath_toggle| {
            if selectorstyle_intersectingpath_toggle.is_active() {
                appwindow.canvas().engine().borrow_mut().penholder.selector.style = SelectorStyle::IntersectingPath;

                if let Err(e) = appwindow.save_engine_config() {
                    log::error!("saving engine config failed after changing selector style, Err `{}`", e);
                }
            }
        }));

        self.resize_lock_aspectratio_togglebutton().connect_toggled(clone!(@weak appwindow = > move |resize_lock_aspectratio_togglebutton| {
            appwindow.canvas().engine().borrow_mut().penholder.selector.resize_lock_aspectratio = resize_lock_aspectratio_togglebutton.is_active();

            if let Err(e) = appwindow.save_engine_config() {
                log::error!("saving engine config failed after changing selector lock aspectratio, Err `{}`", e);
            }
        }));
    }

    pub fn refresh_ui(&self, appwindow: &RnoteAppWindow) {
        let selector = appwindow
            .canvas()
            .engine()
            .borrow()
            .penholder
            .selector
            .clone();

        match selector.style {
            SelectorStyle::Polygon => self.selectorstyle_polygon_toggle().set_active(true),
            SelectorStyle::Rectangle => self.selectorstyle_rect_toggle().set_active(true),
            SelectorStyle::Single => self.selectorstyle_single_toggle().set_active(true),
            SelectorStyle::IntersectingPath => self
                .selectorstyle_intersectingpath_toggle()
                .set_active(true),
        }
        self.resize_lock_aspectratio_togglebutton()
            .set_active(selector.resize_lock_aspectratio);
    }
}
