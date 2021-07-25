use deepspeech::{errors::DeepspeechError, Metadata, Model as DsModel};
use scripty_config::BotConfig;
use std::{
    path::Path,
    sync::{Arc, RwLock},
};

// The model has been trained on this specific
// sample rate. This is in Hz.
pub const SAMPLE_RATE: u32 = 16_000;

pub struct Model {
    ds_model: DsModel,
}

// these two impls SHOULD
unsafe impl Send for Model {}
unsafe impl Sync for Model {}

impl Model {
    pub fn load_from_files(model_path: &Path) -> Self {
        Self {
            ds_model: DsModel::load_from_files(model_path).expect("failed to load model"),
        }
    }

    pub fn speech_to_text(&self, buffer: &[i16]) -> Result<String, DeepspeechError> {
        self.ds_model.speech_to_text(buffer)
    }

    pub fn speech_to_text_with_metadata(
        &self,
        buffer: &[i16],
    ) -> Result<Metadata, DeepspeechError> {
        self.ds_model.speech_to_text_with_metadata(buffer, 1)
    }

    pub fn enable_external_scorer(&mut self, scorer_path: &Path) -> Result<(), DeepspeechError> {
        self.ds_model.enable_external_scorer(scorer_path)
    }
}

pub fn load_model() -> Model {
    let model_dir_str = BotConfig::get()
        .expect("Failed to load config!")
        .model_path();
    let dir_path = Path::new(model_dir_str);
    let mut graph_name: Box<Path> = dir_path.join("output_graph.pb").into_boxed_path();
    let mut scorer_name: Option<Box<Path>> = None;
    // search for model in model directory
    for file in dir_path
        .read_dir()
        .expect("Specified model dir is not a dir")
        .flatten()
    {
        let file_path = file.path();
        if file_path.is_file() {
            if let Some(ext) = file_path.extension() {
                if ext == "pb" || ext == "pbmm" {
                    graph_name = file_path.into_boxed_path();
                } else if ext == "scorer" {
                    scorer_name = Some(file_path.into_boxed_path());
                }
            }
        }
    }
    let mut m = Model::load_from_files(&graph_name);
    // enable external scorer if found in the model folder
    if let Some(scorer) = scorer_name {
        m.enable_external_scorer(&scorer).unwrap();
    }

    m
}

pub async fn run_stt(
    input_data: Vec<i16>,
    m: Arc<RwLock<Model>>,
) -> Result<Metadata, DeepspeechError> {
    tokio::task::spawn_blocking(move || {
        // Start off by converting from stereo audio to mono.
        let input_data = super::stereo_to_mono(input_data);

        // Then convert from 48KHz to SAMPLE_RATE (usually 16KHz)
        let audio_buf = super::hz_to_hz(input_data, 48_000_f64, SAMPLE_RATE as f64);

        // load the model from the Arc<RwLock<Model>> passed in
        // and panic if any other thread panicked while trying to load the model too
        let model = m
            .read()
            .expect("a thread panicked while trying to load the model");

        // and finally run the actual speech to text algorithm
        model.speech_to_text_with_metadata(&audio_buf)
    })
    .await
    .expect("Failed to spawn blocking!")
}
