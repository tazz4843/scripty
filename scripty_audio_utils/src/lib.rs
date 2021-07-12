#![feature(slice_as_chunks)]

mod deepspeech;
mod interpolate;
mod stereo_to_mono;

pub use crate::deepspeech::*;
pub use interpolate::*;
pub use stereo_to_mono::*;
