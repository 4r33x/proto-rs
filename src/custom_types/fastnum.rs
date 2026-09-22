mod signed;
mod unsigned;

fn decode_exponent(scale: i32) -> Result<i32, crate::DecodeError> {
    // The encoder emits an i16 scale. Reject malformed values before negation
    // or construction, neither of which may panic on untrusted wire data.
    let scale = i16::try_from(scale).map_err(|_| crate::DecodeError::new("decimal scale is out of range"))?;
    Ok(-i32::from(scale))
}
