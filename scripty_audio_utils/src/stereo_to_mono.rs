pub fn stereo_to_mono(input_data: Vec<i16>) -> Vec<i16> {
    // div 4 here is because we ignore two of the chunks and sum the remaining two and div by two
    // which results in 3 of them being essentially ignored
    let mut result = Vec::with_capacity(input_data.len() / 4_usize);

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
}
