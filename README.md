# Parker's Paradox Patcher

![GUI Build](https://github.com/parkerokonek/paradoxpatcher/workflows/BuildGUI/badge.svg)
![CLI Build](https://github.com/parkerokonek/paradoxpatcher/workflows/BuildCLI/badge.svg)
![Tests](https://github.com/parkerokonek/paradoxpatcher/workflows/Tests/badge.svg)

This project is mostly aimed at generating large Crusader Kings 2 modpacks for private use. The patcher reads the settings.txt file of the game and uses those mods as the list to determine mod conflicts and what mod content to package. By default, the patcher will only generate a mod that contains the files which conflicted and an accompanying .mod description file that lists its dependencies to ensure load order. 

These patches seem to work for most mods, but run into issues specifically with overhaul mods, or other mods which conflict in the same files as their specified replacement paths.

Using the extract flag is recommended, as it ensures that every person using the modpack has identical data and only needs to enable a single mod.
## Building
The current state of the project is experimental at best.
Requires rust's cargo/rustup to compile.

Install cargo and clone the repository.
### CLI Interface
```
cd /path/to/repository
cargo build --release --features command-line --bin merger-cli
```
### GUI Interface
Requires the GTK-3 dev libraries for your OS or distribution.

On Arch:

```
# pacman -S gtk3
```
On Ubuntu:

```
# apt install libgtk-3-dev
```
On Windows:

Install the gtk3 libraries using [MSYS2](https://www.msys2.org/)

After installing the dependencies run cargo using the following flags.

and then build using:
```
cargo build --release --features gui-interface --bin merger-gui
```

## Usage
### CLI Usage
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
paradoxmerger "Merged Patch" CK2
```
## Appendix
[Diff Match Patch](https://github.com/google/diff-match-patch): library used for diffing mod files and patching them together

[paradox-tools](https://github.com/taw/paradox-tools): a set of Paradox modding utilities written in Ruby 

[cwtools](https://github.com/tboby/cwtools): a library for manipulating Paradox scripts written in F#

[Pdoxcl2Sharp](https://github.com/nickbabcock/Pdoxcl2Sharp): a C# library for parsing and manipulating Paradox scripts
