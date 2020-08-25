use crate::io::encodings;

use std::path::{PathBuf,Path};
use std::fs::{self,File};
use std::io::{prelude::*,BufReader,Error};
use std::collections::HashMap;

fn paths_to_pathbuf(paths: &[&Path]) -> PathBuf {
    paths.iter().collect()
}

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

pub fn write_file_with_content(file_path: &Path, file_content: &[u8], encode: bool) -> Result<(),std::io::Error> {
    if !encode {
        eprintln!("{} AHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH",encode);
    }
    let prefix_path = match file_path.parent() {
        Some(p) => p,
        None => return Err(std::io::Error::from_raw_os_error(1)),
    };

    let _ = fs::create_dir_all(prefix_path)?;
    if encode {
        eprintln!("Writing encoded: {}",file_path.display());
        let stringified = match encodings::read_bytes_to_string(file_content.to_vec(),true,false) {
            Some(s) => s,
            None => return Err(std::io::Error::from_raw_os_error(1)),
        };
        match encodings::encode_latin1(stringified) {
            Some(content) => fs::write(file_path, content)?,
            None => return Err(std::io::Error::from_raw_os_error(1)),
        };
    } else {
        fs::write(file_path, file_content)?;
    }
    Ok(())
}

pub fn copy_directory_tree(source_dir: &Path, result_dir: &Path, overwrite: bool, ignore_direct: bool) {
    let from_files_abs = walk_in_dir(source_dir, None);
    let to_files_abs = walk_in_dir(result_dir, None);
}