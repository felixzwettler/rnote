// Imports
use anyhow::Context;
use rand::Rng;
use rnote_compose::penevents::KeyboardKey;
use rodio::source::Buffered;
use rodio::{Decoder, Source};
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::time::{self, Duration};

/// The audio player for pen sounds.
pub struct AudioPlayer {
    // we need to hold the output streams, even if they are not used.
    #[allow(unused)]
    marker_outputstream: rodio::OutputStream,
    marker_outputstream_handle: rodio::OutputStreamHandle,
    #[allow(unused)]
    brush_outputstream: rodio::OutputStream,
    brush_outputstream_handle: rodio::OutputStreamHandle,
    #[allow(unused)]
    typewriter_outputstream: rodio::OutputStream,
    typewriter_outputstream_handle: rodio::OutputStreamHandle,

    sounds: HashMap<String, Buffered<Decoder<File>>>,

    brush_sink: Option<rodio::Sink>,
}

impl std::fmt::Debug for AudioPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioPlayer")
            .field("marker_outputstream", &"{.. no debug impl ..}")
            .field("marker_outputstream_handle", &"{.. no debug impl ..}")
            .field("brush_outputstream", &"{.. no debug impl ..}")
            .field("brush_outputstream_handle", &"{.. no debug impl ..}")
            .field("typewriter_outputstream", &"{.. no debug impl ..}")
            .field("typewriter_outputstream_handle", &"{.. no debug impl ..}")
            .field("sounds", &"{.. no debug impl ..}")
            .field("brush_sink", &"{.. no debug impl ..}")
            .finish()
    }
}

impl AudioPlayer {
    pub const PLAY_TIMEOUT_TIME: time::Duration = time::Duration::from_millis(500);
    pub const N_SOUND_FILES_MARKER: usize = 15;
    pub const N_SOUND_FILES_TYPEWRITER: usize = 30;
    pub const SOUND_FILE_BRUSH_SEEK_TIMES_SEC: [f64; 5] = [0.0, 0.91, 4.129, 6.0, 8.56];

    /// Create and initialize new audioplayer.
    /// `pkg_data_dir` is the app data directory which has a "sounds" subfolder containing the sound files
    pub fn new_init(mut pkg_data_dir: PathBuf) -> Result<Self, anyhow::Error> {
        pkg_data_dir.push("sounds/");

        let mut sounds = HashMap::new();

        let (brush_outputstream, brush_outputstream_handle) = rodio::OutputStream::try_default()?;
        let (marker_outputstream, marker_outputstream_handle) = rodio::OutputStream::try_default()?;
        let (typewriter_outputstream, typewriter_outputstream_handle) =
            rodio::OutputStream::try_default()?;

        // Init marker sounds
        for i in 0..Self::N_SOUND_FILES_MARKER {
            let name = format!("marker_{i:02}");
            let buffered = load_sound_from_path(pkg_data_dir.clone(), &name, "wav")?;

            sounds.insert(name, buffered);
        }

        // Init brush sounds
        {
            let name = String::from("brush");
            let buffered = load_sound_from_path(pkg_data_dir.clone(), &name, "wav")?;
            sounds.insert(name, buffered);
        }

        // Init typewriter sounds
        // the enumerated key sounds
        for i in 0..Self::N_SOUND_FILES_TYPEWRITER {
            let name = format!("typewriter_{i:02}");
            let buffered = load_sound_from_path(pkg_data_dir.clone(), &name, "wav")?;
            sounds.insert(name, buffered);
        }

        // the custom sounds
        {
            let name = String::from("typewriter_insert");
            let buffered = load_sound_from_path(pkg_data_dir.clone(), &name, "wav")?;
            sounds.insert(name, buffered);
        }

        {
            let name = String::from("typewriter_thump");
            let buffered = load_sound_from_path(pkg_data_dir.clone(), &name, "wav")?;
            sounds.insert(name, buffered);
        }

        {
            let name = String::from("typewriter_bell");
            let buffered = load_sound_from_path(pkg_data_dir.clone(), &name, "wav")?;
            sounds.insert(name, buffered);
        }

        {
            let name = String::from("typewriter_linefeed");
            let buffered = load_sound_from_path(pkg_data_dir.clone(), &name, "wav")?;
            sounds.insert(name, buffered);
        }

        Ok(Self {
            marker_outputstream,
            marker_outputstream_handle,
            brush_outputstream,
            brush_outputstream_handle,
            typewriter_outputstream,
            typewriter_outputstream_handle,

            sounds,

            brush_sink: None,
        })
    }

    pub fn play_random_marker_sound(&self) {
        let mut rng = rand::thread_rng();
        let marker_sound_index = rng.gen_range(0..Self::N_SOUND_FILES_MARKER);

        match rodio::Sink::try_new(&self.marker_outputstream_handle) {
            Ok(sink) => {
                sink.append(self.sounds[&format!("marker_{marker_sound_index:02}")].clone());
                sink.detach();
            }
            Err(e) => log::error!(
                "failed to create sink in play_random_marker_sound(), Err {:?}",
                e
            ),
        }
    }

    pub fn start_random_brush_sound(&mut self) {
        let mut rng = rand::thread_rng();
        let brush_sound_seek_time_index =
            rng.gen_range(0..Self::SOUND_FILE_BRUSH_SEEK_TIMES_SEC.len());

        match rodio::Sink::try_new(&self.brush_outputstream_handle) {
            Ok(sink) => {
                sink.append(
                    self.sounds["brush"]
                        .clone()
                        .repeat_infinite()
                        .skip_duration(Duration::from_millis(
                            (Self::SOUND_FILE_BRUSH_SEEK_TIMES_SEC[brush_sound_seek_time_index]
                                * 1000.0)
                                .round() as u64,
                        )),
                );

                self.brush_sink = Some(sink);
            }
            Err(e) => log::error!(
                "failed to create sink in start_play_random_brush_sound(), Err {:?}",
                e
            ),
        }
    }

    pub fn stop_random_brush_sond(&mut self) {
        if let Some(brush_sink) = self.brush_sink.take() {
            brush_sink.stop();
        }
    }

    /// Play a typewriter sound that fits the given key type, or a generic sound when None.
    pub fn play_typewriter_key_sound(&self, keyboard_key: Option<KeyboardKey>) {
        match rodio::Sink::try_new(&self.typewriter_outputstream_handle) {
            Ok(sink) => match keyboard_key {
                Some(KeyboardKey::CarriageReturn) | Some(KeyboardKey::Linefeed) => {
                    sink.append(
                        self.sounds["typewriter_bell"].clone().mix(
                            self.sounds["typewriter_linefeed"]
                                .clone()
                                .delay(Duration::from_millis(200)),
                        ),
                    );
                    sink.detach();
                }
                // control characters are already filtered out of unicode
                Some(KeyboardKey::Unicode(_))
                | Some(KeyboardKey::BackSpace)
                | Some(KeyboardKey::Delete)
                | Some(KeyboardKey::HorizontalTab)
                | None => {
                    let mut rng = rand::thread_rng();
                    let typewriter_sound_index = rng.gen_range(0..Self::N_SOUND_FILES_TYPEWRITER);

                    sink.append(
                        self.sounds[&format!("typewriter_{typewriter_sound_index:02}")].clone(),
                    );
                    sink.detach();
                }
                _ => {
                    sink.append(self.sounds["typewriter_thump"].clone());
                    sink.detach();
                }
            },
            Err(e) => log::error!(
                "failed to create sink in play_typewriter_sound(), Err {:?}",
                e
            ),
        }
    }
}

fn load_sound_from_path(
    mut resource_path: PathBuf,
    sound_name: &str,
    ending: &str,
) -> anyhow::Result<Buffered<Decoder<File>>> {
    resource_path.push(format!("{sound_name}.{ending}"));

    if resource_path.exists() {
        let buffered =
            rodio::Decoder::new(File::open(resource_path.clone()).with_context(|| {
                anyhow::anyhow!("file open() for path {:?} failed", resource_path,)
            })?)?
            .buffered();

        // initialize the buffer
        buffered.clone().for_each(|_| {});

        Ok(buffered)
    } else {
        Err(anyhow::Error::msg(format!(
            "failed to init audioplayer. File `{resource_path:?}` is missing."
        )))
    }
}
