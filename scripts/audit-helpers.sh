#!/bin/bash
# Helper functions for crate auditing

# Find all TODO/FIXME comments
find_todos() {
    local crate=$1
    echo "=== TODOs in $crate ==="
    rg "TODO|FIXME|todo!|unimplemented!" "crates/$crate/src/" -n --color=never
}

# Find placeholder returns
find_placeholders() {
    local crate=$1
    echo "=== Potential placeholders in $crate ==="
    # Look for suspicious patterns
    rg "return None|return false|return 0\.0|return Vec::new\(\)|Some\(self\.clone\(\)\)" \
       "crates/$crate/src/" -n --color=never -A 2 -B 2
}

# List all public APIs
list_public_apis() {
    local crate=$1
    echo "=== Public APIs in $crate ==="
    rg "pub (fn|struct|enum|trait)" "crates/$crate/src/" -n --color=never
}

# Count functions in a file
count_functions() {
    local file=$1
    rg "^\s*(pub\s+)?fn\s+" "$file" -c
}
