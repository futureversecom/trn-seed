#!/bin/sh
set -e

display_usage() {
    cat << EOF
Usage:
  tools.sh rpc [option] <commit-hash/tag/branch-name>

Description:
	Compares the current branch and a hash/tag/branch to find differences in their RPC implementation

Options:
    --ci  		Copies rpc results to the external '/output' folder
    --help    Display this help message

Example:
	tools.sh rpc 0.0.30
EOF

		exit 0
}

while [ $# -gt 0 ]; do
    case "$1" in
        --ci) 	ci="0";;
        --help) display_usage;;
        *)    	input="$1";;
    esac
    shift
done

find_rpc_endpoints () {
    output_path=$1
    find_pattern_range () {
        start_line=$1
        file=$2

        find_matching_closing_line () {
            # find the matching closing line
            depth=0
            linesToSkip=$start_line;
            end_line=$start_line;
            while read -r line; do
                if ! [ "$linesToSkip" -eq 1 ]; then
                    linesToSkip=$((linesToSkip-1))
                    continue
                fi

                depth=$((depth + $(echo "$line" | grep -o "{" | wc -l)))
                depth=$((depth - $(echo "$line" | grep -o "}" | wc -l)))
                if [ "$depth" -eq 0 ]; then
                    break
                fi
                end_line=$((end_line+1))
            done < "$file"
            echo "$end_line"
        }

        # find the matching closing line
        local end_line=$(find_matching_closing_line "$start_line" "$file")

        # output the range of lines between the start and end line
        sed -n "${start_line},${end_line}p" "$file" | tee -a "$output_path"
    }

    # use find to search for all files and folders in the current directory
    search_result=$(find . -name "*.rs" -type f -exec grep -Hn "$keyword" {} \; | awk -F ':' '{print $1 " line " $2}')

    # loop over the search result and output each file path and line number
    while read -r result; do
        line_number="$(echo "$result" | cut -d ' ' -f3)"
        file_path="$(echo "$result" | cut -d ' ' -f1)"
        find_pattern_range "$line_number" "$file_path"
    done <<EOF
$search_result
EOF
}

keyword="sp_api::decl_runtime_apis!"
current_branch_name=$(git rev-parse --abbrev-ref HEAD)
output_path_1="./output/old_rpc.txt"
output_path_2="./output/new_rpc.txt"
diff_path="./output/rpc.diff"
mkdir -p output

if ! [ "$input" ]; then
    echo "Error. No commit hash, tag or branch name provided"
    exit 1
fi

rm $output_path_1 -f
rm $output_path_2 -f

# Temporary switch to a previous commit
git checkout "$input"
find_rpc_endpoints "$output_path_1"

# Go back to current branch
git checkout "$current_branch_name"
output_path="./scripts/misc/data/rpc.txt"
find_rpc_endpoints "$output_path_2"

# Generate diff
git diff --no-index "$output_path_1" "$output_path_2" > "$diff_path" || true

if [ "$ci" = "0" ]; then
    cp "$output_path_1" /output/
    cp "$output_path_2" /output/
    cp "$diff_path" /output/
fi
