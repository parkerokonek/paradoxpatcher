use std::path::{Path,PathBuf};
use std::collections::{HashSet,HashMap};

pub struct ModInfo {
    mod_path: PathBuf,
    file_tree: HashSet<String>,
    data_path: PathBuf,
    name: String,
    dependencies: Vec<String>,
    replacement_paths: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct ModConflict {
    file_path: PathBuf,
    mod_names: Vec<String>,
}

impl ModConflict {
    pub fn new(path: PathBuf, mods: &[String]) -> ModConflict {
        ModConflict{ file_path: path, mod_names: mods.iter().cloned().collect()}
    }

    pub fn compare_mods(mod_list: &[ModInfo]) -> Vec<ModConflict> {
        let mut out = Vec::new();
        let mut conflicts: HashMap<String,ModConflict> = HashMap::new();

        for mod_info in mod_list {
            for file_path in &mod_info.file_tree {
                if let Some(_conf) = conflicts.get(file_path) {
                    let conf = conflicts.get_mut(file_path).unwrap();
                    conf.mod_names.push(mod_info.name.clone());
                } else {
                    let conf = ModConflict::new(PathBuf::from(file_path),&[mod_info.name.clone()]);
                }
            }
        }

        for conf in conflicts {
            if conf.1.is_real() {
                out.push(conf.1);
            }
        }

        out
    }

    fn is_real(&self) -> bool {
        self.mod_names.len() > 1
    }
}

impl ModInfo {
    pub fn new(mod_path: PathBuf, file_list: &[&str], data_path: PathBuf, name: String, dependencies: &[String], replacement_paths: &[PathBuf]) -> ModInfo {
        let file_tree = ModInfo::list_to_tree(file_list);
        ModInfo {mod_path,file_tree,data_path,name,dependencies: dependencies.iter().cloned().collect(),replacement_paths: replacement_paths.iter().cloned().collect()}
    }

    fn list_to_tree(list: &[&str]) -> HashSet<String> {
        let mut set = HashSet::new();
        for item in list {
            set.insert(item.to_string());
        }
        
        set
    }

    pub fn empty(mod_path: PathBuf, data_path: PathBuf, name: String) -> ModInfo {
        let dependencies = Vec::new();
        let replacement_paths = Vec::new();
        ModInfo{mod_path,file_tree: HashSet::new(),data_path,name,dependencies,replacement_paths}
    }

}