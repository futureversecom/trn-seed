#!/bin/bash

echo "function_name,weight" > weight.csv; \
data='';
find ./runtime/src/weights/* -type f -exec pcregrep -M  "Weight {\n(.* as Weight)" {} \;  | while read line; do

function_name=$(echo $line | grep -o 'fn .*(')
function_weight=$(echo $line | grep -Eo '([[:digit:]]+_[[:digit:]]+_[[:digit:]]+|[0])')
if [[ $function_name ]]
then
data="${function_name}"
fi

if [[ $function_weight ]]
then
data="${data}), ${function_weight}"
echo "$data" >> weight.csv
fi

done

