use encoding::{Encoding, DecoderTrap};
use encoding::all::WINDOWS_1252;

fn decode_latin1(input: &[u8]) -> Option<String> {
    match WINDOWS_1252.decode(input,DecoderTrap::Strict) {
        Ok(s) => Some(s),
        _ => None,
    }
}
pub fn encode_latin1(input: &[u8]) -> String {
    String::new()
}

pub fn read_utf8_or_latin1(input: Vec<u8>) -> Option<String> {
    match String::from_utf8(input.clone()) {
        Err(_e) => decode_latin1(&input),
        Ok(s) => Some(s),
    }
}