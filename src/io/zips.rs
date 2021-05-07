use crate::io::encodings;
use crate::io::files::find_even_with_case;

use std::path::{Path};
use std::fs::{File};
use std::io::{prelude::*,BufReader};
use std::collections::HashMap;
use zip::read::ZipArchive;
use zip::write::ZipWriter;

pub fn zip_fetch_file_relative(file_path: &Path, zip_archive: &Path, decode: bool, normalize: bool) -> Option<String> {
    let zip_path = find_even_with_case(&zip_archive)?;
        let file = match File::open(&zip_path) {
            Ok(f) => f,
            Err(e) => {eprintln!("{}",e); return None},
        };
        let reader = BufReader::new(file);
        let mut zip_file = match ZipArchive::new(reader) {
            Ok(r) => r,
            Err(e) => {eprintln!("{}",e); return None},
        };
        let zip_content = zip_file.by_name(&file_path.to_str()?);
        
        if let Ok(content) = zip_content {
            if content.is_dir() {
                return None;
            }
            let mut output = Vec::new();
            for byte in content.bytes() {
                if let Ok(c) = byte {
                    output.push(c);
                } else if let Err(e) = byte {
                    eprintln!("{}",e);
                    return None;
                }
            }
            //TODO: Bubble up error returns in zip results
            encodings::read_bytes_to_string(output,decode,normalize).ok()
        } else {
            None
        }
}

pub fn zip_fetch_all_files(zip_archive: &Path) -> HashMap<String,Vec<u8>> {
    let mut results = HashMap::new();
    let zip_path = find_even_with_case(zip_archive);
    if let Some(good_path) = zip_path {
        let file = match File::open(good_path) {
            Ok(p) => p,
            Err(e) => {eprintln!("{}",e); return results},
        };
        let reader = BufReader::new(file);
        let mut zip_file = match ZipArchive::new(reader) {
            Ok(z) => z,
            Err(e) => {eprintln!("{}",e); return results},
        };
        let names: Vec<String> = zip_file.file_names().map(|x| x.to_owned()).collect();
        for full_path in names {
            let zip_result = zip_file.by_name(&full_path);
            if let Ok(mut zip_file_content) = zip_result {
                if zip_file_content.is_file() {
                    let mut buf = Vec::new();
                    let out = zip_file_content.read_to_end(&mut buf);
                    match out {
                        Ok(_) => {results.insert(full_path, buf);},
                        Err(e) => {eprintln!("{}",e);},
                    }
                }
            } else {
                eprintln!("File could not be extracted! {}",full_path);
            }
        }
    }
    results
}

pub fn zip_write_files(zip_path: &Path, staged_data: HashMap<String,Vec<u8>>) -> Result<(),std::io::Error> {
    let file = File::open(&zip_path)?;
    let mut writer = ZipWriter::new(file);
    let options = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (file_path,file_data) in staged_data {
        writer.start_file_from_path(Path::new(&file_path), options)?;
        writer.write_all(&file_data)?;
    }

    writer.finish()?;
    Ok(())
}