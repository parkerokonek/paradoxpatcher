use std::path::{Path,PathBuf};
use std::collections::{HashSet,HashMap};

#[derive(Clone)]
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

pub struct ModPack {
    mod_list: Vec<ModInfo>,
    conflicts: Vec<ModConflict>,
    in_vanilla: Vec<PathBuf>,
    mod_lookup: HashMap<String,usize>,
    conflict_lookup: HashMap<String,usize>,
    valid_paths: Vec<PathBuf>,
}

impl ModPack {
    pub fn new() -> Self {
        ModPack{mod_list: Vec::new(),conflicts: Vec::new(), in_vanilla: Vec::new(),mod_lookup: HashMap::new(), conflict_lookup: HashMap::new(), valid_paths: Vec::new()}
    }

    pub fn restrict_paths(mut self, valid_paths: &[PathBuf]) -> Self {
        for path in valid_paths {
            self.valid_paths.push(path.clone());
        }

        self
    }

    pub fn add_mods(&mut self,mods: &[ModInfo], regen: bool, filter_vanilla: bool) {
        for mod_info in mods {
            if let Some(existing) = self.mod_lookup.get(&mod_info.name) {
                self.mod_list[*existing] = mod_info.clone();
            } else {
                let id_of = self.mod_list.len();
                self.mod_list.push(mod_info.clone());
                self.mod_lookup.insert(mod_info.name.clone(), id_of);
            }
        }

        self.resort_by_dependencies();

        if regen || filter_vanilla {
            self.generate_conflicts();
            if filter_vanilla {
                let mut i = 0;
                while i != self.conflicts.len() {
                    if !self.in_vanilla.contains(&mut self.conflicts[i].file_path) {
                        let _ = self.conflicts.remove(i);
                    } else {
                        i+=1;
                    }
                }
            }
        }
    }

    fn resort_by_dependencies(&mut self) {
        let mut no_deps: Vec<ModInfo> = self.mod_list.iter().filter(|x| x.dependencies.is_empty()).cloned().collect();
        let mut no_deps_hash: HashMap<String,usize> = HashMap::new();
        let mut failed_tries = 0;
        for (i, no_dep) in no_deps.iter().enumerate() {
            no_deps_hash.insert(no_dep.name.clone(), i);
        }
        while no_deps.len() < self.mod_list.len() {
            let has_deps: Vec<ModInfo> = self.mod_list.iter().filter(|x| no_deps_hash.get(&x.name).is_none()).cloned().collect();
            for dependent in has_deps {
                let mut is_good = true;
                for dependency in &dependent.dependencies {
                    if self.mod_lookup.get(dependency).is_some() && no_deps_hash.get(dependency).is_none() {
                        is_good = false;
                        failed_tries+=1;
                        break;
                    }
                }
                if is_good {
                    let old_id = no_deps.len();
                    no_deps.push(dependent.clone());
                    no_deps_hash.insert(dependent.name.clone(),old_id);
                } else if failed_tries > self.mod_list.len().pow(2) {
                    eprintln!("Gave up on a mod, likely has cyclical dependencies:\t{}",&dependent.name);

                    let old_id = no_deps.len();
                    no_deps.push(dependent.clone());
                    no_deps_hash.insert(dependent.name.clone(),old_id);
                }
            }
        }

        self.mod_list = no_deps;
        self.mod_lookup = no_deps_hash;
    }

    pub fn generate_conflicts(&mut self) {
        self.conflicts = ModConflict::compare_mods(&self.mod_list, if self.valid_paths.is_empty() {None} else {Some(&self.valid_paths)});
        self.conflict_lookup.clear();
        for (i,conf) in self.conflicts.iter().enumerate() {
            let key: String = conf.path().to_str().unwrap().to_owned();
            let value: usize = i;
            self.conflict_lookup.insert(key, value);
        }
    }

    pub fn register_vanilla(&mut self, files: &[&Path]) {
        for i in files {
            self.in_vanilla.push(i.to_path_buf());
        }
        self.in_vanilla.dedup();
    }

    pub fn list_conflicts(&self) -> &Vec<ModConflict> {
        &self.conflicts
    }

    pub fn load_order(&self) -> Vec<&str> {
        let mut out = Vec::new();
        for file in &self.mod_list {
            out.push(file.name.as_str());
        }
        out
    }

    pub fn get_mod(&self, name: &str) -> Option<&ModInfo> {
        let id = self.mod_lookup.get(name);
        if let Some(real_id) = id  {
            Some(&self.mod_list[*real_id])
        } else {
            None
        }
    }
}

impl ModConflict {
    pub fn new(path: PathBuf, mods: &[String]) -> ModConflict {
        ModConflict{ file_path: path, mod_names: mods.iter().cloned().collect()}
    }

    pub fn compare_mods(mod_list: &[ModInfo],valid_paths: Option<&Vec<PathBuf>>) -> Vec<ModConflict> {
        let mut out = Vec::new();
        let mut conflicts: HashMap<String,ModConflict> = HashMap::new();

        for mod_info in mod_list {
            for file_path in &mod_info.file_tree {
                let mut file_path = file_path.to_string();
                file_path.make_ascii_lowercase();
                if let Some(_conf) = conflicts.get(&file_path) {
                    conflicts.get_mut(&file_path).unwrap().mod_names.push(mod_info.name.clone());
                } else {
                    let conf = ModConflict::new(PathBuf::from(&file_path),&[mod_info.name.clone()]);
                    if let Some(real_valid) = valid_paths {
                        for valid in real_valid {
                            if conf.in_folder(valid) && &conf.file_path != valid {
                                let component = conf.file_path.components().last();
                                if let Some(end) = component {
                                    if let Some(value) = end.as_os_str().to_str() {
                                        if value.contains('.') {
                                            conflicts.insert(file_path, conf);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        conflicts.insert(file_path, conf);
                    }
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

    pub fn display(&self) {
        println!("{}",self.file_path.display());
        //let components: Vec<std::path::Component> = self.file_path.components().collect();
        //println!("{:?}",components);
        println!("{:?}",self.mod_names);
    }

    pub fn in_folder(&self,folder: &Path) -> bool {
        self.file_path.starts_with(folder)
    }

    pub fn path(&self) -> &Path {
        &self.file_path
    }

    pub fn list_mods(&self) -> &[String] {
        &self.mod_names
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

    pub fn is_zip(&self) -> bool {
        let out = self.data_path.extension();
        if let Some(ext) = out {
            ext == "zip"
        } else {
            false
        }
    }

    pub fn get_data_path(&self) -> &Path {
        &self.data_path
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}