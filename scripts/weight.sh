#!/bin/bash

echo "pallet_name,function_name,weight" > weight.csv; \
find ./runtime/src/weights/* -type f -exec pcregrep -H -M  "Weight {\n(.* as Weight)" {} \;  | while read line; do
file_name=$(echo $line | awk -v FS="(runtime/src/weights/|.rs:)" '{print $2}')
function_name=$(echo $line | grep -o 'fn .*(')
function_weight=$(echo $line | grep -Eo '([[:digit:]]+_[[:digit:]]+_[[:digit:]]+|[0])')

if [[ $file_name ]]
then
data="${file_name}"
fi
if [[ $function_name ]]
then
data="${data},${function_name}"
fi

if [[ $function_weight ]]
then
data="${data}), ${function_weight}"
echo "$data" >> weight.csv
fi

done
