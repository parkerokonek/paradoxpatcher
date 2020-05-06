use std::path::{Path,PathBuf};
use rose_tree::RoseTree;

pub struct ModInfo {
    mod_path: PathBuf,
    conflicts: Vec<ModConflict>,
    file_tree: RoseTree<String>,
    data_path: PathBuf,
    name: String,
    dependencies: Vec<String>,
    replacement_paths: Vec<PathBuf>,
}

pub struct ModConflict {
    file_path: PathBuf,
    mod_names: Vec<String>,
}

impl ModConflict {
    pub fn new(path: PathBuf, mods: [String]) -> ModConflict {
        ModConflict{ file_path: path, mod_names: mods.iter().cloned().collect()}
    }
}

impl ModInfo {}