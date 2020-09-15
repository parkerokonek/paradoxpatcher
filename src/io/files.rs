use crate::io::encodings;
use crate::io::re;

use std::path::{PathBuf,Path};
use std::fs::{self,File};
use std::io::{prelude::*};
use std::collections::HashMap;
use regex::Regex;

pub fn fetch_file_in_path(file_path: &Path, decode: bool, normalize: bool) -> Option<String> {
    let file = File::open(file_path);
    if let Ok(file_open) = file {
        let mut contents = Vec::new();
        for byte in file_open.bytes() {
            if let Ok(c) = byte {
                contents.push(c);
            } else if let Err(e) = byte {
                eprintln!("{}",e);
                return None;
            }
        }
        encodings::read_bytes_to_string(contents,decode,normalize)
    } else if let Err(e) = file {
        eprintln!("{}",e);
        None
    } else {
        None
    }
}

pub fn find_even_with_case(path: &Path) -> Option<PathBuf> {
    if let Ok(_file) = File::open(path) {
        return Some(PathBuf::from(&path));
    } else if path.has_root() {
        if let Ok(dir) = std::fs::read_dir(path.parent()?) {
            for entry in dir {
                let entry_path = match entry {
                    Ok(p) => p.path(),
                    Err(_e) => continue,
                };
                let mut lowerpath: String = String::from(entry_path.to_str()?);
                let mut lowercompare: String = String::from(path.to_str()?);

                lowerpath.make_ascii_lowercase();
                lowercompare.make_ascii_lowercase();
                
                if lowercompare == lowerpath {
                    return Some(entry_path);
                }
            }
        }
    }
    
    None
}

   
pub fn walk_in_dir(dir: &Path, relative: Option<&Path>) -> Vec<PathBuf> {
    let mut to_do: Vec<PathBuf> = vec![dir.to_path_buf()];
    let mut output: Vec<PathBuf> = Vec::new();
    'walk: loop {
        let mut found_dirs = Vec::new();
        if to_do.is_empty() {
            break 'walk;
        }
        for directory in &to_do {
            let read_directory = match fs::read_dir(directory) {
                Ok(dir) => dir,
                Err(_e) => continue,
            };
            for entry in read_directory {
                if let Ok(new_entry) = entry {
                    if new_entry.path().is_dir() {
                        found_dirs.push(new_entry.path());
                    } else {
                        output.push(new_entry.path());
                    }
                } else {
                    break 'walk;
                }
            }
        }
        
        to_do.clear();
        to_do.append(&mut found_dirs);
    }
    
    if let Some(rel_path) = relative {
        let mut real_out = Vec::new();
        for stripped in output.iter().map(|path| path.strip_prefix(rel_path)) {
            if let Ok(value) = stripped {
                real_out.push(value.to_path_buf());
            }
        }
        real_out
    } else {
        output
    }
}

pub fn relative_folder_path(mod_folder: &Path, path: &Path) -> Result<PathBuf,std::io::Error> {
    let current_dir = std::env::current_dir()?;
    let mod_folder = Path::new(&mod_folder);
    let full_path: PathBuf = [&current_dir,mod_folder,path].iter().collect();
    Ok(full_path)
}

#[allow(dead_code)]
pub fn fetch_file_in_relative_path(path_base: &Path, rel_path: &Path) -> Option<String> {
    let full_path: PathBuf = path_base.join(rel_path);
    fetch_file_in_path(&full_path,false,false)
}

pub fn fetch_all_files_in_path(path: &Path) -> HashMap<String,Vec<u8>>{
    let mut results = HashMap::new();
    let folder_path = match find_even_with_case(&path) {
        Some(p) => p,
        None => return results,
    };
    let all_files_rel = walk_in_dir(&folder_path, Some(&folder_path));
    let all_files_abs = walk_in_dir(&folder_path, None);

    for (rel_path,file_path) in all_files_rel.iter().zip(all_files_abs) {
        let file = File::open(file_path);
        let real_path: String = match rel_path.to_str() {
            Some(s) => s.to_owned(),
            None => {eprintln!("Could not read file path for {}",rel_path.display()); continue},
        };
        if let Ok(mut file_open) = file {
            let mut contents = Vec::new();
            match file_open.read_to_end(&mut contents) {
                Ok(_) => {results.insert(real_path, contents);},
                Err(e) => eprintln!("{}",e),
            };
        }
    }

    results
}

pub fn write_file_with_content(file_path: &Path, file_content: &[u8]) -> Result<(),std::io::Error> {
    let prefix_path = match file_path.parent() {
        Some(p) => p,
        None => {eprintln!("This file path is not allowed, I guess: {}",file_path.display()); return Err(std::io::Error::from_raw_os_error(22))},
    };

    let _result = fs::create_dir_all(prefix_path)?;
    fs::write(file_path, file_content)?;
    Ok(())
}

pub fn write_file_with_string(file_path: &Path, file_content: String, encode: bool) -> Result<(),std::io::Error> {
    let content = if encode {
        encodings::encode_latin1(file_content)
    } else {
        Some(file_content.as_bytes().to_vec())
    };
    match content {
        Some(bytes) => write_file_with_content(file_path, &bytes),
        None => Err(std::io::Error::from_raw_os_error(126)),
    }
}

pub fn copy_directory_tree(source_dir: &Path, result_dir: &Path, overwrite: bool, _ignore_direct: bool) -> Result<(),std::io::Error> {
    //let from_files_abs = walk_in_dir(source_dir, None);
    let from_files_rel = walk_in_dir(source_dir, Some(source_dir));

    for file in from_files_rel {
        let from_abs_path = source_dir.join(&file);
        let to_abs_path = result_dir.join(file);

        if !overwrite || !to_abs_path.exists() {
            fs::copy(from_abs_path,to_abs_path)?;
        }
    }

    Ok(())
}

/// Performs a search using a pre-compiled regular expression on a given file path and returns all matching strings
/// If no matches are found, this returns an empty vector.
/// An error will be printed if the file cannot be opened.
/// # Arguments
/// 
/// * `file_path` - File to open and search for matching strings
/// 
/// * `re` - the pre-compiled regex to match against
/// 
/// * `all_matches` - if true, return all matches, otherwise only return the first match
/// 
pub fn fgrep(file_path: &Path, reg: &Regex, all_matches: bool) -> Vec<String> {
    if let Some(input) = fetch_file_in_path(file_path,true,true) {
        return re::grep(&input,reg,all_matches);
    }
    eprintln!("Failed to open file.\t{}",file_path.display());
    
    Vec::new()
}