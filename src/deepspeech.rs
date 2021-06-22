use crate::globals::BotConfig;
use dasp_interpolate::linear::Linear;
use dasp_signal::{from_iter, interpolate::Converter, Signal};
use deepspeech::{errors::DeepspeechError, Model as DsModel};
use std::{path::Path, sync::{Arc, RwLock}};

// The model has been trained on this specific
// sample rate.
pub const SAMPLE_RATE: u32 = 16_000;

pub struct Model {
    ds_model: DsModel
}

unsafe impl Send for Model {}
unsafe impl Sync for Model {}

impl Model {
    pub fn load_from_files(model_path: &Path) -> Self {
        Self { ds_model: DsModel::load_from_files(model_path).expect("failed to load model") }
    }

    pub fn speech_to_text(&mut self, buffer: &[i16]) -> Result<String, DeepspeechError> {
        self.ds_model.speech_to_text(buffer)
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

pub async fn run_stt(input_data: Vec<i16>, m: Arc<RwLock<Model>>) -> Result<String, DeepspeechError> {
    // Run the speech to text algorithm
    tokio::task::spawn_blocking(move || {
        let input_data = {
            // div 4 here is because we ignore two of the chunks and sum the remaining two and div by two
            // which results in 3 of them being essentially ignored
            let mut result = Vec::with_capacity(input_data.len()/4_usize);

            // there's other things we could use but this is a const so should be faster
            let (_, chunks) = input_data.as_rchunks::<4>();

            // the reason for the unsafe code here is because this is in the hot path and will
            // (probably) be called very often, so we want it to be fast, and we know some things
            // for sure so we can use unsafe with those things we know
            for chunk in chunks {
                let left = unsafe {
                    // SAFETY: the chunk size is determined by a constant value and will always be == 4
                    chunk.get_unchecked(0)
                };
                let right = unsafe {
                    // SAFETY: see above
                    chunk.get_unchecked(1)
                };
                result.push((left + right) / 2_i16);
            }
            result
        };

        let interpolator = Linear::new([0i16], [0]);
        let conv = Converter::from_hz_to_hz(
            from_iter(input_data.iter().map(|v| [*v]).collect::<Vec<_>>()),
            interpolator,
            48000_f64,
            SAMPLE_RATE as f64,
        );
        let audio_buf: Vec<_> = conv.until_exhausted().map(|v| v[0]).collect();

        // Run the speech to text algorithm
        let mut model = m.write().expect("a thread panicked while trying to load the model");
        model.speech_to_text(&audio_buf)
    })
    .await
    .expect("Failed to spawn blocking!")
}
