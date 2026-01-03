#!/usr/bin/env bash
#
# Publish skia-rs crates to crates.io
#
# This script publishes crates in dependency order, checking if each
# crate version is already published before attempting to publish.
#
# Usage:
#   ./scripts/publish.sh              # Dry run (shows what would be published)
#   ./scripts/publish.sh --execute    # Actually publish to crates.io
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Crates in dependency order (must publish in this order)
CRATES=(
    "skia-rs-core"
    "skia-rs-path"
    "skia-rs-paint"
    "skia-rs-text"
    "skia-rs-codec"
    "skia-rs-canvas"
    "skia-rs-gpu"
    "skia-rs-svg"
    "skia-rs-pdf"
    "skia-rs-skottie"
    "skia-rs-ffi"
    "skia-rs-safe"
)

# Parse arguments
DRY_RUN=true
if [[ "${1:-}" == "--execute" ]]; then
    DRY_RUN=false
fi

# Get the workspace root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║          skia-rs Crates.io Publishing Script               ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

if $DRY_RUN; then
    echo -e "${YELLOW}🔍 DRY RUN MODE - No crates will be published${NC}"
    echo -e "${YELLOW}   Run with --execute to actually publish${NC}"
else
    echo -e "${RED}⚠️  EXECUTE MODE - Crates will be published to crates.io${NC}"
fi
echo ""

# Function to get local version from Cargo.toml
get_local_version() {
    local crate_name="$1"
    local crate_dir="$WORKSPACE_ROOT/crates/$crate_name"

    if [[ -f "$crate_dir/Cargo.toml" ]]; then
        # Check if version is workspace-inherited
        if grep -q 'version.workspace = true' "$crate_dir/Cargo.toml"; then
            # Get version from workspace Cargo.toml
            grep '^version = ' "$WORKSPACE_ROOT/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/'
        else
            # Get version directly from crate Cargo.toml
            grep '^version = ' "$crate_dir/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/'
        fi
    else
        echo ""
    fi
}

# Function to get published version from crates.io
get_published_version() {
    local crate_name="$1"

    # Query crates.io API with timeout
    local response
    response=$(curl -s --max-time 10 -H "User-Agent: skia-rs-publish-script" \
        "https://crates.io/api/v1/crates/$crate_name" 2>/dev/null || echo '{"errors":[]}')

    if echo "$response" | grep -q '"errors"'; then
        echo ""  # Not published or error
    else
        # Extract the newest version
        echo "$response" | grep -oP '"newest_version"\s*:\s*"\K[^"]+' | head -1 || echo ""
    fi
}

# Function to compare versions
version_exists() {
    local local_ver="$1"
    local published_ver="$2"

    if [[ -z "$published_ver" ]]; then
        return 1  # Not published
    fi

    if [[ "$local_ver" == "$published_ver" ]]; then
        return 0  # Same version exists
    fi

    return 1  # Different version
}

# Counters
PUBLISHED=0
SKIPPED=0
FAILED=0
TO_PUBLISH=0

echo -e "${BLUE}Checking crate versions...${NC}"
echo ""

# First pass: check all versions
declare -A CRATE_STATUS
for crate in "${CRATES[@]}"; do
    local_ver=$(get_local_version "$crate")
    published_ver=$(get_published_version "$crate")

    if [[ -z "$local_ver" ]]; then
        echo -e "  ${RED}✗${NC} $crate - ${RED}Could not read local version${NC}"
        CRATE_STATUS[$crate]="error"
        continue
    fi

    if version_exists "$local_ver" "$published_ver"; then
        echo -e "  ${GREEN}✓${NC} $crate@$local_ver - ${GREEN}Already published${NC}"
        CRATE_STATUS[$crate]="published"
        ((SKIPPED++)) || true
    else
        if [[ -z "$published_ver" ]]; then
            echo -e "  ${YELLOW}○${NC} $crate@$local_ver - ${YELLOW}New crate (not on crates.io)${NC}"
        else
            echo -e "  ${YELLOW}○${NC} $crate@$local_ver - ${YELLOW}New version (published: $published_ver)${NC}"
        fi
        CRATE_STATUS[$crate]="needs_publish"
        ((TO_PUBLISH++)) || true
    fi
done

echo ""
echo -e "${BLUE}Summary:${NC}"
echo -e "  ${GREEN}Already published:${NC} $SKIPPED"
echo -e "  ${YELLOW}To publish:${NC} $TO_PUBLISH"
echo ""

if [[ $TO_PUBLISH -eq 0 ]]; then
    echo -e "${GREEN}✅ All crates are already published at the current version!${NC}"
    exit 0
fi

if $DRY_RUN; then
    echo -e "${YELLOW}Would publish the following crates:${NC}"
    for crate in "${CRATES[@]}"; do
        if [[ "${CRATE_STATUS[$crate]:-}" == "needs_publish" ]]; then
            local_ver=$(get_local_version "$crate")
            echo "  - $crate@$local_ver"
        fi
    done
    echo ""
    echo -e "${YELLOW}Run with --execute to publish these crates${NC}"
    exit 0
fi

# Second pass: publish crates that need it
echo -e "${BLUE}Publishing crates...${NC}"
echo ""

for crate in "${CRATES[@]}"; do
    if [[ "${CRATE_STATUS[$crate]:-}" != "needs_publish" ]]; then
        continue
    fi

    local_ver=$(get_local_version "$crate")
    crate_dir="$WORKSPACE_ROOT/crates/$crate"

    echo -e "${BLUE}Publishing $crate@$local_ver...${NC}"

    if (cd "$crate_dir" && cargo publish --no-verify); then
        echo -e "  ${GREEN}✓ Published $crate@$local_ver${NC}"
        ((PUBLISHED++)) || true

        # Wait for crates.io to index the new crate before publishing dependents
        echo -e "  ${YELLOW}Waiting 30s for crates.io to index...${NC}"
        sleep 30
    else
        echo -e "  ${RED}✗ Failed to publish $crate${NC}"
        ((FAILED++)) || true

        # Ask if we should continue
        echo -e "${RED}Continue with remaining crates? [y/N]${NC}"
        read -r response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            echo -e "${RED}Aborting.${NC}"
            exit 1
        fi
    fi
done

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Publishing Complete${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "  ${GREEN}Published:${NC} $PUBLISHED"
echo -e "  ${YELLOW}Skipped:${NC} $SKIPPED"
echo -e "  ${RED}Failed:${NC} $FAILED"

if [[ $FAILED -gt 0 ]]; then
    exit 1
fi

echo ""
echo -e "${GREEN}✅ All crates published successfully!${NC}"
