#!/usr/bin/env bash

# ignore `cd`
cd recusive-zipping

# Recursively zip all files in each folder, naming zip after folder
find . -type d | while read dir; do
    # Skip current directory
    [ "$dir" = "." ] && continue

    # Get folder name
    foldername=$(basename "$dir")

    # Create zip with folder name inside parent directory
    (cd "$dir" && zip -r "../${foldername}.zip" .)
done
