use encoding::{Encoding, DecoderTrap, EncoderTrap};
use encoding::all::WINDOWS_1252;

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

pub fn read_utf8_or_latin1(input: Vec<u8>) -> Option<String> {
    match String::from_utf8(input.clone()) {
        Err(_e) => decode_latin1(&input),
        Ok(s) => Some(s),
    }
}