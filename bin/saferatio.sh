#!/bin/bash
echo > saferatio.csv
mkdir -p ../safe ../unsafe
folder=$(pwd)
if [ "$1" != "" ]; then
	folder=$(pwd)/$1
fi
find $folder -name "Cargo.toml" | while read f; do
   echo ===== $f ===== | tee >> saferatio.csv
   cargo geiger --output-format=Ratio --manifest-path $f >> saferatio.csv
done
mv ../safe .
mv ../unsafe .
grep "=====" saferatio.csv | wc
tokei -e safe -e unsafe -t=Rust
tokei safe
tokei unsafe
tar cvfj $ROOT/../$(basename $(pwd)).tar.bz2 safe unsafe saferatio.csv

