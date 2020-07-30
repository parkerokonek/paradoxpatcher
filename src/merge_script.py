import diff_match_patch as dmp
import shutil
from pathlib import Path
import re
import os

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
    else:
        return (True,new_text)
    return (False,"")