# Parker's Paradox Patcher
## Installation
Requires Python 3.8 and the following packages for Python/pip
* toml
* pyside2

## Usage
```
usage: paradox_merger.py [-h] [-c CONFIG] [-x] [-d] [-v] game_id patch_name

Merge Paradox mod conflicts.

positional arguments:
  game_id               game in the config to use
  patch_name            name of the generated mod

optional arguments:
  -h, --help            show this help message and exit
  -c CONFIG, --config CONFIG
                        configuration file to load, defaults to current directory
  -x, --extract         extract all non-conflicting files to a folder
  -d, --dry-run         list file conflicts without merging
  -v, --verbose         print information about processed mods
```

An example merging mod conflicts for CK2 using the supplied config file.
```bash
cd PATH/TO/MERGER
python3 ./paradox_merger.py CK2 "Merged Patch"
```
