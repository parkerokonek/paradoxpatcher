# Parker's Paradox Patcher

[![Action Status](https://github.com/parkerokonek/paradoxpatcher/workflows/Build/badge.svg)](https://github.com/parkerokonek/paradoxpatcher/actions)

This project is mostly aimed at generating large Crusader Kings 2 modpacks for private use. The patcher reads the settings.txt file of the game and uses those mods as the list to determine mod conflicts and what mod content to package. By default, the patcher will only generate a mod that contains the files which conflicted and an accompanying .mod description file that lists its dependencies to ensure load order. 

These patches seem to work for most mods, but run into issues specifically with overhaul mods, or other mods which conflict in the same files as their specified replacement paths.

Using the extract flag is recommended, as it ensures that every person using the modpack has identical data and only needs to enable a single mod.
## Building
Not recommended at the moment as the project has strange build requirements due to using Python 3's Diff Match and Patch implementation. The current state of the project is experimental at best.

Currently Requires Python 3.5 or higher, Rust (Stable or Nightly), and Google's Diff Match Patch for python3.
Then use cargo to compile.

## Usage
```
USAGE:
    paradoxmerger [FLAGS] [OPTIONS] <patch_name> [game_id]

FLAGS:
    -d, --dry-run    list file conflicts without merging
    -x, --extract    extract all non-conflicting files to a folder
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    print information about processed mods

OPTIONS:
    -c, --config <CONFIG_FILE>    configuration file to load, defaults to current directory

ARGS:
    <patch_name>    name of the generated mod
    <game_id>       game in the config to use
```

An example merging mod conflicts for CK2 using the supplied config file.
```bash
cd PATH/TO/MERGER
paradoxmerger CK2 "Merged Patch"
```
## Appendix
[Diff Match Patch](https://github.com/google/diff-match-patch): library used for diffing mod files and patching them together

[paradox-tools](https://github.com/taw/paradox-tools): a set of Paradox modding utilities written in Ruby 

[cwtools](https://github.com/tboby/cwtools): a library for manipulating Paradox scripts written in F#

[Pdoxcl2Sharp](https://github.com/nickbabcock/Pdoxcl2Sharp): a C# library for parsing and manipulating Paradox scripts
