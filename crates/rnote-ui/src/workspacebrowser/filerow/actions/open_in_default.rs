// Imports
use crate::workspacebrowser::RnFileRow;
use gtk4::{gio, glib, glib::clone};

/// Create a new `open-in-default` action.
pub(crate) fn open_in_default(filerow: &RnFileRow) -> gio::SimpleAction {
    let action_open_in_default = gio::SimpleAction::new("open-in-default", None);
    action_open_in_default.connect_activate(
        clone!(@weak filerow => move |_action_open_in_default, _| {
            if let Some(_current_file) = filerow.current_file() {
                log::debug!("Open in default");
            }
        }),
    );

    action_open_in_default
}
