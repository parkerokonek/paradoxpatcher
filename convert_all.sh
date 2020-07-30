#!/bin/bash

tmp=$(mktemp /tmp/convert_case.XXXX)

for f in $(find "$1" -maxdepth 2)
do
	#echo "$f"
	file_info=$(file -i "$f")
	good="0"
	if [[ "$f" == *".txt"* ]]; then
		good="1"
	fi
	if [[ "$f" == *".csv"* ]]; then
		good="1"
	fi

	if [[ good == "0" ]]; then
		continue
	fi

	if [[ "$file_info" == *"charset=utf-8"* ]]; then
		echo "Found UTF-8: $f"
		iconv -f UTF-8 -t WINDOWS-1252 "$f" > "$tmp"
		if [ $? -eq 0 ]; then
			mv "$tmp" "$f"
		fi
	fi
done
