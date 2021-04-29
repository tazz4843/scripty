use std::{fs::File};

use audrey::read::Reader;
use dasp_interpolate::linear::Linear;
use dasp_signal::{from_iter, interpolate::Converter, Signal};
use deepspeech::{errors::DeepspeechError};
use std::sync::Arc;
use crate::ds_model::DsModel;

// The model has been trained on this specific
// sample rate.
pub const SAMPLE_RATE: u32 = 16_000;

pub async fn run_stt(audio_file_path: String, model: Arc<DsModel>) -> Result<String, DeepspeechError> {
    // Run the speech to text algorithm
    tokio::task::spawn_blocking(move || {
        let mut m = model.model.write().expect("lock was poisoned");

        let audio_file = File::open(audio_file_path).unwrap();
        let mut reader = Reader::new(audio_file).unwrap();
        let desc = reader.description();
        assert_eq!(
            1,
            desc.channel_count(),
            "The channel count is required to be one, at least for now"
        );

        // Obtain the buffer of samples
        let audio_buf: Vec<_> = if desc.sample_rate() == SAMPLE_RATE {
            reader.samples().map(|s| s.unwrap()).collect()
        } else {
            // We need to interpolate to the target sample rate
            let interpolator = Linear::new([0i16], [0]);
            let conv = Converter::from_hz_to_hz(
                from_iter(reader.samples::<i16>().map(|s| [s.unwrap()])),
                interpolator,
                desc.sample_rate() as f64,
                SAMPLE_RATE as f64,
            );
            conv.until_exhausted().map(|v| v[0]).collect()
        };

        // Run the speech to text algorithm
        m.speech_to_text(&audio_buf)
    })
    .await
    .expect("Failed to spawn blocking!")
}
