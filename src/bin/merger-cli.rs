use paradoxmerger::{parse_args,parse_configs,ModInfo,ModPack,generate_mod_list,files_in_vanilla,extract_all_files,auto_merge,write_mod_desc_to_folder};

use std::path::Path;

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