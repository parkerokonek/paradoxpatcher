use std::collections::HashMap;
use std::path::{Path, PathBuf};

use fxhash::hash64;

use super::mod_conflict::ModConflict;
use super::mod_info::ModInfo;

#[derive(Clone, Debug)]
pub struct ModPack {
    mod_list: Vec<ModInfo>,
    conflicts: Vec<ModConflict>,
    in_vanilla: Vec<PathBuf>,
    mod_lookup: HashMap<String, usize>,
    conflict_lookup: HashMap<String, usize>,
    valid_paths: Vec<PathBuf>,
    valid_extensions: Vec<String>,
    noncrypto_hash: u64,
}

#[derive(Clone, Debug)]
pub struct ModStatus {
    active: bool,
    mod_name: String,
    special_number: ModToken,
    file_path: PathBuf,
}

#[derive(Clone, Copy, Debug)]
pub struct ModToken {
    idx: usize,
    valid_hash: u64,
}

impl ModStatus {
    pub fn name(&self) -> &str {
        &self.mod_name
    }

    pub fn status(&self) -> bool {
        self.active
    }

    pub fn special_number(&self) -> ModToken {
        self.special_number
    }

    pub fn mod_file(&self) -> &Path {
        &self.file_path
    }
}

impl ModPack {
    pub fn default() -> Self {
        ModPack {
            mod_list: Vec::new(),
            conflicts: Vec::new(),
            in_vanilla: Vec::new(),
            mod_lookup: HashMap::new(),
            conflict_lookup: HashMap::new(),
            valid_paths: Vec::new(),
            valid_extensions: Vec::new(),
            noncrypto_hash: hash64(&String::new()),
        }
    }

    pub fn restrict_paths(mut self, valid_paths: &[PathBuf]) -> Self {
        for path in valid_paths {
            self.valid_paths.push(path.clone());
        }

        self
    }

    pub fn restrict_extensions(mut self, valid_extensions: &[String]) -> Self {
        for ext in valid_extensions {
            self.valid_extensions.push(ext.clone());
        }

        self
    }

    pub fn add_mods(&mut self, mods: &[ModInfo], regen: bool, filter_vanilla: bool) {
        for mod_info in mods {
            if let Some(existing) = self.mod_lookup.get(mod_info.get_name()) {
                self.mod_list[*existing] = mod_info.clone();
            } else {
                let id_of = self.mod_list.len();
                self.mod_list.push(mod_info.clone());
                self.mod_lookup
                    .insert(mod_info.get_name().to_string(), id_of);
            }
        }

        self.resort_by_dependencies();

        if regen || filter_vanilla {
            self.generate_conflicts();
            if filter_vanilla {
                let mut i = 0;
                while i != self.conflicts.len() {
                    if !self
                        .in_vanilla
                        .contains(&PathBuf::from(self.conflicts[i].path()))
                    {
                        let _ = self.conflicts.remove(i);
                    } else {
                        i += 1;
                    }
                }
            }
            self.update_hash_fast();
        }
    }

    fn resort_by_dependencies(&mut self) {
        let mut no_deps: Vec<ModInfo> = self
            .mod_list
            .iter()
            .filter(|x| x.list_dependencies().is_empty())
            .cloned()
            .collect();
        let mut no_deps_hash: HashMap<String, usize> = HashMap::new();
        let mut failed_tries = 0;
        for (i, no_dep) in no_deps.iter().enumerate() {
            no_deps_hash.insert(no_dep.get_name().to_string(), i);
        }
        while no_deps.len() < self.mod_list.len() {
            let has_deps: Vec<ModInfo> = self
                .mod_list
                .iter()
                .filter(|x| no_deps_hash.get(x.get_name()).is_none())
                .cloned()
                .collect();
            for dependent in has_deps {
                let mut is_good = true;
                for dependency in dependent.list_dependencies() {
                    if self.mod_lookup.get(dependency).is_some()
                        && no_deps_hash.get(dependency).is_none()
                    {
                        is_good = false;
                        failed_tries += 1;
                        break;
                    }
                }
                if is_good {
                    let old_id = no_deps.len();
                    no_deps.push(dependent.clone());
                    no_deps_hash.insert(dependent.get_name().to_string(), old_id);
                } else if failed_tries > self.mod_list.len().pow(2) {
                    eprintln!(
                        "Gave up on a mod, likely has cyclical dependencies:\t{}",
                        dependent.get_name()
                    );

                    let old_id = no_deps.len();
                    no_deps.push(dependent.clone());
                    no_deps_hash.insert(dependent.get_name().to_string(), old_id);
                }
            }
        }

        self.mod_list = no_deps;
        self.mod_lookup = no_deps_hash;
    }

    pub fn generate_conflicts(&mut self) {
        self.conflicts = ModConflict::compare_mods(
            &self.mod_list,
            if self.valid_paths.is_empty() {
                None
            } else {
                Some(&self.valid_paths)
            },
            if self.valid_extensions.is_empty() {
                None
            } else {
                Some(&self.valid_extensions)
            },
        );
        self.conflict_lookup.clear();
        for (i, conf) in self.conflicts.iter().enumerate() {
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

    pub fn load_order(&self) -> Vec<ModStatus> {
        let mut out = Vec::new();
        for (idx, file) in self.mod_list.iter().enumerate() {
            out.push(ModStatus {
                active: file.get_active(),
                mod_name: file.get_name().to_string(),
                special_number: ModToken {
                    idx,
                    valid_hash: self.noncrypto_hash,
                },
                file_path: PathBuf::from(file.get_mod_path()),
            });
        }
        out
    }

    pub fn get_mod(&self, name: &str) -> Option<&ModInfo> {
        match self.mod_lookup.get(name) {
            Some(real_id) => Some(&self.mod_list[*real_id]),
            None => None,
        }
    }

    pub fn get_mod_token(&self, name: &str) -> Option<ModToken> {
        match self.mod_lookup.get(name) {
            Some(real_id) => Some(ModToken {
                idx: *real_id,
                valid_hash: self.noncrypto_hash,
            }),
            None => None,
        }
    }

    pub fn toggle_by_name(&mut self, name: &str) -> Option<bool> {
        let our_mod = match self.mod_lookup.get(name) {
            None => return None,
            Some(real) => real,
        };
        let old = &self.mod_list[*our_mod].get_active();
        self.mod_list[*our_mod].toggle();
        Some(*old)
    }

    pub fn toggle_by_token(&mut self, token: ModToken) -> Option<bool> {
        if token.valid_hash != self.noncrypto_hash {
            None
        } else {
            let old = &self.mod_list[token.idx].get_active();
            self.mod_list[token.idx].toggle();
            Some(*old)
        }
    }

    pub fn list_replacement_paths(&self) -> Vec<&Path> {
        // Hashmap to preserve insertion order
        let mut replacement_paths: HashMap<&Path, usize> = HashMap::new();
        let mut idx = 0;
        for mod_info in &self.mod_list {
            for replacement_path in mod_info.list_replacement_paths() {
                replacement_paths.insert(replacement_path, idx);
                idx += 1;
            }
        }

        let mut path_list: Vec<(&Path, usize)> = replacement_paths.into_iter().collect();
        path_list.sort_unstable_by(|(_, b1), (_, b2)| b1.cmp(b2));
        path_list.into_iter().map(|(a, _)| a).collect()
    }

    pub fn list_user_dirs(&self) -> Vec<String> {
        let mut user_dirs = Vec::new();
        for mod_info in &self.mod_list {
            if let Some(user_dir) = mod_info.get_user_dir() {
                user_dirs.push(user_dir.clone());
            }
        }
        user_dirs
    }

    fn update_hash_fast(&mut self) {
        let mut hashee = String::new();
        for (game_id, number) in &self.mod_lookup {
            hashee.push_str(&game_id);
            hashee.push_str(&number.to_string());
        }
        self.noncrypto_hash = hash64(&hashee);
    }
}
