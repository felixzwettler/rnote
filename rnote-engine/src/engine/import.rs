use std::ops::Range;

use futures::channel::oneshot;
use rnote_fileformats::{rnoteformat, xoppformat, FileFormatLoader};
use serde::{Deserialize, Serialize};

use crate::document::{background, Background, Format};
use crate::pens::penholder::PenStyle;
use crate::store::chrono_comp::StrokeLayer;
use crate::store::{StoreSnapshot, StrokeKey};
use crate::strokes::{BitmapImage, Stroke, TextStroke, VectorImage};
use crate::{Document, RnoteEngine, StrokeStore, WidgetFlags};

use super::EngineConfig;

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, num_derive::FromPrimitive, num_derive::ToPrimitive,
)]
#[serde(rename = "pdf_import_pages_type")]
pub enum PdfImportPagesType {
    #[serde(rename = "bitmap")]
    Bitmap = 0,
    #[serde(rename = "vector")]
    Vector,
}

impl Default for PdfImportPagesType {
    fn default() -> Self {
        Self::Vector
    }
}

impl TryFrom<u32> for PdfImportPagesType {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        num_traits::FromPrimitive::from_u32(value).ok_or_else(|| {
            anyhow::anyhow!(
                "PdfImportPagesType try_from::<u32>() for value {} failed",
                value
            )
        })
    }
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, num_derive::FromPrimitive, num_derive::ToPrimitive,
)]
#[serde(rename = "pdf_import_page_spacing")]
pub enum PdfImportPageSpacing {
    #[serde(rename = "continuous")]
    Continuous = 0,
    #[serde(rename = "one_per_document_page")]
    OnePerDocumentPage,
}

impl Default for PdfImportPageSpacing {
    fn default() -> Self {
        Self::Continuous
    }
}

impl TryFrom<u32> for PdfImportPageSpacing {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        num_traits::FromPrimitive::from_u32(value).ok_or_else(|| {
            anyhow::anyhow!(
                "PdfImportPageSpacing try_from::<u32>() for value {} failed",
                value
            )
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename = "pdf_import_prefs")]
pub struct PdfImportPrefs {
    /// The pdf pages type
    #[serde(rename = "pages_type")]
    pub pages_type: PdfImportPagesType,
    /// The pdf page width in percentage to the format width
    #[serde(rename = "page_width_perc")]
    pub page_width_perc: f64,
    /// The pdf page spacing
    #[serde(rename = "page_spacing")]
    pub page_spacing: PdfImportPageSpacing,
}

impl Default for PdfImportPrefs {
    fn default() -> Self {
        Self {
            pages_type: PdfImportPagesType::default(),
            page_width_perc: 50.0,
            page_spacing: PdfImportPageSpacing::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename = "xopp_import_prefs")]
pub struct XoppImportPrefs {
    /// The import DPI
    #[serde(rename = "pages_type")]
    pub dpi: f64,
}

impl Default for XoppImportPrefs {
    fn default() -> Self {
        Self { dpi: 96.0 }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default, rename = "import_prefs")]
pub struct ImportPrefs {
    #[serde(rename = "pdf_import_prefs")]
    pub pdf_import_prefs: PdfImportPrefs,
    #[serde(rename = "xopp_import_prefs")]
    pub xopp_import_prefs: XoppImportPrefs,
}

impl RnoteEngine {
    /// opens a .rnote file. We need to split this into two methods,
    /// because we can't have it as a async function and await when the engine is wrapped in a refcell without causing panics :/
    pub fn open_from_rnote_bytes_p1(
        &mut self,
        bytes: Vec<u8>,
    ) -> anyhow::Result<oneshot::Receiver<anyhow::Result<StoreSnapshot>>> {
        let rnote_file = rnoteformat::RnotefileMaj0Min5::load_from_bytes(&bytes)?;

        self.document = serde_json::from_value(rnote_file.document)?;

        let (store_snapshot_sender, store_snapshot_receiver) =
            oneshot::channel::<anyhow::Result<StoreSnapshot>>();

        rayon::spawn(move || {
            let result = || -> anyhow::Result<StoreSnapshot> {
                Ok(serde_json::from_value(rnote_file.store_snapshot)?)
            };

            if let Err(_data) = store_snapshot_sender.send(result()) {
                log::error!("sending result to receiver in open_from_rnote_bytes() failed. Receiver already dropped.");
            }
        });

        Ok(store_snapshot_receiver)
    }

    // Part two for opening a file. imports the store snapshot.
    pub fn open_from_store_snapshot_p2(
        &mut self,
        store_snapshot: &StoreSnapshot,
    ) -> anyhow::Result<()> {
        self.store.import_snapshot(store_snapshot);

        self.update_pens_states();

        Ok(())
    }

    /// Opens a  Xournal++ .xopp file, and replaces the current state with it.
    pub fn open_from_xopp_bytes(&mut self, bytes: Vec<u8>) -> anyhow::Result<()> {
        let xopp_import_prefs = self.import_prefs.xopp_import_prefs;

        let xopp_file = xoppformat::XoppFile::load_from_bytes(&bytes)?;

        // Extract the largest width of all pages, add together all heights
        let (doc_width, doc_height) = xopp_file
            .xopp_root
            .pages
            .iter()
            .map(|page| (page.width, page.height))
            .fold((0_f64, 0_f64), |prev, next| {
                // Max of width, sum heights
                (prev.0.max(next.0), prev.1 + next.1)
            });
        let no_pages = xopp_file.xopp_root.pages.len() as u32;

        let mut doc = Document::default();
        let mut format = Format::default();
        let mut background = Background::default();
        let mut store = StrokeStore::default();

        // We convert all values from the hardcoded 72 DPI of Xopp files to the preferred dpi
        format.dpi = xopp_import_prefs.dpi;

        doc.x = 0.0;
        doc.y = 0.0;
        doc.width = crate::utils::convert_value_dpi(
            doc_width,
            xoppformat::XoppFile::DPI,
            xopp_import_prefs.dpi,
        );
        doc.height = crate::utils::convert_value_dpi(
            doc_height,
            xoppformat::XoppFile::DPI,
            xopp_import_prefs.dpi,
        );

        format.width = crate::utils::convert_value_dpi(
            doc_width,
            xoppformat::XoppFile::DPI,
            xopp_import_prefs.dpi,
        );
        format.height = crate::utils::convert_value_dpi(
            doc_height / (no_pages as f64),
            xoppformat::XoppFile::DPI,
            xopp_import_prefs.dpi,
        );

        if let Some(first_page) = xopp_file.xopp_root.pages.get(0) {
            if let xoppformat::XoppBackgroundType::Solid {
                color: _color,
                style: _style,
            } = &first_page.background.bg_type
            {
                // Xopp background styles are not compatible with Rnotes, so everything is plain for now
                background.pattern = background::PatternStyle::None;
            }
        }

        // Offsetting as rnote has one global coordinate space
        let mut offset = na::Vector2::<f64>::zeros();

        for (_page_i, page) in xopp_file.xopp_root.pages.into_iter().enumerate() {
            for layers in page.layers.into_iter() {
                // import strokes
                for new_xoppstroke in layers.strokes.into_iter() {
                    match Stroke::from_xoppstroke(new_xoppstroke, offset, xopp_import_prefs.dpi) {
                        Ok((new_stroke, layer)) => {
                            store.insert_stroke(new_stroke, Some(layer));
                        }
                        Err(e) => {
                            log::error!(
                                "from_xoppstroke() failed in open_from_xopp_bytes() with Err {}",
                                e
                            );
                        }
                    }
                }

                // import images
                for new_xoppimage in layers.images.into_iter() {
                    match Stroke::from_xoppimage(new_xoppimage, offset, xopp_import_prefs.dpi) {
                        Ok(new_image) => {
                            store.insert_stroke(new_image, None);
                        }
                        Err(e) => {
                            log::error!(
                                "from_xoppimage() failed in open_from_xopp_bytes() with Err {}",
                                e
                            );
                        }
                    }
                }
            }

            // Only add to y offset, results in vertical pages
            offset[1] += crate::utils::convert_value_dpi(
                page.height,
                xoppformat::XoppFile::DPI,
                xopp_import_prefs.dpi,
            );
        }

        doc.background = background;
        doc.format = format;

        // Import into engine
        self.document = doc;
        self.store.import_snapshot(&*store.take_store_snapshot());

        self.update_pens_states();

        Ok(())
    }

    //// generates a vectorimage for the bytes ( from a SVG file )
    pub fn generate_vectorimage_from_bytes(
        &self,
        pos: na::Vector2<f64>,
        bytes: Vec<u8>,
    ) -> oneshot::Receiver<anyhow::Result<VectorImage>> {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel::<anyhow::Result<VectorImage>>();

        rayon::spawn(move || {
            let result = || -> anyhow::Result<VectorImage> {
                let svg_str = String::from_utf8(bytes)?;

                VectorImage::import_from_svg_data(&svg_str, pos, None)
            };

            if let Err(_data) = oneshot_sender.send(result()) {
                log::error!("sending result to receiver in generate_vectorimage_from_bytes() failed. Receiver already dropped.");
            }
        });

        oneshot_receiver
    }

    //// generates a bitmapimage for the bytes ( from a bitmap image file (PNG, JPG) )
    pub fn generate_bitmapimage_from_bytes(
        &self,
        pos: na::Vector2<f64>,
        bytes: Vec<u8>,
    ) -> oneshot::Receiver<anyhow::Result<BitmapImage>> {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel::<anyhow::Result<BitmapImage>>();

        rayon::spawn(move || {
            let result = || -> anyhow::Result<BitmapImage> {
                BitmapImage::import_from_image_bytes(&bytes, pos)
            };

            if let Err(_data) = oneshot_sender.send(result()) {
                log::error!("sending result to receiver in generate_bitmapimage_from_bytes() failed. Receiver already dropped.");
            }
        });

        oneshot_receiver
    }

    //// generates a textstroke for the string
    pub fn generate_textstroke_from_string(
        &self,
        pos: na::Vector2<f64>,
        text: String,
    ) -> anyhow::Result<TextStroke> {
        let mut text_style = self.penholder.typewriter.text_style.clone();
        text_style.ranged_text_attributes.clear();

        Ok(TextStroke::new(text, pos, text_style))
    }

    //// generates image strokes for each page for the bytes ( from a PDF file )
    pub fn generate_pdf_pages_from_bytes(
        &self,
        bytes: Vec<u8>,
        insert_pos: na::Vector2<f64>,
        page_range: Option<Range<u32>>,
    ) -> oneshot::Receiver<anyhow::Result<Vec<(Stroke, Option<StrokeLayer>)>>> {
        let (oneshot_sender, oneshot_receiver) =
            oneshot::channel::<anyhow::Result<Vec<(Stroke, Option<StrokeLayer>)>>>();
        let pdf_import_prefs = self.import_prefs.pdf_import_prefs;

        let format = self.document.format.clone();

        rayon::spawn(move || {
            let result = || -> anyhow::Result<Vec<(Stroke, Option<StrokeLayer>)>> {
                match pdf_import_prefs.pages_type {
                    PdfImportPagesType::Bitmap => {
                        let bitmapimages = BitmapImage::import_from_pdf_bytes(
                            &bytes,
                            pdf_import_prefs,
                            insert_pos,
                            page_range,
                            &format,
                        )?
                        .into_iter()
                        .map(|s| (Stroke::BitmapImage(s), Some(StrokeLayer::Document)))
                        .collect::<Vec<(Stroke, Option<StrokeLayer>)>>();
                        Ok(bitmapimages)
                    }
                    PdfImportPagesType::Vector => {
                        let vectorimages = VectorImage::import_from_pdf_bytes(
                            &bytes,
                            pdf_import_prefs,
                            insert_pos,
                            page_range,
                            &format,
                        )?
                        .into_iter()
                        .map(|s| (Stroke::VectorImage(s), Some(StrokeLayer::Document)))
                        .collect::<Vec<(Stroke, Option<StrokeLayer>)>>();
                        Ok(vectorimages)
                    }
                }
            };

            if let Err(_data) = oneshot_sender.send(result()) {
                log::error!("sending result to receiver in import_pdf_bytes() failed. Receiver already dropped.");
            }
        });

        oneshot_receiver
    }

    /// Imports the generated strokes into the store
    pub fn import_generated_strokes(
        &mut self,
        strokes: Vec<(Stroke, Option<StrokeLayer>)>,
    ) -> WidgetFlags {
        let mut widget_flags = self.store.record();

        let all_strokes = self.store.keys_unordered();
        self.store.set_selected_keys(&all_strokes, false);

        widget_flags.merge_with_other(self.change_pen_style(PenStyle::Selector));

        let inserted = strokes
            .into_iter()
            .map(|(stroke, layer)| self.store.insert_stroke(stroke, layer))
            .collect::<Vec<StrokeKey>>();

        // after inserting the strokes, but before set the inserted strokes selected
        self.resize_to_fit_strokes();

        self.store.set_selected_keys(&inserted, true);

        self.update_pens_states();
        self.update_rendering_current_viewport();

        widget_flags.redraw = true;
        widget_flags.resize = true;
        widget_flags.indicate_changed_store = true;
        widget_flags.refresh_ui = true;

        widget_flags
    }

    /// Exports the current engine config as JSON string
    pub fn save_engine_config(&self) -> anyhow::Result<String> {
        let engine_config = EngineConfig {
            document: serde_json::to_value(&self.document)?,
            penholder: serde_json::to_value(&self.penholder)?,
            import_prefs: serde_json::to_value(&self.import_prefs)?,
            export_prefs: serde_json::to_value(&self.export_prefs)?,
            pen_sounds: serde_json::to_value(&self.pen_sounds)?,
        };

        Ok(serde_json::to_string(&engine_config)?)
    }
}
