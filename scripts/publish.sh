#!/usr/bin/env bash
#
# Publish skia-rs crates to crates.io
#
# This script publishes crates in dependency order with:
# - Version checking to skip already-published crates
# - Exponential backoff for rate limiting
# - Automatic retry on failure
#
# Usage:
#   ./scripts/publish.sh              # Dry run (shows what would be published)
#   ./scripts/publish.sh --execute    # Actually publish to crates.io
#   ./scripts/publish.sh --check      # Only check versions, don't publish
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
MAX_RETRIES=5
INITIAL_BACKOFF=10      # Initial wait time in seconds
MAX_BACKOFF=300         # Maximum wait time (5 minutes)
BACKOFF_MULTIPLIER=2    # Exponential backoff multiplier
INDEX_WAIT=45           # Time to wait for crates.io to index a new crate

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
    "skia-rs-bench"
)

# Parse arguments
MODE="dry-run"
if [[ "${1:-}" == "--execute" ]]; then
    MODE="execute"
elif [[ "${1:-}" == "--check" ]]; then
    MODE="check"
fi

# Get the workspace root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# =============================================================================
# Utility Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}ℹ${NC} $*"
}

log_success() {
    echo -e "${GREEN}✓${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $*"
}

log_error() {
    echo -e "${RED}✗${NC} $*"
}

log_step() {
    echo -e "${CYAN}→${NC} $*"
}

# =============================================================================
# Version Functions
# =============================================================================

# Get local version from Cargo.toml
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

# Get published version from crates.io with retry
get_published_version() {
    local crate_name="$1"
    local retries=3
    local backoff=2

    for ((i=1; i<=retries; i++)); do
        local response
        response=$(curl -s --max-time 10 -H "User-Agent: skia-rs-publish-script/1.0" \
            "https://crates.io/api/v1/crates/$crate_name" 2>/dev/null || echo '{"errors":[]}')

        # Check for rate limiting
        if echo "$response" | grep -q '"Too Many Requests"'; then
            if [[ $i -lt $retries ]]; then
                sleep $backoff
                backoff=$((backoff * 2))
                continue
            fi
            echo ""
            return 1
        fi

        if echo "$response" | grep -q '"errors"'; then
            echo ""  # Not published or error
            return 0
        else
            # Extract the newest version
            echo "$response" | grep -oP '"newest_version"\s*:\s*"\K[^"]+' | head -1 || echo ""
            return 0
        fi
    done

    echo ""
}

# Check if version exists on crates.io
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

# =============================================================================
# Publishing Functions
# =============================================================================

# Calculate backoff time with exponential increase
calculate_backoff() {
    local attempt="$1"
    local backoff=$INITIAL_BACKOFF

    for ((i=1; i<attempt; i++)); do
        backoff=$((backoff * BACKOFF_MULTIPLIER))
        if [[ $backoff -gt $MAX_BACKOFF ]]; then
            backoff=$MAX_BACKOFF
            break
        fi
    done

    echo $backoff
}

# Publish a single crate with retry and exponential backoff
publish_crate() {
    local crate_name="$1"
    local crate_dir="$WORKSPACE_ROOT/crates/$crate_name"
    local local_ver
    local_ver=$(get_local_version "$crate_name")

    for ((attempt=1; attempt<=MAX_RETRIES; attempt++)); do
        log_step "Publishing $crate_name@$local_ver (attempt $attempt/$MAX_RETRIES)..."

        # Try to publish
        local output
        local exit_code=0
        output=$(cd "$crate_dir" && cargo publish --no-verify 2>&1) || exit_code=$?

        if [[ $exit_code -eq 0 ]]; then
            log_success "Published $crate_name@$local_ver"
            return 0
        fi

        # Check for specific errors
        if echo "$output" | grep -q "already uploaded"; then
            log_warning "$crate_name@$local_ver is already published (race condition)"
            return 0
        fi

        if echo "$output" | grep -q "rate limit"; then
            local backoff
            backoff=$(calculate_backoff "$attempt")
            log_warning "Rate limited. Waiting ${backoff}s before retry..."
            sleep "$backoff"
            continue
        fi

        if echo "$output" | grep -q "timed out"; then
            local backoff
            backoff=$(calculate_backoff "$attempt")
            log_warning "Request timed out. Waiting ${backoff}s before retry..."
            sleep "$backoff"
            continue
        fi

        if echo "$output" | grep -q "503\|502\|500"; then
            local backoff
            backoff=$(calculate_backoff "$attempt")
            log_warning "Server error. Waiting ${backoff}s before retry..."
            sleep "$backoff"
            continue
        fi

        # Check if it's a dependency issue (crate not yet indexed)
        if echo "$output" | grep -q "failed to select a version"; then
            local backoff
            backoff=$(calculate_backoff "$attempt")
            log_warning "Dependency not yet indexed. Waiting ${backoff}s before retry..."
            sleep "$backoff"
            continue
        fi

        # If we're not on the last attempt, apply backoff
        if [[ $attempt -lt $MAX_RETRIES ]]; then
            local backoff
            backoff=$(calculate_backoff "$attempt")
            log_warning "Publish failed. Waiting ${backoff}s before retry..."
            echo "$output" | tail -5
            sleep "$backoff"
        else
            log_error "Failed to publish $crate_name after $MAX_RETRIES attempts"
            echo "$output"
            return 1
        fi
    done

    return 1
}

# Wait for crate to be indexed on crates.io
wait_for_index() {
    local crate_name="$1"
    local expected_version="$2"
    local max_wait=300  # 5 minutes max
    local waited=0
    local check_interval=15

    log_step "Waiting for crates.io to index $crate_name@$expected_version..."

    while [[ $waited -lt $max_wait ]]; do
        sleep $check_interval
        waited=$((waited + check_interval))

        local published_ver
        published_ver=$(get_published_version "$crate_name")

        if [[ "$published_ver" == "$expected_version" ]]; then
            log_success "$crate_name@$expected_version is now indexed"
            return 0
        fi

        echo -e "  ${YELLOW}...still waiting (${waited}s elapsed, latest: ${published_ver:-none})${NC}"
    done

    log_warning "$crate_name may not be indexed yet, continuing anyway..."
    return 0
}

# =============================================================================
# Main Script
# =============================================================================

echo ""
echo -e "${MAGENTA}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${MAGENTA}║           skia-rs Crates.io Publishing Script                  ║${NC}"
echo -e "${MAGENTA}╠════════════════════════════════════════════════════════════════╣${NC}"
echo -e "${MAGENTA}║  Features:                                                     ║${NC}"
echo -e "${MAGENTA}║  • Version checking (skip already-published)                   ║${NC}"
echo -e "${MAGENTA}║  • Exponential backoff for rate limiting                       ║${NC}"
echo -e "${MAGENTA}║  • Automatic retry on failure (${MAX_RETRIES} attempts)                     ║${NC}"
echo -e "${MAGENTA}║  • Waits for crates.io indexing between publishes              ║${NC}"
echo -e "${MAGENTA}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

case "$MODE" in
    "dry-run")
        echo -e "${YELLOW}🔍 DRY RUN MODE${NC} - No crates will be published"
        echo -e "   Run with ${CYAN}--execute${NC} to actually publish"
        echo -e "   Run with ${CYAN}--check${NC} to only check versions"
        ;;
    "check")
        echo -e "${BLUE}📋 CHECK MODE${NC} - Only checking versions"
        ;;
    "execute")
        echo -e "${RED}⚠️  EXECUTE MODE${NC} - Crates will be published to crates.io"
        echo ""
        echo -e "${YELLOW}Press Enter to continue or Ctrl+C to abort...${NC}"
        read -r
        ;;
esac
echo ""

# Check for cargo login
if [[ "$MODE" == "execute" ]]; then
    if ! cargo login --help &>/dev/null; then
        log_error "cargo is not available"
        exit 1
    fi
fi

# First pass: check all versions
log_info "Checking crate versions against crates.io..."
echo ""

declare -A CRATE_STATUS
declare -A CRATE_LOCAL_VER
declare -A CRATE_PUBLISHED_VER

PUBLISHED=0
SKIPPED=0
TO_PUBLISH=0
ERRORS=0

for crate in "${CRATES[@]}"; do
    local_ver=$(get_local_version "$crate")
    CRATE_LOCAL_VER[$crate]="$local_ver"

    # Small delay to avoid rate limiting during version checks
    sleep 0.5

    published_ver=$(get_published_version "$crate")
    CRATE_PUBLISHED_VER[$crate]="$published_ver"

    if [[ -z "$local_ver" ]]; then
        log_error "$crate - Could not read local version"
        CRATE_STATUS[$crate]="error"
        ((ERRORS++)) || true
        continue
    fi

    if version_exists "$local_ver" "$published_ver"; then
        echo -e "  ${GREEN}✓${NC} $crate@$local_ver - ${GREEN}Already published${NC}"
        CRATE_STATUS[$crate]="published"
        ((SKIPPED++)) || true
    else
        if [[ -z "$published_ver" ]]; then
            echo -e "  ${YELLOW}○${NC} $crate@$local_ver - ${YELLOW}New crate${NC}"
        else
            echo -e "  ${YELLOW}○${NC} $crate@$local_ver - ${YELLOW}Update (current: $published_ver)${NC}"
        fi
        CRATE_STATUS[$crate]="needs_publish"
        ((TO_PUBLISH++)) || true
    fi
done

echo ""
echo -e "${BLUE}┌─────────────────────────────────────┐${NC}"
echo -e "${BLUE}│          Version Summary            │${NC}"
echo -e "${BLUE}├─────────────────────────────────────┤${NC}"
echo -e "${BLUE}│${NC}  ${GREEN}Already published:${NC} $SKIPPED              ${BLUE}│${NC}"
echo -e "${BLUE}│${NC}  ${YELLOW}To publish:${NC} $TO_PUBLISH                      ${BLUE}│${NC}"
echo -e "${BLUE}│${NC}  ${RED}Errors:${NC} $ERRORS                          ${BLUE}│${NC}"
echo -e "${BLUE}└─────────────────────────────────────┘${NC}"
echo ""

if [[ $ERRORS -gt 0 ]]; then
    log_error "There were errors reading some crate versions"
    exit 1
fi

if [[ $TO_PUBLISH -eq 0 ]]; then
    log_success "All crates are already published at the current version!"
    exit 0
fi

# Handle check mode
if [[ "$MODE" == "check" ]]; then
    log_info "Check complete. Run with --execute to publish."
    exit 0
fi

# Handle dry run
if [[ "$MODE" == "dry-run" ]]; then
    echo -e "${YELLOW}Would publish the following crates in order:${NC}"
    echo ""
    idx=1
    for crate in "${CRATES[@]}"; do
        if [[ "${CRATE_STATUS[$crate]:-}" == "needs_publish" ]]; then
            local_ver="${CRATE_LOCAL_VER[$crate]}"
            echo -e "  ${CYAN}$idx.${NC} $crate@$local_ver"
            ((idx++)) || true
        fi
    done
    echo ""
    echo -e "${YELLOW}Run with --execute to publish these crates${NC}"
    exit 0
fi

# =============================================================================
# Execute Publishing
# =============================================================================

echo -e "${BLUE}Starting publish process...${NC}"
echo ""

PUBLISHED=0
FAILED=0
START_TIME=$(date +%s)

for crate in "${CRATES[@]}"; do
    if [[ "${CRATE_STATUS[$crate]:-}" != "needs_publish" ]]; then
        continue
    fi

    local_ver="${CRATE_LOCAL_VER[$crate]}"

    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}Publishing: $crate@$local_ver${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    if publish_crate "$crate"; then
        ((PUBLISHED++)) || true

        # Wait for crates.io to index before publishing dependent crates
        # This is important to avoid "failed to select a version" errors
        wait_for_index "$crate" "$local_ver"
    else
        ((FAILED++)) || true
        log_error "Failed to publish $crate"

        echo ""
        echo -e "${RED}Continue with remaining crates? [y/N]${NC}"
        read -r response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            log_error "Aborting publish process."
            break
        fi
    fi
done

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo ""
echo -e "${MAGENTA}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${MAGENTA}║                    Publishing Complete                         ║${NC}"
echo -e "${MAGENTA}╠════════════════════════════════════════════════════════════════╣${NC}"
echo -e "${MAGENTA}║${NC}  ${GREEN}Published:${NC} $PUBLISHED                                              ${MAGENTA}║${NC}"
echo -e "${MAGENTA}║${NC}  ${YELLOW}Skipped:${NC} $SKIPPED                                                ${MAGENTA}║${NC}"
echo -e "${MAGENTA}║${NC}  ${RED}Failed:${NC} $FAILED                                                 ${MAGENTA}║${NC}"
echo -e "${MAGENTA}║${NC}  ${BLUE}Duration:${NC} ${DURATION}s                                              ${MAGENTA}║${NC}"
echo -e "${MAGENTA}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

if [[ $FAILED -gt 0 ]]; then
    log_error "Some crates failed to publish. Check the output above for details."
    exit 1
fi

if [[ $PUBLISHED -gt 0 ]]; then
    log_success "All crates published successfully!"
    echo ""
    echo -e "${CYAN}View your crates at:${NC}"
    for crate in "${CRATES[@]}"; do
        if [[ "${CRATE_STATUS[$crate]:-}" == "needs_publish" ]]; then
            echo -e "  ${BLUE}https://crates.io/crates/$crate${NC}"
        fi
    done
fi

exit 0
