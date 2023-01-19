mod colorpad;
mod colorsetter;

// Re-exports
pub(crate) use colorpad::ColorPad;
pub(crate) use colorsetter::ColorSetter;

// Imports
use std::cell::{Cell, RefCell};

use gtk4::{
    gdk, glib, glib::clone, glib::translate::IntoGlib, prelude::*, subclass::prelude::*, BoxLayout,
    Button, ColorChooserWidget, CompositeTemplate, MenuButton, Orientation, Popover, PositionType,
    Widget,
};

use once_cell::sync::Lazy;
use rnote_compose::{color, Color};
use rnote_engine::utils::GdkRGBAHelpers;

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/colorpicker.ui")]
    pub(crate) struct ColorPicker {
        pub(crate) stroke_color: RefCell<gdk::RGBA>,
        pub(crate) fill_color: RefCell<gdk::RGBA>,
        pub(crate) position: Cell<PositionType>,

        #[template_child]
        pub(crate) active_colors_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub(crate) stroke_color_pad: TemplateChild<ColorPad>,
        #[template_child]
        pub(crate) fill_color_pad: TemplateChild<ColorPad>,
        #[template_child]
        pub(crate) setter_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub(crate) setter_1: TemplateChild<ColorSetter>,
        #[template_child]
        pub(crate) setter_2: TemplateChild<ColorSetter>,
        #[template_child]
        pub(crate) setter_3: TemplateChild<ColorSetter>,
        #[template_child]
        pub(crate) setter_4: TemplateChild<ColorSetter>,
        #[template_child]
        pub(crate) setter_5: TemplateChild<ColorSetter>,
        #[template_child]
        pub(crate) setter_6: TemplateChild<ColorSetter>,
        #[template_child]
        pub(crate) setter_7: TemplateChild<ColorSetter>,
        #[template_child]
        pub(crate) setter_8: TemplateChild<ColorSetter>,
        #[template_child]
        pub(crate) colorpicker_button: TemplateChild<MenuButton>,
        #[template_child]
        pub(crate) colorpicker_popover: TemplateChild<Popover>,
        #[template_child]
        pub(crate) colorchooser: TemplateChild<ColorChooserWidget>,
        #[template_child]
        pub(crate) colorchooser_editor_gobackbutton: TemplateChild<Button>,
        #[template_child]
        pub(crate) colorchooser_editor_selectbutton: TemplateChild<Button>,
    }

    impl Default for ColorPicker {
        fn default() -> Self {
            Self {
                stroke_color: RefCell::new(gdk::RGBA::from_compose_color(
                    *super::STROKE_COLOR_DEFAULT,
                )),
                fill_color: RefCell::new(gdk::RGBA::from_compose_color(*super::FILL_COLOR_DEFAULT)),
                position: Cell::new(PositionType::Right),

                active_colors_box: TemplateChild::default(),
                stroke_color_pad: TemplateChild::default(),
                fill_color_pad: TemplateChild::default(),
                setter_box: TemplateChild::default(),
                setter_1: TemplateChild::default(),
                setter_2: TemplateChild::default(),
                setter_3: TemplateChild::default(),
                setter_4: TemplateChild::default(),
                setter_5: TemplateChild::default(),
                setter_6: TemplateChild::default(),
                setter_7: TemplateChild::default(),
                setter_8: TemplateChild::default(),
                colorpicker_button: TemplateChild::default(),
                colorpicker_popover: TemplateChild::default(),
                colorchooser: TemplateChild::default(),
                colorchooser_editor_gobackbutton: TemplateChild::default(),
                colorchooser_editor_selectbutton: TemplateChild::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ColorPicker {
        const NAME: &'static str = "ColorPicker";
        type Type = super::ColorPicker;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ColorPicker {
        fn constructed(&self) {
            self.parent_constructed();
            let inst = self.instance();

            let colorchooser = self.colorchooser.get();
            let colorpicker_popover = self.colorpicker_popover.get();
            let colorchooser_editor_gobackbutton = self.colorchooser_editor_gobackbutton.get();

            self.setup_setters();

            inst.connect_notify_local(Some("stroke-color"), move |colorpicker, _pspec| {
                if colorpicker.imp().stroke_color_pad.is_active() {
                    colorpicker
                        .imp()
                        .colorchooser
                        .set_rgba(&colorpicker.stroke_color());
                }
            });

            inst.connect_notify_local(Some("fill-color"), move |colorpicker, _pspec| {
                if colorpicker.imp().fill_color_pad.is_active() {
                    colorpicker
                        .imp()
                        .colorchooser
                        .set_rgba(&colorpicker.fill_color());
                }
            });

            self.colorchooser.connect_show_editor_notify(
                clone!(@weak colorchooser_editor_gobackbutton => move |_colorchooser| {
                    colorchooser_editor_gobackbutton.set_visible(true);
                }),
            );

            self.colorchooser_editor_selectbutton.connect_clicked(
                clone!(@weak inst as colorpicker, @weak colorchooser, @weak colorpicker_popover => move |_colorchooser_editor_selectbutton| {
                    let color = colorchooser.rgba();
                    colorpicker.set_color_active_setter(color);

                    colorpicker_popover.popdown();
                }),
            );

            self.colorchooser_editor_gobackbutton.connect_clicked(
                clone!(@weak colorchooser => move |colorchooser_editor_gobackbutton| {
                    colorchooser.set_show_editor(false);
                    colorchooser_editor_gobackbutton.set_visible(false);
                }),
            );

            self.colorchooser.connect_rgba_notify(
                clone!(@weak inst as colorpicker => move |colorchooser| {
                    let color = colorchooser.rgba();

                    colorpicker.set_color_active_pad(color);
                    colorpicker.set_color_active_setter(color);
                }),
            );

            self.stroke_color_pad
                .bind_property("color", &*inst, "stroke-color")
                .sync_create()
                .bidirectional()
                .build();

            self.stroke_color_pad.connect_active_notify(
                clone!(@weak inst as colorpicker => move |_| {
                    colorpicker.deselect_setters();
                }),
            );

            self.fill_color_pad
                .bind_property("color", &*inst, "fill-color")
                .sync_create()
                .bidirectional()
                .build();

            self.fill_color_pad.connect_active_notify(
                clone!(@weak inst as colorpicker => move |_| {
                    colorpicker.deselect_setters();
                }),
            );
        }

        fn dispose(&self) {
            while let Some(child) = self.instance().first_child() {
                child.unparent();
            }
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecEnum::new(
                        "position",
                        "position",
                        "position",
                        PositionType::static_type(),
                        PositionType::Right.into_glib(),
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpecBoxed::new(
                        "stroke-color",
                        "stroke-color",
                        "stroke-color",
                        gdk::RGBA::static_type(),
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpecBoxed::new(
                        "fill-color",
                        "fill-color",
                        "fill-color",
                        gdk::RGBA::static_type(),
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let inst = self.instance();

            match pspec.name() {
                "position" => {
                    let layout_manager = inst
                        .layout_manager()
                        .unwrap()
                        .downcast::<BoxLayout>()
                        .unwrap();

                    let position = value
                        .get::<PositionType>()
                        .expect("value not of type `PositionType`");
                    self.position.replace(position);

                    self.setter_1.set_position(position);
                    self.setter_2.set_position(position);
                    self.setter_3.set_position(position);
                    self.setter_4.set_position(position);
                    self.setter_5.set_position(position);
                    self.setter_6.set_position(position);
                    self.setter_7.set_position(position);
                    self.setter_8.set_position(position);

                    match position {
                        PositionType::Left => {
                            layout_manager.set_orientation(Orientation::Vertical);
                            self.active_colors_box
                                .set_orientation(Orientation::Vertical);
                            self.setter_box.set_orientation(Orientation::Vertical);
                            self.colorpicker_popover.set_position(PositionType::Right);
                        }
                        PositionType::Right => {
                            layout_manager.set_orientation(Orientation::Vertical);
                            self.active_colors_box
                                .set_orientation(Orientation::Vertical);
                            self.setter_box.set_orientation(Orientation::Vertical);
                            self.colorpicker_popover.set_position(PositionType::Left);
                        }
                        PositionType::Top => {
                            layout_manager.set_orientation(Orientation::Horizontal);
                            self.active_colors_box
                                .set_orientation(Orientation::Horizontal);
                            self.setter_box.set_orientation(Orientation::Horizontal);
                            self.colorpicker_popover.set_position(PositionType::Bottom);
                        }
                        PositionType::Bottom => {
                            layout_manager.set_orientation(Orientation::Horizontal);
                            self.active_colors_box
                                .set_orientation(Orientation::Horizontal);
                            self.setter_box.set_orientation(Orientation::Horizontal);
                            self.colorpicker_popover.set_position(PositionType::Top);
                        }
                        _ => {}
                    }
                }
                "stroke-color" => {
                    self.stroke_color.replace(
                        value
                            .get::<gdk::RGBA>()
                            .expect("value not of type `gdk::RGBA`"),
                    );
                }
                "fill-color" => {
                    self.fill_color.replace(
                        value
                            .get::<gdk::RGBA>()
                            .expect("value not of type `gdk::RGBA`"),
                    );
                }
                _ => panic!("invalid property name"),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "position" => self.position.get().to_value(),
                "stroke-color" => self.stroke_color.borrow().to_value(),
                "fill-color" => self.fill_color.borrow().to_value(),
                _ => panic!("invalid property name"),
            }
        }
    }

    impl WidgetImpl for ColorPicker {}

    impl ColorPicker {
        fn setup_setters(&self) {
            let inst = self.instance();

            self.setter_1.set_color(Self::default_color(0, 8));
            self.setter_2.set_color(Self::default_color(1, 8));
            self.setter_3.set_color(Self::default_color(2, 8));
            self.setter_4.set_color(Self::default_color(3, 8));
            self.setter_5.set_color(Self::default_color(4, 8));
            self.setter_6.set_color(Self::default_color(5, 8));
            self.setter_7.set_color(Self::default_color(6, 8));
            self.setter_8.set_color(Self::default_color(7, 8));

            self.setter_1.connect_active_notify(
                clone!(@weak inst as colorpicker => move |setter| {
                    if setter.is_active() {
                        colorpicker.setter_2().set_active(false);
                        colorpicker.setter_3().set_active(false);
                        colorpicker.setter_4().set_active(false);
                        colorpicker.setter_5().set_active(false);
                        colorpicker.setter_6().set_active(false);
                        colorpicker.setter_7().set_active(false);
                        colorpicker.setter_8().set_active(false);
                        // Must come after setting the other setters inactive
                        colorpicker.set_color_active_pad(setter.color());
                    }
                }),
            );

            self.setter_2.connect_active_notify(
                clone!(@weak inst as colorpicker => move |setter| {
                    if setter.is_active() {
                        colorpicker.setter_1().set_active(false);
                        colorpicker.setter_3().set_active(false);
                        colorpicker.setter_4().set_active(false);
                        colorpicker.setter_5().set_active(false);
                        colorpicker.setter_6().set_active(false);
                        colorpicker.setter_7().set_active(false);
                        colorpicker.setter_8().set_active(false);
                        colorpicker.set_color_active_pad(setter.color());
                    }
                }),
            );

            self.setter_3.connect_active_notify(
                clone!(@weak inst as colorpicker => move |setter| {
                    if setter.is_active() {
                        colorpicker.setter_1().set_active(false);
                        colorpicker.setter_2().set_active(false);
                        colorpicker.setter_4().set_active(false);
                        colorpicker.setter_5().set_active(false);
                        colorpicker.setter_6().set_active(false);
                        colorpicker.setter_7().set_active(false);
                        colorpicker.setter_8().set_active(false);
                        colorpicker.set_color_active_pad(setter.color());
                    }
                }),
            );

            self.setter_4.connect_active_notify(
                clone!(@weak inst as colorpicker => move |setter| {
                    if setter.is_active() {
                        colorpicker.setter_1().set_active(false);
                        colorpicker.setter_2().set_active(false);
                        colorpicker.setter_3().set_active(false);
                        colorpicker.setter_5().set_active(false);
                        colorpicker.setter_6().set_active(false);
                        colorpicker.setter_7().set_active(false);
                        colorpicker.setter_8().set_active(false);
                        colorpicker.set_color_active_pad(setter.color());
                    }
                }),
            );

            self.setter_5.connect_active_notify(
                clone!(@weak inst as colorpicker => move |setter| {
                    if setter.is_active() {
                        colorpicker.setter_1().set_active(false);
                        colorpicker.setter_2().set_active(false);
                        colorpicker.setter_3().set_active(false);
                        colorpicker.setter_4().set_active(false);
                        colorpicker.setter_6().set_active(false);
                        colorpicker.setter_7().set_active(false);
                        colorpicker.setter_8().set_active(false);
                        colorpicker.set_color_active_pad(setter.color());
                    }
                }),
            );

            self.setter_6.connect_active_notify(
                clone!(@weak inst as colorpicker => move |setter| {
                    if setter.is_active() {
                        colorpicker.setter_1().set_active(false);
                        colorpicker.setter_2().set_active(false);
                        colorpicker.setter_3().set_active(false);
                        colorpicker.setter_4().set_active(false);
                        colorpicker.setter_5().set_active(false);
                        colorpicker.setter_7().set_active(false);
                        colorpicker.setter_8().set_active(false);
                        colorpicker.set_color_active_pad(setter.color());
                    }
                }),
            );

            self.setter_7.connect_active_notify(
                clone!(@weak inst as colorpicker => move |setter| {
                    if setter.is_active() {
                        colorpicker.setter_1().set_active(false);
                        colorpicker.setter_2().set_active(false);
                        colorpicker.setter_3().set_active(false);
                        colorpicker.setter_4().set_active(false);
                        colorpicker.setter_5().set_active(false);
                        colorpicker.setter_6().set_active(false);
                        colorpicker.setter_8().set_active(false);
                        colorpicker.set_color_active_pad(setter.color());
                    }
                }),
            );

            self.setter_8.connect_active_notify(
                clone!(@weak inst as colorpicker => move |setter| {
                    if setter.is_active() {
                        colorpicker.setter_1().set_active(false);
                        colorpicker.setter_2().set_active(false);
                        colorpicker.setter_3().set_active(false);
                        colorpicker.setter_4().set_active(false);
                        colorpicker.setter_5().set_active(false);
                        colorpicker.setter_6().set_active(false);
                        colorpicker.setter_7().set_active(false);
                        colorpicker.set_color_active_pad(setter.color());
                    }
                }),
            );
        }

        fn default_color(i: usize, amount_setters: usize) -> gdk::RGBA {
            let color_step =
                (2.0 * std::f32::consts::PI) / ((amount_setters.saturating_sub(1)) as f32);
            let rgb_offset = (2.0 / 3.0) * std::f32::consts::PI;
            let color_offset = (5.0 / 4.0) * std::f32::consts::PI + 0.4;

            gdk::RGBA::new(
                0.5 * (i as f32 * color_step + 0.0 * rgb_offset + color_offset).sin() + 0.5,
                0.5 * (i as f32 * color_step + 1.0 * rgb_offset + color_offset).sin() + 0.5,
                0.5 * (i as f32 * color_step + 2.0 * rgb_offset + color_offset).sin() + 0.5,
                1.0,
            )
        }
    }
}

glib::wrapper! {
    pub(crate) struct ColorPicker(ObjectSubclass<imp::ColorPicker>)
        @extends Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl Default for ColorPicker {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) static STROKE_COLOR_DEFAULT: Lazy<Color> =
    Lazy::new(|| Color::from(color::GNOME_DARKS[4]));
pub(crate) static FILL_COLOR_DEFAULT: Lazy<Color> =
    Lazy::new(|| Color::from(color::GNOME_BLUES[1]));

impl ColorPicker {
    pub(crate) fn new() -> Self {
        glib::Object::new(&[])
    }

    #[allow(unused)]
    pub(crate) fn position(&self) -> PositionType {
        self.property::<PositionType>("position")
    }

    #[allow(unused)]
    pub(crate) fn set_position(&self, position: PositionType) {
        self.set_property("position", position.to_value());
    }

    #[allow(unused)]
    pub(crate) fn stroke_color(&self) -> gdk::RGBA {
        self.property::<gdk::RGBA>("stroke-color")
    }

    #[allow(unused)]
    pub(crate) fn set_stroke_color(&self, color: gdk::RGBA) {
        self.set_property("stroke-color", color.to_value());
    }

    #[allow(unused)]
    pub(crate) fn fill_color(&self) -> gdk::RGBA {
        self.property::<gdk::RGBA>("fill-color")
    }

    #[allow(unused)]
    pub(crate) fn set_fill_color(&self, color: gdk::RGBA) {
        self.set_property("fill-color", color.to_value());
    }

    pub(crate) fn setter_1(&self) -> ColorSetter {
        self.imp().setter_1.get()
    }

    pub(crate) fn setter_2(&self) -> ColorSetter {
        self.imp().setter_2.get()
    }

    pub(crate) fn setter_3(&self) -> ColorSetter {
        self.imp().setter_3.get()
    }

    pub(crate) fn setter_4(&self) -> ColorSetter {
        self.imp().setter_4.get()
    }

    pub(crate) fn setter_5(&self) -> ColorSetter {
        self.imp().setter_5.get()
    }

    pub(crate) fn setter_6(&self) -> ColorSetter {
        self.imp().setter_6.get()
    }

    pub(crate) fn setter_7(&self) -> ColorSetter {
        self.imp().setter_7.get()
    }

    pub(crate) fn setter_8(&self) -> ColorSetter {
        self.imp().setter_8.get()
    }

    fn set_color_active_setter(&self, color: gdk::RGBA) {
        let imp = self.imp();

        if imp.setter_1.is_active() {
            imp.setter_1.set_color(color);
        } else if imp.setter_2.is_active() {
            imp.setter_2.set_color(color);
        } else if imp.setter_3.is_active() {
            imp.setter_3.set_color(color);
        } else if imp.setter_4.is_active() {
            imp.setter_4.set_color(color);
        } else if imp.setter_5.is_active() {
            imp.setter_5.set_color(color);
        } else if imp.setter_6.is_active() {
            imp.setter_6.set_color(color);
        } else if imp.setter_7.is_active() {
            imp.setter_7.set_color(color);
        } else if imp.setter_8.is_active() {
            imp.setter_8.set_color(color);
        }
    }

    fn set_color_active_pad(&self, color: gdk::RGBA) {
        if self.imp().stroke_color_pad.is_active() {
            self.set_stroke_color(color);
        } else if self.imp().fill_color_pad.is_active() {
            self.set_fill_color(color);
        }
    }

    pub(crate) fn deselect_setters(&self) {
        let imp = self.imp();

        imp.setter_1.set_active(false);
        imp.setter_2.set_active(false);
        imp.setter_3.set_active(false);
        imp.setter_4.set_active(false);
        imp.setter_5.set_active(false);
        imp.setter_6.set_active(false);
        imp.setter_7.set_active(false);
        imp.setter_8.set_active(false);
    }
}
