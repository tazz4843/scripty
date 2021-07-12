use dasp_interpolate::linear::Linear;
use dasp_signal::interpolate::Converter;
use dasp_signal::{from_iter, Signal};

pub fn hz_to_hz(input_data: Vec<i16>, source_hz: f64, target_hz: f64) -> Vec<i16> {
    // start off by preparing a linear interpolator for the model
    let interpolator = Linear::new([0i16], [0]);

    // then make a converter that takes this interpolator and converts it
    let conv = Converter::from_hz_to_hz(
        from_iter(input_data.iter().map(|v| [*v]).collect::<Vec<_>>()),
        interpolator,
        source_hz,
        target_hz,
    );

    // finally, perform the actual conversion
    conv.until_exhausted()
        .map(|v| unsafe { *v.get_unchecked(0) })
        .collect()
}
