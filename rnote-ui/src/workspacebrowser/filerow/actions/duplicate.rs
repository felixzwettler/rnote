use std::path::PathBuf;

use fs_extra::dir::{CopyOptions, TransitProcessResult};
use fs_extra::{copy_items_with_progress, TransitProcess};
use gtk4::prelude::FileExt;
use gtk4::{gio, glib, glib::clone};

use crate::workspacebrowser::FileRow;

const DUPLICATE_SUFFIX: &str = ".dup";

impl FileRow {
    pub fn duplicate_action(&self) -> gio::SimpleAction {
        let action = gio::SimpleAction::new("duplicate-file", None);

        let yeet = |process: TransitProcess| -> TransitProcessResult {
            clone!(@weak self as filerow => @default-return glib::source::Continue(false), move || {
                let status = {
                    let status = process.copied_bytes / process.total_bytes;
                    status as f64
                };
                filerow.canvas_progressbar().set_fraction(status);
                glib::source::Continue(true)
            });
            TransitProcessResult::ContinueOrAbort
        };

        action.connect_activate(
            clone!(@weak self as filerow => move |_action_duplicate_file, _| {
                if let Some(current_file) = filerow.current_file() {
                    if let Some(current_path) = current_file.path() {
                        let source_path = current_path.clone().into_boxed_path();

                        if source_path.is_dir() {
                            duplicate_dir(current_path, yeet);
                        } else if source_path.is_file() {
                            duplicate_file(current_path);
                        }
                    }
                }
            }),
        );

        action
    }

    // fn get_progress_fn(&self) -> impl Fn(TransitProcess) -> TransitProcessResult {
    //     |processs: TransitProcess| {
    //         TransitProcessResult::ContinueOrAbort
    //     }
    //
    //     // let status = {
    //     //     let status = process_info.copied_bytes / process_info.total_bytes;
    //     //     status as f64
    //     // };
    //
    //     // TransitProcessResult::ContinueOrAbort
    // }
}

fn dummy(process_info: TransitProcess) -> TransitProcessResult {
    println!(
        "Process: {}B/{}B",
        process_info.copied_bytes, process_info.total_bytes
    );
    TransitProcessResult::ContinueOrAbort
}

fn duplicate_file(source_path: PathBuf) {
    if let Some(destination) = get_destination_path(&source_path) {
        let source = source_path.into_boxed_path();

        log::debug!("Duplicate source: {}", source.display());
        log::debug!("Duplicate destination: {}", destination.display());
        if let Err(err) = std::fs::copy(source, destination) {
            log::error!("Couldn't duplicate file: {}", err);
        }
    }
    log::info!("Destination-file for duplication not found.");
}

fn duplicate_dir<F>(source_path: PathBuf, copy_progress: F)
where
    F: Fn(TransitProcess) -> TransitProcessResult,
{
    if let Some(destination) = get_destination_path(&source_path) {
        let source = source_path.into_boxed_path();
        let options = CopyOptions::new();

        log::debug!("Duplicate source: {}", source.display());
        log::debug!("Duplicate destination: {}", destination.display());
        if let Err(err) = copy_items_with_progress(&[source], destination, &options, copy_progress)
        {
            log::error!("Couldn't copy items: {}", err);
        }
    }
}

fn get_destination_path(source_path: &PathBuf) -> Option<PathBuf> {
    if let Some(destination_file_name) = source_path.file_name() {
        let mut destination_file_name = {
            let mut file_name = destination_file_name.to_os_string();
            file_name.push(DUPLICATE_SUFFIX);
            file_name
        };

        let mut destination_path = {
            let mut path = source_path.clone().to_path_buf();
            path.set_file_name(destination_file_name.clone());
            path
        };

        while destination_path.exists() {
            destination_file_name.push(DUPLICATE_SUFFIX);
            destination_path.set_file_name(destination_file_name.clone());
        }

        Some(destination_path)
    } else {
        None
    }
}
