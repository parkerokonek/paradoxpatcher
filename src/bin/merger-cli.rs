use paradoxmerger::{parse_configs,ModInfo,ModPack,generate_mod_list,files_in_vanilla,extract_all_files,auto_merge,write_mod_desc_to_folder,ArgOptions};

use std::path::{PathBuf,Path};
use clap::{Arg,App};


fn main() {
    let args = parse_args();
    let config = parse_configs(&args).expect("Couldn't Parse the config file.");
    
    let mut mod_pack = ModPack::new().restrict_paths(&config.valid_paths);
    let mod_list: Vec<ModInfo> = generate_mod_list(&config.mod_path);
    let vanilla = files_in_vanilla(&config);
    let val_ref: Vec<&Path> = vanilla.iter().map(|x| x.as_path()).collect();
    mod_pack.register_vanilla(&val_ref);
    
    mod_pack.add_mods(&mod_list, true, true);

    if !args.dry_run {
        if args.extract{
            println!("Extracting all files, this could take some time.");
            extract_all_files(&mod_pack, &args, &config, false);
        }

        let aout = auto_merge(&config, &args , &mod_pack);

        if let Ok(num_good) = aout {
            let num_mods: f32 = mod_pack.list_conflicts().len() as f32;
            if !mod_pack.list_conflicts().is_empty() {
            let results = 100f32 * (num_good as f32) / num_mods;
            println!("{}% of merges completed successfully",results);
            println!("Unmerged mod files output to folder {}_bad",args.folder_name());
            }
            else {
                println!("No mod conflicts were found");
            }
        }
    }

    match write_mod_desc_to_folder(&args, &mod_pack) {
        Ok(_) => {},
        Err(e) => {eprintln!("{}",e);}
    }
        
}

pub fn parse_args() -> ArgOptions {
    let args = App::new("Parker's Paradox Patcher")
    .version("0.3")
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
    
    ArgOptions::new(config_path,extract,dry_run,verbose,game_id,patch_name)
}