#!/usr/bin/python3
# Usable under the MIT or Apache-2.0
# No Warranty or support provided
from zipfile import ZipFile
from collections import defaultdict,OrderedDict
import toml
import re
import os
import math
import argparse
from pathlib import Path
import diff_match_patch as dmp
import shutil

class Tree(defaultdict):
    def __init__(self, value=None):
        super(Tree, self).__init__(Tree)
        self.value = value

class ModInfo:
    def __init__(self,modpath,datapath,name,files,conflicts, depends, replacement_paths, isArchive=False):
        self.modpath = modpath
        self.conflicts = conflicts
        self.files = files
        self.datapath = datapath
        self.name = name
        self.depends = depends
        self.isArchive = isArchive
        self.replacement_paths = replacement_paths
    def full_path(self):
        return self.modpath + "/" + self.datapath

class Conflict:
    def __init__(self,mod_names,path):
        self.path = path
        self.mod_names = mod_names

def caseless_zip(path):
    modpath = path.rsplit("/",1)
    path_zip = modpath[1]
    prepend = modpath[0]

    for file in os.listdir(prepend):
        if path_zip.lower() == file.lower():
            return prepend+"/"+file
    return path

def grep(file,reg,extra=0):
    inputText = ""
    with open(file) as textFile:
        inputText = textFile.read()
    matches = re.findall(reg,inputText, extra)

    return matches

def zip_list(zipPath):
    biglist = []
    with ZipFile(caseless_zip(zipPath), 'r') as modArchive:
        biglist = list(filter(lambda s: s.find("/") != -1, modArchive.namelist()))

    dirs = Tree()

    for path in biglist:
        separated = path.split("/")
        if len(separated) == 2:
            if dirs[separated[0]].value == None:
                dirs[separated[0]].value = [separated[1]]
            else:
                dirs[separated[0]].value.append(separated[1])
        elif len(separated) == 3:
            if dirs[separated[0]].value == None:
                dirs[separated[0]].value = []
            if dirs[separated[0]][separated[1]].value == None:
                dirs[separated[0]][separated[1]].value = [separated[2]]
            else:
                dirs[separated[0]][separated[1]].value.append(separated[2])
    #print(dirs)

    return dirs

def dir_list(dirPath):
    for dirs in os.listdir(dirPath):
        print(dirs)
    return {}

def collect_dependencies(modfile):
    esc_quote = "\\\""
    depends = grep(modfile,"dependencies")
    out = list()
    if len(depends) > 0:
        depends = grep(modfile,"dependencies[^}]+}",re.MULTILINE)
        depends_trimmed = depends[0].replace("\n","").replace("\r","").rstrip("}").split("{")[1].strip()
        depends_trimmed = depends_trimmed.replace(esc_quote,"").replace("\"\"","\"")
        depends_list = depends_trimmed.split("\"")

        
        for i in depends_list:
            if len(i) > 0:
                out.append(i)
    return out

def sort_by_dependencies(mod_list):
    result_list = [x for x in mod_list if len(x.depends) == 0]
    dependent = [x for x in mod_list if len(x.depends) > 0]
    for dep_mod in dependent:
        for dep in dep_mod.depends:
            if dep in map(lambda x: x.name, mod_list) and dep not in map(lambda x: x.name, result_list):
                dependent.append(dep_mod)
                break
        else:
            result_list.append(dep_mod)
    return result_list

def generate_mod_data(modPath):

    matches = grep(modPath+"/settings.txt","\"mod\/[^\"]*\"")
    mod_list = []
    
    for modmod in matches:
        mod_file = modPath+"/"+modmod[1:-1]
        zips = grep(mod_file,"archive\s*=\s*\"[^\"]*\.zip")
        paths = grep(mod_file,"archive\s*=\s*\"[^\"]*\"")
        names = grep(mod_file,"name\s*=\s*\"[^\"]*\"")
        dependencies = collect_dependencies(mod_file)
        replace_paths = grep(mod_file,"replace_path\s*=\s*\"[^\"]\"")

        if (len(zips) < 1 and len(paths) < 1) or len(names) < 1:
            print(modmod)
            print("Spaghetti-Os")
        elif len(names) > 0 and len(zips) > 0:
            zipfile = zips[0].split("\"")[1]
            archive = modPath+"/"+zipfile
            #print(archive)
            name = names[0].split("\"")[1]
            #print(name)
            filetree = zip_list(archive)
            #print(filetree)
            new_mod = ModInfo(modPath,zipfile,name,filetree,[],dependencies,replace_paths,True)
            mod_list.append(new_mod)
        elif len(paths) > 0 and len(names) > 0:
            dirfile = paths[0].split("\"")[1]
            directory = modPath+"/"+dirfile
            name = names[0].split("\"")[1]
            filetree = dir_list(directory)
            new_mod = ModInfo(modPath,dirfile,name,filetree,[],dependencies,replace_paths)
    return sort_by_dependencies(mod_list)


def generate_conflicts(mods,conflict_dirs):
    possible = {}
    for dirs in conflict_dirs:
        for mod in mods:
            if mod.files[dirs].value != None:
                files = mod.files[dirs].value
                for file in files:
                    if not ((dirs+"/"+file).lower() in possible):
                        possible[(dirs+"/"+file).lower()] = []
                    new_list = list(filter(lambda x: x not in mod.depends, possible[(dirs+"/"+file).lower()]))
                    possible[(dirs+"/"+file).lower()] = new_list
                    possible[(dirs+"/"+file).lower()].append(mod.name)
                    
                for folder in mod.files[dirs]:
                    files = mod.files[dirs][folder].value
                    for file in files:
                        if not ((dirs+"/"+folder+"/"+file.lower()) in possible):
                            possible[(dirs+"/"+folder+"/"+file).lower()] = []
                        new_list = list(filter(lambda x: x not in mod.depends, possible[(dirs+"/"+folder+"/"+file).lower()]))
                        possible[(dirs+"/"+folder+"/"+file).lower()] = new_list
                        possible[(dirs+"/"+folder+"/"+file).lower()].append(mod.name)

    dudes = []
    for dude in possible:
        if len(possible[dude]) < 2 :
            dudes.append(dude)

    for dude in dudes:
        del possible[dude]

    return possible

def generate_mod(conflicts,mods,mod_name,mod_archive,file_name,extract=False):
    esc_quote = "\\\""
    file_out = "name = \""+mod_name+"\"\n"
    file_out += "archive = \"mod/"+mod_archive+"\"\n"
    file_out += "dependencies = {\n"
    mod_list = []
    if extract:
        for mod in mods:
            if not mod.name in mod_list and mod.name != mod_name:
                mod_list.append(mod.name)
    else:
        for conflict in conflicts:
            for name in conflicts[conflict]:
                if not name in mod_list and name != mod_name:
                    mod_list.append(name)
    for mod in mod_list:
        if " " in mod:
            file_out += "\""+esc_quote+mod+esc_quote+"\"\n"
        else:
            file_out += "\""+mod+"\"\n"
    file_out += "}\n"

    f = open(file_name,"w")
    f.write(file_out)

def conflicts_against_base(basedir,conflicts,valid_paths):
    in_vanilla = {}
    for conflict in conflicts:
        dirs = conflict.split("/")
        if len(dirs) == 2:
            matches = filter(lambda s: s.lower() == dirs[1],os.listdir(basedir+"/"+dirs[0]))
            for i in matches:
                in_vanilla[conflict] = basedir+"/"+dirs[0]+"/"+i
        elif len(dirs) == 3:
            matches = filter(lambda s: s.lower() == dirs[2],os.listdir(basedir+"/"+dirs[0]+"/"+dirs[1]))
            for i in matches:
                in_vanilla[conflict] = basedir+"/"+dirs[0]+"/"+dirs[1]+"/"+i

    return in_vanilla

def extract_all(conflicts,in_vanilla,mods,modpath,outpath):
    by_mod = {}
    current_path = os.getcwd()
    outpath = os.path.abspath(outpath)
    if not os.path.exists(outpath):
        os.mkdir(outpath, mode=0o777 )
        os.mkdir(outpath+"/vanilla", mode=0o0777)

    for conf in in_vanilla:
        #print(in_vanilla[conf])
        #print(outpath+"/vanilla/"+conf.rsplit("/",1)[0])
        Path(outpath+"/vanilla/"+conf.rsplit("/",1)[0]).mkdir(parents=True,exist_ok=True)
        current = ""
        with open(in_vanilla[conf],"r",encoding="ISO-8859-1") as f:
            current = f.read()
        with open(outpath+"/vanilla/"+conf,"w",encoding="utf-8") as f:
            f.write((current.replace("\r\n","\n")).replace("\n","\r\n")+"\n")

    for conf in conflicts:
        for mod in conflicts[conf]:
            if mod not in by_mod:
                by_mod[mod] = list()
            by_mod[mod].append(conf)
    
    print("------")

    for mod in by_mod:
        new_dir = outpath + "/" + "".join(list(filter(lambda c: c.isalnum() ,mod)))
        if not os.path.exists(new_dir):
            os.mkdir(os.path.abspath(new_dir),mode=0o777)
        os.chdir(os.path.abspath(new_dir))
        #print(os.getcwd())
        valid = []
        biglist = []
        mod_data = next(filter(lambda x: x.name == mod,mods))
        with ZipFile(caseless_zip(mod_data.full_path()), 'r') as modArchive:
            biglist = list(filter(lambda s: s.find("/") != -1, modArchive.namelist()))
            for file in by_mod[mod]:
                filtered = list(filter(lambda s: s.lower() == file,biglist))
                #print(filtered[0])
                modArchive.extract(filtered[0])
                #print(mod)
                #print(conf)
                current = ""
                with open(filtered[0],"r",encoding="ISO-8859-1") as f:
                    current = f.read()
                if file in in_vanilla:
                    print(conflicts[conf])
                    print("++ "+file)
                    with open(outpath+"/vanilla/"+conf,"w",encoding="utf-8") as f:
                        f.write((current.replace("\r\n","\n")).replace("\n","\r\n")+"\n")

        os.chdir(outpath)
    os.chdir(current_path)

def clean_file_contents(file_content):
    info = list(filter(lambda x: len(x) > 0, list(map(lambda s: s.partition('#')[0].strip(), file_content.split("\n")))))

    info.insert(0," ")
    info.append(" ")


    return "\n".join(info)


def line_diff(text1,text2):
    diff_match = dmp.diff_match_patch()
    diff_struct = diff_match.diff_linesToChars(text1, text2)

    lineText1 = diff_struct[0] # .chars1
    lineText2 = diff_struct[1] # .chars2
    lineArray = diff_struct[2] # .lineArray

    diffs = diff_match.diff_main( lineText1, lineText2, False )
    diff_match.diff_charsToLines( diffs, lineArray )
    diff_match.diff_cleanupSemantic( diffs )
    #print(diff_match.diff_prettyText(diffs))
    return diffs

def dif_auto_once(base_file,mod_files,verbose):
    orig = base_file
    diffs = []
    differ = dmp.diff_match_patch()
    for file in mod_files:
        if len(file) < 3:
            pass
        bob = line_diff(orig,file)
        diffs.append(bob)
        patches = differ.patch_make(bob)
    patches = list(map(lambda d: differ.patch_make(orig,d),diffs))
    patches.sort(key=lambda x: len(x),reverse=False)
    new_text = orig
    no_good = False
    for patch in patches:
        tmp_text, results = differ.patch_apply(patch,new_text)
        for i in results:
            if not i:
                no_good = True
        if no_good:
            break
        new_text = tmp_text
    if no_good:
        if verbose:
            print("This file will need manual merging")      
    else:
        return new_text
    return None

def dif_auto(conflicts,in_vanilla,mods,modpath,outpath,verbose=False):
    shutil.rmtree(outpath+"/merged_patch/",ignore_errors=True)
    Path(outpath+"/merged_patch/").mkdir(mode=0o0777)
    bad = []
    good = 0
    total = 0

    if verbose:
        print("\n--- Starting Automerge Process ---\n")

    for conf in in_vanilla:
        folders = list(map(lambda s: "".join(list(filter(lambda c: c.isalnum(), s))),conflicts[conf]))
        folders.insert(0,"vanilla")
        file_contents = []

        for folder in folders:
            file_path = Path(outpath+"/"+folder+"/"+conf).parent
            file = Path(conf).name
            filtered = list(filter(lambda s: s.lower() == file.lower(),os.listdir(file_path)))
            try:
                file = filtered[0]
            except: 
                print(file_path)
                print(file)
                print(os.listdir(file_path))
                quit()
            with open(str(file_path)+"/"+file,"r",encoding="ISO-8859-1") as f:
                file_contents.append(clean_file_contents(f.read().replace("\r\n","\n")))
        orig = file_contents[0]
        diffs = []
        differ = dmp.diff_match_patch()

        for file in file_contents[1:]:
            if len(file) < 3:
                pass
            bob = line_diff(orig,file)
            diffs.append(bob)
            patches = differ.patch_make(bob)
        patches = list(map(lambda d: differ.patch_make(orig,d),diffs))
        patches.sort(key=lambda x: len(x),reverse=False)

        new_text = orig
        no_good = False

        for patch in patches:
            tmp_text, results = differ.patch_apply(patch,new_text)
            for i in results:
                if not i:
                    no_good = True
            if no_good:
                bad.append(conf)
                break
            new_text = tmp_text

        if no_good:
            if verbose:
                print("This file will need manual merging: "+conf) 
            total += 1      
        else:
            total += 1
            good += 1
            print(outpath+"/merged_patch/"+conf)
            new_path = Path(outpath+"/merged_patch/"+conf).parent
            if not os.path.exists(new_path):
                new_path.mkdir(parents=True,exist_ok=True,mode=0o0777)
            with open(outpath+"/merged_patch/"+conf,"w",encoding="ISO-8859-1") as f:
                f.write((new_text.replace("\r\n","\n")).replace("\n","\r\n")+"\n")
    return bad

def extract_all_zips(base_dir,mods,folder):
    based_dir = os.path.join(base_dir,"mod")
    for mod in mods:
        data_path = os.path.join(base_dir,mod.datapath)
        mod_path = list(filter(lambda s: os.path.join("mod",s.lower()) == mod.datapath.lower(),os.listdir(based_dir)))
        with ZipFile(os.path.join(base_dir,"mod",mod_path[0])) as z:
            z.extractall(folder)


def read_configs(config_file):
    configs = {}
    try:
        configs = toml.load(config_file)
        #print(configs)
    except FileNotFoundError:
        print("Oh no")
    except toml.TomlDecodeError:
        print("Something is wrong with the config file")
    
    if len(configs) < 1:
        exit("Couldn't load configs")
    return configs

def convert_all(folder):
    file_dirs = []
    for root, dirs, files in os.walk(folder):
        path = root.split(os.sep)
        for file in files:
                if file.endswith(".txt"):
                    file_dirs.append(root+"/"+file)

    for file in file_dirs:
        content = str()
        try:
            with open(file,"r",encoding="utf-8") as f:
                content = f.read()
        except:
            with open(file,"r",encoding="ISO-8859-1") as f:
                content = f.read()
        #print(file)
        try:
            yeah = content.encode("cp1252")
            with open(file,"w", encoding="cp1252") as f:
                f.write(content)
        except:
            print(file)

def parse_arguments():
    parser = argparse.ArgumentParser(description='Merge Paradox mod conflicts.')
    parser.add_argument('-c','--config',help='configuration file to load, defaults to current directory')
    parser.add_argument('-x','--extract',help='extract all non-conflicting files to a folder',action='store_true')
    parser.add_argument('-d','--dry-run',help='list file conflicts without merging',action='store_true')
    parser.add_argument('-v','--verbose',help='print information about processed mods',action='store_true')
    parser.add_argument('game_id',help='game in the config to use')
    parser.add_argument('patch_name',help='name of the generated mod')

    args = parser.parse_args()
    if args.config == None:
        args.config = "./merger.toml"
    return args

def main():
    args = parse_arguments()
    configs = read_configs(args.config)
    mods = generate_mod_data(configs[args.game_id]["modpath"])

    conflicts = generate_conflicts(mods,configs[args.game_id]["valid_paths"])
    
    flat_name = args.patch_name.lower()
    flat_name = re.sub(r'[^a-z0-9 ]+','',flat_name)
    flat_name = re.sub(r' ','_',flat_name)

    in_vanilla = conflicts_against_base(configs[args.game_id]["datapath"],conflicts,configs[args.game_id]["valid_paths"])

    by_mod = OrderedDict()

    if args.verbose:
        print("--- Mod Conflicts in Vanilla Files ---\n")
    for conf in in_vanilla:
        if args.verbose:
            print(conf)
        for mod in conflicts[conf]:
            if args.verbose:
                print("-- "+mod)
            if mod not in by_mod:
                by_mod[mod] = list()
            by_mod[mod].append(conf)

    #for conf in conflicts:
    #    print(conf)
    #    print(conflicts[conf])

    generate_mod(conflicts,mods,args.patch_name,flat_name+".zip",flat_name+".mod",args.extract)
    if args.dry_run:
        exit()

    if args.extract:
        extract_all_zips(configs[args.game_id]["modpath"],mods,"./"+flat_name+"_tmp_zip")
    extract_all(conflicts,in_vanilla,mods,configs[args.game_id]["modpath"],"./"+flat_name+"_tmp")
    
    bad = dif_auto(conflicts,in_vanilla,mods,configs[args.game_id]["modpath"],"./"+flat_name+"_tmp",args.verbose)
    
    if len(bad) > 0:
        print("\n List of Bad mods:")
        for conf in bad:
            print(conf)
            print(conflicts[conf],end="\n\n")
        print("{:.2f}% of merges completed automatically.".format(100.0-100.0*len(bad)/len(conflicts)))
    else:
        print("All merges completed automatically.")
    

if __name__ == "__main__":
    main()