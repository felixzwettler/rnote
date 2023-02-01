use gettextrs::gettext;

pub(crate) const APP_LICENSE: gtk4::License = gtk4::License::Gpl30;

// Make sure the icons are actually installed
pub(crate) const WORKSPACELISTENTRY_ICONS_LIST: &[&str] = &[
    "workspacelistentryicon-bandaid-symbolic",
    "workspacelistentryicon-bank-symbolic",
    "workspacelistentryicon-bookmark-symbolic",
    "workspacelistentryicon-book-symbolic",
    "workspacelistentryicon-bread-symbolic",
    "workspacelistentryicon-calendar-symbolic",
    "workspacelistentryicon-camera-symbolic",
    "workspacelistentryicon-chip-symbolic",
    "workspacelistentryicon-code-symbolic",
    "workspacelistentryicon-compose-symbolic",
    "workspacelistentryicon-document-symbolic",
    "workspacelistentryicon-drinks-symbolic",
    "workspacelistentryicon-flag-symbolic",
    "workspacelistentryicon-folder-symbolic",
    "workspacelistentryicon-footprints-symbolic",
    "workspacelistentryicon-gamepad-symbolic",
    "workspacelistentryicon-gear-symbolic",
    "workspacelistentryicon-hammer-symbolic",
    "workspacelistentryicon-heart-symbolic",
    "workspacelistentryicon-hourglass-symbolic",
    "workspacelistentryicon-key-symbolic",
    "workspacelistentryicon-language-symbolic",
    "workspacelistentryicon-lightbulb-symbolic",
    "workspacelistentryicon-math-symbolic",
    "workspacelistentryicon-meeting-symbolic",
    "workspacelistentryicon-money-symbolic",
    "workspacelistentryicon-musicnote-symbolic",
    "workspacelistentryicon-paintbrush-symbolic",
    "workspacelistentryicon-pencilandpaper-symbolic",
    "workspacelistentryicon-people-symbolic",
    "workspacelistentryicon-person-symbolic",
    "workspacelistentryicon-projector-symbolic",
    "workspacelistentryicon-scratchpad-symbolic",
    "workspacelistentryicon-shapes-symbolic",
    "workspacelistentryicon-shopping-symbolic",
    "workspacelistentryicon-speechbubble-symbolic",
    "workspacelistentryicon-speedometer-symbolic",
    "workspacelistentryicon-star-symbolic",
    "workspacelistentryicon-terminal-symbolic",
    "workspacelistentryicon-text-symbolic",
    "workspacelistentryicon-travel-symbolic",
    "workspacelistentryicon-weather-symbolic",
    "workspacelistentryicon-weight-symbolic",
];

pub(crate) fn workspace_icons_list_to_display_name(icon_name: &str) -> String {
    match icon_name {
        "workspacelistentryicon-bandaid-symbolic" => gettext("Band-Aid"),
        "workspacelistentryicon-bank-symbolic" => gettext("Bank"),
        "workspacelistentryicon-bookmark-symbolic" => gettext("Bookmark"),
        "workspacelistentryicon-book-symbolic" => gettext("Book"),
        "workspacelistentryicon-bread-symbolic" => gettext("Bread"),
        "workspacelistentryicon-calendar-symbolic" => gettext("Calendar"),
        "workspacelistentryicon-camera-symbolic" => gettext("Camera"),
        "workspacelistentryicon-chip-symbolic" => gettext("Chip"),
        "workspacelistentryicon-code-symbolic" => gettext("Code"),
        "workspacelistentryicon-compose-symbolic" => gettext("Compose"),
        "workspacelistentryicon-document-symbolic" => gettext("Document"),
        "workspacelistentryicon-drinks-symbolic" => gettext("Drinks"),
        "workspacelistentryicon-flag-symbolic" => gettext("Flag"),
        "workspacelistentryicon-folder-symbolic" => gettext("Folder"),
        "workspacelistentryicon-footprints-symbolic" => gettext("Footprints"),
        "workspacelistentryicon-gamepad-symbolic" => gettext("Gamepad"),
        "workspacelistentryicon-gear-symbolic" => gettext("Gear"),
        "workspacelistentryicon-hammer-symbolic" => gettext("Hammer"),
        "workspacelistentryicon-heart-symbolic" => gettext("Heart"),
        "workspacelistentryicon-hourglass-symbolic" => gettext("Hourglass"),
        "workspacelistentryicon-key-symbolic" => gettext("Key"),
        "workspacelistentryicon-language-symbolic" => gettext("Language"),
        "workspacelistentryicon-lightbulb-symbolic" => gettext("Lightbulb"),
        "workspacelistentryicon-math-symbolic" => gettext("Math"),
        "workspacelistentryicon-meeting-symbolic" => gettext("Meeting"),
        "workspacelistentryicon-money-symbolic" => gettext("Money"),
        "workspacelistentryicon-musicnote-symbolic" => gettext("Musical Note"),
        "workspacelistentryicon-paintbrush-symbolic" => gettext("Paintbrush"),
        "workspacelistentryicon-pencilandpaper-symbolic" => gettext("Pencil and Paper"),
        "workspacelistentryicon-people-symbolic" => gettext("People"),
        "workspacelistentryicon-person-symbolic" => gettext("Person"),
        "workspacelistentryicon-projector-symbolic" => gettext("Projector"),
        "workspacelistentryicon-scratchpad-symbolic" => gettext("Scratchpad"),
        "workspacelistentryicon-shapes-symbolic" => gettext("Shapes"),
        "workspacelistentryicon-shopping-symbolic" => gettext("Shopping"),
        "workspacelistentryicon-speechbubble-symbolic" => gettext("Speech Bubble"),
        "workspacelistentryicon-speedometer-symbolic" => gettext("Speedometer"),
        "workspacelistentryicon-star-symbolic" => gettext("Star"),
        "workspacelistentryicon-terminal-symbolic" => gettext("Terminal"),
        "workspacelistentryicon-text-symbolic" => gettext("Text"),
        "workspacelistentryicon-travel-symbolic" => gettext("Travel"),
        "workspacelistentryicon-weather-symbolic" => gettext("Weather"),
        "workspacelistentryicon-weight-symbolic" => gettext("Weight"),
        _ => unimplemented!(),
    }
}

pub(crate) const CURSORS_LIST: &[&str] = &[
    "cursor-crosshair-small",
    "cursor-crosshair-medium",
    "cursor-crosshair-large",
    "cursor-dot-small",
    "cursor-dot-medium",
    "cursor-dot-large",
    "cursor-teardrop-nw-small",
    "cursor-teardrop-nw-medium",
    "cursor-teardrop-nw-large",
    "cursor-teardrop-ne-small",
    "cursor-teardrop-ne-medium",
    "cursor-teardrop-ne-large",
    "cursor-teardrop-n-small",
    "cursor-teardrop-n-medium",
    "cursor-teardrop-n-large",
    "cursor-beam-small",
    "cursor-beam-medium",
    "cursor-beam-large",
];

pub(crate) fn cursors_list_to_display_name(icon_name: &str) -> String {
    match icon_name {
        "cursor-crosshair-small" => gettext("Crosshair (Small)"),
        "cursor-crosshair-medium" => gettext("Crosshair (Medium)"),
        "cursor-crosshair-large" => gettext("Crosshair (Large)"),
        "cursor-dot-small" => gettext("Dot (Small)"),
        "cursor-dot-medium" => gettext("Dot (Medium)"),
        "cursor-dot-large" => gettext("Dot (Large)"),
        "cursor-teardrop-nw-small" => gettext("Teardrop North-West (Small)"),
        "cursor-teardrop-nw-medium" => gettext("Teardrop North-West (Medium)"),
        "cursor-teardrop-nw-large" => gettext("Teardrop North-West (Large)"),
        "cursor-teardrop-ne-small" => gettext("Teardrop North-East (Small)"),
        "cursor-teardrop-ne-medium" => gettext("Teardrop North-East (Medium)"),
        "cursor-teardrop-ne-large" => gettext("Teardrop North-East (Large)"),
        "cursor-teardrop-n-small" => gettext("Teardrop North (Small)"),
        "cursor-teardrop-n-medium" => gettext("Teardrop North (Medium)"),
        "cursor-teardrop-n-large" => gettext("Teardrop North (Large)"),
        "cursor-beam-small" => gettext("Beam (Small)"),
        "cursor-beam-medium" => gettext("Beam (Medium)"),
        "cursor-beam-large" => gettext("Beam (Large)"),
        _ => unimplemented!(),
    }
}
