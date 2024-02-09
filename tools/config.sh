#!/bin/bash

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT license.
# SPDX-License-Identifier: MIT

# Exits immediately on failure.
set -eu

# Function to display usage information
usage() {
    echo "Usage: $0 <-t|--template-file TEMPLATE_FILE> <-o|--output-file OUTPUT_FILE> <-p|--param PARAM_NAME="PARAM_VALUE"> [-p|--param PARAM_NAME="PARAM_VALUE"]..."
    echo "Example:"
    echo "  $0 -t /path/template.yaml -o /path/output.yaml -p param1=\"value1\" -p param2=\"value2\"..."
}

# Create param list
param_list=()

# Parse command line arguments
while [[ $# -gt 0 ]]
do
    key="$1"

    case $key in
        -t|--template-file)
            t_file="$2"
            shift # past argument
            shift # past value
            ;;
        -o|--output-file)
            out_file="$2"
            shift # past argument
            shift # past value
            ;;
        -p|--param)
            param_list+=("$2")
            shift # past argument
            shift # past value
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $key"
            usage
            exit 1
    esac
done

# Check if all required arguments have been set
if [[ -z "${t_file}" || -z "${out_file}" || ${#param_list[@]} -eq 0 ]]; then
    echo "Error: Missing required arguments:"
    [[ -z "${t_file}" ]] && echo "  -t|--template-file"
    [[ -z "${out_file}" ]] && echo "  -o|--output-file"
    [[ ${#param_list[@]} -eq 0 ]] && echo "  -p|--param"
    echo -e "\n"
    usage
    exit 1
fi

cat $t_file > $out_file

# Iterates through passed in parameters and sets them in the new file
for i in "${param_list[@]}"
do
    # Split entry into name and value
    IFS='=' read name value <<< "${i}"

    # Escape certain characters so that sed can properly replace the values
    esc_name=$(echo "$name" | sed 's,\/,\\/,g')
    esc_value=$(echo "$value" | sed 's,\/,\\/,g')

    # Replace parameter placeholder in template
    sed -i -e 's/# '"$esc_name"': <<value>>'/"$esc_name"': "'"$esc_value"'"/g' $out_file
done
