use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use rnote_engine::engine::{import::XoppImportPrefs, EngineSnapshot};
use rnote_engine::Engine;
use smol::fs::File;
use smol::io::{AsyncReadExt, AsyncWriteExt};
use std::path::PathBuf;
use std::time::Duration;

use crate::export::{run_export, ExportCommands};
use crate::validators;

///    rnote-cli{n}{n}
///    This program is free software; you can redistribute it and/or modify it under the terms of the GPL v3 or
///    (at your option) any later version.
#[derive(Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Tests if the specified files can be opened and are valid rnote files.
    Test {
        /// The rnote files.
        rnote_files: Vec<PathBuf>,
    },
    /// Imports the specified input file and saves it as a rnote save file.{n}
    /// Currently only `.xopp` files can be imported.
    Import {
        /// The rnote save file.
        rnote_file: PathBuf,
        /// The import input file.
        #[arg(short = 'i', long)]
        input_file: PathBuf,
        /// When importing a .xopp file, the import dpi can be specified.
        #[arg(long, default_value_t = XoppImportPrefs::default().dpi)]
        xopp_dpi: f64,
    },
    /// Exports the Rnote file(s) and saves it in the desired format.{n}
    /// See sub-commands for usage.
    Export {
        #[command(subcommand)]
        export_command: ExportCommands,
        /// The rnote save file.
        #[arg(global = true)]
        rnote_files: Vec<PathBuf>,
        /// The action that will be performed if the to be exported file(s) already exist(s).
        #[arg(long, default_value = "ask", global = true)]
        on_conflict: OnConflict,
        /// Export without background.
        #[arg(short = 'b', long, action = ArgAction::SetTrue, global = true)]
        no_background: bool,
        /// Export without background pattern.
        #[arg(short = 'p', long, action = ArgAction::SetTrue, global = true)]
        no_pattern: bool,
        /// Optimize the background and stroke colors for printing.
        #[arg(long, action = ArgAction::SetTrue, global = true)]
        optimize_printing: bool,
        /// Inspect the result after the export is finished.{n}
        /// Opens output folder when using "doc-pages" sub-command.
        #[arg(long, action = ArgAction::SetTrue, global = true)]
        open: bool,
    },
}

#[derive(ValueEnum, Copy, Clone, Debug, Default)]
pub(crate) enum OnConflict {
    #[default]
    /// Ask before overwriting.
    Ask,
    /// Overwrite existing files.
    Overwrite,
    #[value(skip)]
    AlwaysOverwrite,
    /// Skip the export.
    Skip,
    #[value(skip)]
    AlwaysSkip,
    /// Append a number as a suffix to the file name.
    Suffix,
    #[value(skip)]
    AlwaysSuffix,
}

impl std::fmt::Display for OnConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ask => "Open existing file for inspection and ask again",
                Self::Overwrite => "Overwrite existing file",
                Self::AlwaysOverwrite => "Always overwrite existing files",
                Self::Skip => "Skip file",
                Self::AlwaysSkip => "Always skip file",
                Self::Suffix => "Append a number as a suffix to the file name",
                Self::AlwaysSuffix => "Always append a number as a suffix to the file name",
            }
        )
    }
}

pub(crate) async fn run() -> anyhow::Result<()> {
    let mut engine = Engine::default();

    let cli = Cli::parse();

    match cli.command {
        Commands::Test { rnote_files } => {
            println!("Testing..");

            for rnote_file in rnote_files.into_iter() {
                validators::file_has_ext(&rnote_file, "rnote")?;
                let file_disp = rnote_file.display().to_string();
                let pb = new_pb(format!("Testing file \"{file_disp}\""));
                // test
                if let Err(e) = test_file(&mut engine, rnote_file).await {
                    let msg = format!("Test failed, Err: {e:?}");
                    if pb.is_hidden() {
                        println!("{msg}");
                    }
                    pb.abandon_with_message(msg);
                    return Err(e);
                } else {
                    let msg = format!("Test succeeded for file \"{file_disp}\"");
                    if pb.is_hidden() {
                        println!("{msg}");
                    }
                    pb.finish_with_message(msg);
                }
            }

            println!("Tests finished successfully!");
        }
        Commands::Import {
            rnote_file,
            input_file,
            xopp_dpi,
        } => {
            validators::file_has_ext(&rnote_file, "rnote")?;
            // xopp files dont require file extensions
            validators::path_is_file(&input_file)?;
            println!("Importing..");

            // apply given arguments to import prefs
            engine.import_prefs.xopp_import_prefs.dpi = xopp_dpi;

            let rnote_file_disp = rnote_file.display().to_string();
            let input_file_disp = input_file.display().to_string();
            let pb = new_pb(format!(
                "Importing \"{input_file_disp}\" to: \"{rnote_file_disp}\""
            ));
            // import
            if let Err(e) = import_file(&mut engine, input_file, rnote_file).await {
                let msg = format!(
                    "Import \"{input_file_disp}\" to \"{rnote_file_disp}\" failed, Err: {e:?}"
                );
                if pb.is_hidden() {
                    println!("{msg}");
                }
                pb.abandon_with_message(msg);
                return Err(e);
            } else {
                let msg =
                    format!("Import \"{input_file_disp}\" to \"{rnote_file_disp}\" succeeded");
                if pb.is_hidden() {
                    println!("{msg}");
                }
                pb.finish_with_message(msg);
            }

            println!("Import finished!");
        }
        Commands::Export {
            rnote_files,
            no_background,
            no_pattern,
            optimize_printing,
            on_conflict,
            open,
            export_command,
        } => {
            println!("Exporting..");
            run_export(
                export_command,
                &mut engine,
                rnote_files,
                no_background,
                no_pattern,
                optimize_printing,
                on_conflict,
                open,
            )
            .await?
        }
    }

    Ok(())
}

pub(crate) fn new_pb(message: String) -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new_spinner().with_message(message);
    pb.set_draw_target(indicatif::ProgressDrawTarget::stdout());
    pb.enable_steady_tick(Duration::from_millis(8));
    pb
}

pub(crate) async fn test_file(_engine: &mut Engine, rnote_file: PathBuf) -> anyhow::Result<()> {
    let mut rnote_bytes = vec![];
    File::open(rnote_file)
        .await?
        .read_to_end(&mut rnote_bytes)
        .await?;

    let _ = EngineSnapshot::load_from_rnote_bytes(rnote_bytes).await?;
    // Loading a valid engine snapshot can't fail, so we skip it
    Ok(())
}

pub(crate) async fn import_file(
    engine: &mut Engine,
    input_file: PathBuf,
    rnote_file: PathBuf,
) -> anyhow::Result<()> {
    let mut input_bytes = vec![];
    let Some(rnote_file_name) = rnote_file.file_name().map(|s| s.to_string_lossy().to_string()) else {
        return Err(anyhow::anyhow!("Failed to get filename from rnote_file"));
    };

    let mut ifh = File::open(input_file).await?;
    ifh.read_to_end(&mut input_bytes).await?;

    let snapshot =
        EngineSnapshot::load_from_xopp_bytes(input_bytes, engine.import_prefs.xopp_import_prefs)
            .await?;

    let _ = engine.load_snapshot(snapshot);

    let rnote_bytes = engine.save_as_rnote_bytes(rnote_file_name).await??;

    let mut ofh = File::create(rnote_file).await?;
    ofh.write_all(&rnote_bytes).await?;
    ofh.sync_all().await?;

    Ok(())
}
