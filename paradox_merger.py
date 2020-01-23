from zipfile import ZipFile
from collections import defaultdict
import toml
import re

class Tree(defaultdict):
    def __init__(self, value=None):
        super(Tree, self).__init__(Tree)
        self.value = value

class ModInfo:
    def __init__(self,modpath,datapath,name,files,overrides, depends):
        self.modpath = modpath
        self.overrides = overrides
        self.files = files
        self.datapath = datapath
        self.name = name
        self.depends = depends

def grep(file,reg):
    inputText = ""
    with open(file) as textFile:
        inputText = textFile.read()
    matches = re.findall(reg,inputText)

    return matches

def zip_list(zipPath):
    biglist = []
    valid_dirs = []
    with ZipFile(zipPath, 'r') as modArchive:
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
            if dirs[separated[0]][separated[1]].value == None:
                dirs[separated[0]][separated[1]].value = list()
                dirs[separated[0]][separated[1]].value.append([separated[2]])
            else:
                dirs[separated[0]][separated[1]].value.append([separated[2]])
    #print(dirs)

    return dirs

def dir_list(dirPath):
    return {}

def generate_mod_data(modPath):

    matches = grep(modPath+"/settings.txt","\"mod\/[^\"]*\"")
    mod_list = []
    
    for modmod in matches:
        zips = grep(modPath+"/"+modmod[1:-1],"\"[^\"]*\.zip")
        names = grep(modPath+"/"+modmod[1:-1],"name\s*=\s*\"[^\"]*\"")
        if len(zips) < 1 or len(names) < 1:
            print("Spaghetti-Os")
        else:
            archive = modPath+"/"+zips[0][1:]
            #print(archive)
            name = names[0].split("\"")[1]
            print(name)
            filetree = zip_list(archive)
            new_mod = ModInfo(modPath,zips[0][1:],name,filetree,[],[])
            mod_list.append(new_mod)
    return mod_list


def generate_conflicts(modPath):
    mods = generate_mod_data(modPath)
    print(len(mods))
    return []

def main():
    configs = {}
    try:
        configs = toml.load("./merger.toml")
        #print(configs)
    except FileNotFoundError:
        print("Oh no")
    except toml.TomlDecodeError:
        print("Something is wrong with the config file")
    
    if len(configs) < 1:
        exit("Couldn't load configs")
    
    conflicts = generate_conflicts(configs["CK2"]["modpath"])

main()