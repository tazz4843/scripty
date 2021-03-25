use std::env::args;
use std::fs::File;
use std::path::Path;

use crate::utils::ModelWrapper;
use audrey::read::Reader;
use dasp_interpolate::linear::Linear;
use dasp_signal::{from_iter, interpolate::Converter, Signal};
use deepspeech::errors::DeepspeechError;
use deepspeech::Model;
use serenity::prelude::Context;

// The model has been trained on this specific
// sample rate.
pub const SAMPLE_RATE: u32 = 16_000;

/*
TODO list:
* better resampling (right now it seems that recognition is impaired compared to manual resampling)...
  maybe use sinc?
* channel cropping
* use clap or something to parse the command line arguments
*/
pub async fn run_stt(ctx: Context, audio_file_path: String) -> Result<String, DeepspeechError> {
    let mut m_lock = ctx
        .data
        .read()
        .await
        .get::<ModelWrapper>()
        .expect("Expected DeepSpeech model to be placed in at initialization.");
    let mut m = m_lock.read().await;

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
        tokio::task::spawn_blocking(conv.until_exhausted().map(|v| v[0]).collect())
            .await
            .expect("Failed to spawn blocking task!")
    };

    // Run the speech to text algorithm
    tokio::task::spawn_blocking(|| m.speech_to_text(&audio_buf))
        .await
        .expect("test")
}
