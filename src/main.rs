mod mod_information;
mod merge_script;
use mod_information::{ModInfo};
use clap::{Arg,App};
use std::path::{PathBuf,Path};
use std::fs::{self,File};
use std::io::prelude::*;
use std::io::BufReader;
use serde::Deserialize;
use regex::Regex;
use zip::read::ZipArchive;
use pyo3::{*,types::PyModule};

struct ArgOptions {
    config_path: PathBuf,
    extract: bool,
    dry_run: bool,
    verbose: bool,
    game_id: String,
    patch_name: String,
}

impl ArgOptions {
    fn folder_name(&self) -> String {
        let mut mod_folder = self.patch_name.clone();
        mod_folder.make_ascii_lowercase();
        mod_folder
    }
}

#[derive(Deserialize,Debug)]
struct ConfigOptions {
    game_name: String,
    mod_path: PathBuf,
    data_path: PathBuf,
    valid_paths: Vec<PathBuf>,
}

#[derive(Deserialize,Debug)]
struct ConfigListItem {
    datapath: String,
    modpath: String,
    valid_paths: Vec<String>,
}

impl From<(String,ConfigListItem)> for ConfigOptions {
    fn from(from_tuple: (String, ConfigListItem)) -> Self {
        let valid_paths: Vec<PathBuf> = from_tuple.1.valid_paths.iter().map(|x| PathBuf::from(&x)).collect();
        ConfigOptions {game_name: from_tuple.0, mod_path: PathBuf::from(from_tuple.1.modpath), data_path: PathBuf::from(from_tuple.1.datapath), valid_paths}
    }
}

fn main() {
    let args = parse_args();
    let config = parse_configs(&args).expect("Couldn't Parse the config file.");
    
    let mut mod_pack = mod_information::ModPack::new().restrict_paths(&config.valid_paths);
    let mod_list: Vec<mod_information::ModInfo> = generate_mod_list(&config.mod_path);
    let vanilla = files_in_vanilla(&config);
    let val_ref: Vec<&Path> = vanilla.iter().map(|x| x.as_path()).collect();
    mod_pack.register_vanilla(&val_ref);
    
    mod_pack.add_mods(&mod_list, true, true);
    
    //println!("{:?}",mod_pack.load_order());
    //for i in mod_pack.list_conflicts() {
        //    i.display();
        //}
    if !args.dry_run {
    let aout = auto_merge(&config, &args , &mod_pack);
        if let Ok(num_good) = aout {
            let results: f32 = 100. * (num_good as f32) / (mod_pack.list_conflicts().len() as f32);
            println!("{}% of merges completed successfully",results);
        }
    }
    let mout = write_mod_desc_to_folder(&args, &mod_pack);
        
}
    
fn parse_args() -> ArgOptions {
        let args = App::new("Parker's Paradox Patcher")
        .version("0.2")
        .about("Merges some mods together automatically sometimes.")
        .author("Parker Okonek")
        .arg(Arg::with_name("config")
        .short("c")
        .long("config")
        .value_name("CONFIG_FILE")
        .help("configuration file to load, defaults to current directory")
        .takes_value(true))
        .arg(Arg::with_name("extract")
        .short("x")
        .long("extract")
        .help("extract all non-conflicting files to a folder"))
        .arg(Arg::with_name("dry-run")
        .short("d")
        .long("dry-run")
        .help("list file conflicts without merging"))
        .arg(Arg::with_name("verbose")
        .short("v")
        .long("verbose")
        .help("print information about processed mods"))
        .arg(Arg::with_name("game_id")
        .required(false)
        .index(2)
        .help("game in the config to use"))
        .arg(Arg::with_name("patch_name")
        .index(1)
        .required(true)
        .help("name of the generated mod"))
        .get_matches();
        
        
        let mut config_path = PathBuf::new();
        config_path.push(args.value_of("config").unwrap_or("./merger.toml"));
        let extract = args.is_present("extract");
        let dry_run = args.is_present("dry-run");
        let verbose = args.is_present("verbose");
        let game_id = String::from(args.value_of("game_id").unwrap_or(""));
        let patch_name: String = String::from(args.value_of("patch_name").unwrap_or("merged_patch"));
        
        ArgOptions{config_path,extract,dry_run,verbose,game_id,patch_name}
}
    
fn parse_configs(arguments: &ArgOptions) -> Result<ConfigOptions,std::io::Error> {
        let mut config_file = File::open(&arguments.config_path);
        if let Ok(mut file_ok) = config_file {
            let mut contents = String::new();
            let err = file_ok.read_to_string(&mut contents);
            if let Ok(_e) = err {
                let configs_untyped: toml::value::Table = toml::from_str(&contents).expect("Configuration file could not be read!");
                
                if !arguments.game_id.is_empty() && configs_untyped.keys().find(|k| *k == &arguments.game_id).is_none() {
                    eprintln!("Game not found in configuration file.");
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "Game not found in configuration file."));
                }
                
                let game_id: String = if !arguments.game_id.is_empty() {
                    arguments.game_id.to_string()
                }
                else {
                    configs_untyped.keys().next().expect("Bye").to_string()
                };
                
                let configs_untyped = configs_untyped[&game_id].to_string();
                
                let config: ConfigListItem = toml::from_str(&configs_untyped).expect("Malformed configuration for game found.");
                
                return Ok(ConfigOptions::from((game_id,config)));
            } else if let Err(e) = err {
                return Err(e);
            }
        } else if let Err(file_bad) = config_file {
            return Err(file_bad);
        }
        Err(std::io::Error::new(std::io::ErrorKind::Other, "This is a config parsing error that should never appear."))
}
    
fn file_to_string(file_path: &Path) -> Option<String> {
        let mut file = File::open(file_path);
        if let Ok(mut file_open) = file {
            let mut contents = String::new();
            for byte in file_open.bytes() {
                if let Ok(c) = byte {
                    contents.push(c as char);
                } else if let Err(e) = byte {
                    eprintln!("{}",e);
                    return None;
                }
            }
            return Some(contents);
        } else if let Err(e) = file {
            eprintln!("{}",e);
        }
        
        None
}
    
fn fgrep(file_path: &Path, re: &Regex, all_matches: bool) -> Vec<String> {
        if let Some(input) = file_to_string(file_path) {
            return grep(&input,re,all_matches);
        }
        eprintln!("Failed to open file.\t{}",file_path.display());
        
        Vec::new()
}
    
fn grep(input: &str, re: &Regex, all_matches: bool) -> Vec<String> {
        let mut matches = re.find_iter(&input);
        if all_matches {
            return matches.map(|x| x.as_str().to_string()).collect();
        } else if let Some(valid) = matches.next() {
            return vec![valid.as_str().to_string()];
        }
        
        Vec::new()
}
    
fn collect_dependencies(mod_path: &Path) -> Vec<String> {
        let escape = r#"\""#;
        let re_dep = Regex::new(r#"(?m)dependencies[^}]+"#).unwrap();
        let re_sing = Regex::new(r#""[^"]+""#).unwrap();
        let results = fgrep(mod_path,&re_dep,false);
        
        if !results.is_empty() {
            let dependencies = results[0].replace(r#"\""#, "").replace("\r","");
            let single_deps = grep(&dependencies,&re_sing,true);
            let single_deps: Vec<String> = single_deps.iter().map(|x| trim_quotes(x)).collect();
            
            return single_deps;
        }
        
        Vec::new()
}
    
fn trim_quotes(input: &str) -> String {
        let left: Vec<&str> = input.split('"').collect();
        if left.len() == 3 {
            return left[1].to_string();
        }
        String::new()
}
    
fn find_even_with_case(path: &Path) -> Option<PathBuf> {
        if let Ok(_file) = File::open(path) {
            return Some(PathBuf::from(&path));
        } else if path.has_root() {
            if let Ok(dir) = std::fs::read_dir(path.parent().unwrap()) {
                for entry in dir {
                    if entry.is_ok() {
                        let entry_path = entry.unwrap().path();
                        let mut lowerpath: String = entry_path.to_str().unwrap().to_string();
                        lowerpath.make_ascii_lowercase();
                        let mut lowercompare: String = String::from(path.to_str().unwrap());
                        lowercompare.make_ascii_lowercase();
                        if lowercompare == lowerpath {
                            return Some(entry_path);
                        }
                    }
                }
            }
        }
        
        None
}
    
fn generate_single_mod(mod_path: &Path, mod_file: &Path) -> Option<mod_information::ModInfo> {
        let re_archive = Regex::new(r#"archive\s*=\s*"[^"]*\.zip""#).unwrap();
        let re_paths = Regex::new(r#"path\s*=\s*"[^"]*""#).unwrap();
        let re_names = Regex::new(r#"name\s*=\s*"[^"]*""#).unwrap();
        let re_replace = Regex::new(r#"replace_path\s*=\s*"[^"]*""#).unwrap();
        let modmod_path: PathBuf = [mod_path,mod_file].iter().collect();
        let dependencies = collect_dependencies(&modmod_path);
        
        let modmod_content = file_to_string(&modmod_path).unwrap_or_default();
        
        let archive: Vec<String> = grep(&modmod_content, &re_archive, false).iter().map(|x| trim_quotes(x) ).collect();
        let mut path: Vec<String> = grep(&modmod_content, &re_paths, false).iter().map(|x| trim_quotes(x)).collect();
        let name: Vec<String> = grep(&modmod_content, &re_names, false).iter().map(|x| trim_quotes(x)).collect();
        let replace_paths: Vec<PathBuf> = grep(&modmod_content, &re_replace, true).iter().map(|x| PathBuf::from(trim_quotes(x))).collect();
        
        let path: Vec<String> = path.into_iter().filter(|x| !&replace_paths.contains(&PathBuf::from(&x))).collect();
        
        if archive.is_empty() && path.is_empty() || !archive.is_empty() && !path.is_empty() {
            eprintln!("{}\n Spaghetti-Os", &modmod_path.display());
            eprintln!("{},{}",archive.is_empty(),path.is_empty());
            return None;
        } else if name.len() == 1 && archive.len() == 1 {
            let zip_path: PathBuf = [mod_path.to_str()?,&archive[0]].iter().collect();
            let zip_path = find_even_with_case(&zip_path)?;
            let file = File::open(&zip_path).unwrap();
            let reader = BufReader::new(file);
            let zipfile = ZipArchive::new(reader).unwrap();
            
            let files: Vec<&str> = zipfile.file_names().collect();
            
            return Some(ModInfo::new(mod_path.to_path_buf(),&files,zip_path,name[0].clone(),&dependencies,&replace_paths));
        } else if name.len() == 1 && path.len() == 1 {
            let dir_path: PathBuf = [mod_path.to_str()?,&path[0]].iter().collect();
            let dir_path = find_even_with_case(&dir_path)?;
            
            let file_check = walk_in_dir(&dir_path,Some(&dir_path));
            let files_ref: Vec<&str> = file_check.iter().map(|x| x.to_str().unwrap_or_default()).collect();
            return Some(ModInfo::new(mod_path.to_path_buf(),&files_ref,dir_path,name[0].clone(),&dependencies,&replace_paths));
        }
        
        None
}
    
fn generate_mod_list(path: &Path) -> Vec<mod_information::ModInfo> {
        let mod_reg = Regex::new("\"mod/[^\"]*\"").unwrap();
        let mut settings = path.to_path_buf();
        settings.push("./settings.txt");
        let enabled_mods = fgrep(&settings,&mod_reg,true);
        if enabled_mods.is_empty() {
            eprintln!("Had an issue reading the settings file.");
            return Vec::new();
        }
        let mut mods = Vec::new();
        for i in enabled_mods {
            let mod_file: PathBuf = PathBuf::from(trim_quotes(&i));
            let smod = generate_single_mod(&path,&mod_file);
            if let Some(good_mod) = smod {
                mods.push(good_mod);
            } else {
                eprintln!("The following mod failed to load:\t{}",mod_file.display());
            }
        }
        mods
}
    
fn files_in_vanilla(config: &ConfigOptions) -> Vec<PathBuf> {
        let vanilla_path = &config.data_path;
        let check_paths: Vec<PathBuf> = config.valid_paths.iter().map(|x| [vanilla_path,x].iter().collect()).collect();
        let mut out = Vec::new();
        for i in &check_paths {
            let mut path_vec: Vec<PathBuf> = Vec::new();
            let bob = |x| {path_vec.push(x)};
            let mut results = walk_in_dir(&i,Some(&vanilla_path));
            out.append(&mut results);
        }
        
        if out.is_empty() && check_paths.is_empty() {
            out = walk_in_dir(vanilla_path,Some(&vanilla_path));
        }
        
        out
}
    
fn walk_in_dir(dir: &Path, relative: Option<&Path>) -> Vec<PathBuf> {
        let mut to_do: Vec<PathBuf> = vec![dir.to_path_buf()];
        let mut output: Vec<PathBuf> = Vec::new();
        'walk: loop {
            let mut found_dirs = Vec::new();
            if to_do.is_empty() {
                break 'walk;
            }
            for directory in &to_do {
                for entry in fs::read_dir(directory).unwrap() {
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
    
fn auto_merge(config: &ConfigOptions, args: &ArgOptions, mod_pack: &mod_information::ModPack) -> Result<u32,()> {
    let mut successful = 0;
    let gil = Python::acquire_gil();
    let py = gil.python();
    let py_code = merge_script::give_script();
    let module = PyModule::from_code(py, &py_code, "merge.py", "mergers");

        for conf in mod_pack.list_conflicts() {
            if args.verbose {
                println!("Attempting to merge: {}",conf.path().display());
            }
            let mut file_contents: Vec<String> = Vec::new();
            let mut vanilla_file = String::new();
            let try_fetch = vanilla_fetch(conf.path(),config);
            if let Some(contents) = try_fetch {
                vanilla_file = contents;
            } else {
                eprintln!("Error opening vanilla file for comparison: {}",conf.path().display());
                return Err(());
            }
            // Requisite setup for python

            for mod_info in conf.list_mods() {
                if let Some(current) = mod_pack.get_mod(&mod_info) {
                    if current.is_zip() {
                        if let Some(contents) = mod_zip_fetch(conf.path(), current) {
                            //println!("{}",contents);
                            file_contents.push(contents);
                        } else {
                            eprintln!("Error unpacking file in previously registered .zip: {}",mod_info);
                        }
                    } else if let Some(contents) = mod_path_fetch(conf.path(), current) {
                        //println!("{}",contents);
                        file_contents.push(contents);
                    } else {
                        eprintln!("Error unpacking file in previously registered folder: {}",mod_info);
                    }
                } else {
                    eprintln!("Was unable to unpack one of the previously read mods: {}",mod_info);
                    return Err(());
                }
            }

            let mut file_content = Ok(None);

            if let Ok(good_module) = module {
                file_content = py_dif_auto(&py,good_module,&vanilla_file, &file_contents, true);
            } else if let Err(e) = module {
                eprintln!("Couldn't build code: {:?}",e.to_object(py));
                return Err(());
            }
            if let Ok(Some(content)) = &file_content {
                //println!("{}",content);
                let try_write = write_to_mod_folder(args, content, conf.path());
                if try_write.is_ok() {
                    successful+=1;
                } else {
                    eprintln!("{}",try_write.err().unwrap());
                }
                continue;
            }
            if let Err(e) = &file_content {
                //let out = exceptions::OSError::e.to_string();
                //eprintln!("{:?}",err);
                eprintln!("Error occurred inside of python code {:?}",e);
                return Err(());
            }
        }
        Ok(successful)
}

fn relative_folder_path(args: &ArgOptions, path: &Path) -> Result<PathBuf,std::io::Error> {
    let current_dir = std::env::current_dir()?;
    let mut mod_folder = args.patch_name.clone();
    mod_folder.make_ascii_lowercase();
    let mod_folder = Path::new(&mod_folder);
    let full_path: PathBuf = [&current_dir,mod_folder,path].iter().collect();
    Ok(full_path)
}

fn write_to_mod_folder(args: &ArgOptions, contents: &str, path: &Path) -> Result<(),std::io::Error> {
    let full_path = relative_folder_path(args, path)?;
    let almost_path = match full_path.parent() {Some(good) => Ok(good),None => Err(std::io::Error::from_raw_os_error(1))}?;
    let _ = fs::create_dir_all(almost_path)?;
    
    fs::write(full_path,contents)?;

    Ok(())
}

fn write_mod_desc_to_folder(args: &ArgOptions, mod_pack: &mod_information::ModPack) -> Result<(),std::io::Error> {
    let mut mod_file_name = PathBuf::from(args.folder_name());
    mod_file_name.set_extension("mod");
    let full_path = relative_folder_path(args, &mod_file_name)?;

    let mut file_contents = format!("name = \"{}\"\narchive = \"mod/{}.zip\"\n",args.patch_name,args.folder_name());


    
    file_contents.push_str("dependencies = {\n");
    for dep in mod_pack.load_order() {
        let dep_text = format!("\"\\\"{}\\\"\"\n",dep);
        file_contents.push_str(&dep_text);
    }
    file_contents.push_str("}\n");

    fs::write(full_path,file_contents)?;
    Ok(())
}
    
fn mod_zip_fetch(dir: &Path, mod_entry: &mod_information::ModInfo) -> Option<String> {
        if !mod_entry.is_zip() {
            return None;
        }
        let zip_archive = mod_entry.get_data_path();
        let zip_path = find_even_with_case(&zip_archive)?;
        let file = File::open(&zip_path).unwrap();
        let mut reader = BufReader::new(file);
        let mut zip_file = ZipArchive::new(reader).unwrap();
        let zip_content = zip_file.by_name(&dir.to_str()?);
        
        if let Ok(content) = zip_content {
            let mut output = String::new();
            for byte in content.bytes() {
                if let Ok(c) = byte {
                    output.push(c as char);
                } else if let Err(e) = byte {
                    eprintln!("{}",e);
                    return None;
                }
            }
            Some(output)
        } else {
            None
        }
}
    
fn mod_path_fetch(dir: &Path, mod_entry: &mod_information::ModInfo) -> Option<String> {
        let full_path: PathBuf = [mod_entry.get_data_path(),dir].iter().collect();
        file_to_string(&full_path)
}
    
fn vanilla_fetch(dir: &Path, config: &ConfigOptions) -> Option<String> {
        let full_path: PathBuf = [&config.data_path,dir].iter().collect();
        file_to_string(&full_path)
}

fn py_dif_auto<'p>(py: &'p Python, good_mergers: &PyModule, original: &String,others: &[String], verbose: bool) -> PyResult<Option<String>> {
    let others: Vec<String> = others.to_vec();
    let auto_result = good_mergers.call1("dif_auto_once",(original.clone(),others,verbose));
    if let Ok(auto_string) = auto_result {
        let results = auto_string.extract();
        if let Ok((valid,output)) = results {
            if valid {
                return Ok(Some(output));
            } else {
                eprintln!("This file will need manual merging")
            }
        } else if let Err(e) = results {
            let err = e.to_object(*py);
            eprintln!("Couldn't extract result: {:?}",err);
            return Err(e);
        }
    } else if let Err(e) = auto_result {
    eprintln!("Calling function failed: {:?}",e.to_object(*py));
    return Err(e);
    }
    Ok(None)
}