use encoding::{Encoding, DecoderTrap, EncoderTrap};
use encoding::all::WINDOWS_1252;
use crate::error::MergerError;

fn decode_latin1(input: &[u8]) -> Option<String> {
    match WINDOWS_1252.decode(input,DecoderTrap::Strict) {
        Ok(s) => Some(s),
        _ => None,
    }
}
pub fn encode_latin1(input: String) -> Option<Vec<u8>> {
    match WINDOWS_1252.encode(&input, EncoderTrap::Strict) {
        Ok(s) => Some(s),
        Err(_e) => None,
    }
}

fn read_utf8_or_latin1(input: Vec<u8>) -> Option<String> {
    match String::from_utf8(input.clone()) {
        Err(_e) => decode_latin1(&input),
        Ok(s) => Some(s),
    }
}

pub fn normalize_line_endings(data: String) -> String {
    data.replace("\r\n", "\n").replace("\n", "\r\n")
}

pub fn read_bytes_to_string(input: Vec<u8>, decode: bool, normalize: bool) -> Result<String,MergerError> {
    //TODO: Update both decoding functions to return actual errors
    let output: String = if decode {
        read_utf8_or_latin1(input).ok_or(MergerError::UnknownError)?
    } else {
        String::from_utf8(input).or(Err(MergerError::UnknownError))?
    };

    if normalize {
        Ok(normalize_line_endings(output))
    } else {
        Ok(output)
    }
}
