#!/bin/bash
set -e

# grobid-rs pre-commit hook
# This script runs rustfmt on staged Rust files before each commit
# and ensures code follows formatting standards

# ANSI color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}📝 Running pre-commit checks for grobid-rs...${NC}"

# Ensure rustfmt is installed
if ! command -v rustfmt &> /dev/null; then
    echo -e "${YELLOW}⚠️  rustfmt not found. Installing...${NC}"
    rustup component add rustfmt
fi

# Ensure clippy is installed
if ! command -v cargo-clippy &> /dev/null; then
    echo -e "${YELLOW}⚠️  clippy not found. Installing...${NC}"
    rustup component add clippy
fi

# Get staged Rust files
STAGED_RS_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.rs$' || true)

if [ -z "$STAGED_RS_FILES" ]; then
    echo -e "${GREEN}✓ No Rust files to check${NC}"
    exit 0
fi

echo -e "${BLUE}🔍 Checking ${#STAGED_RS_FILES[@]} Rust files...${NC}"

# Format staged files with rustfmt
echo -e "${BLUE}Running rustfmt...${NC}"
if ! cargo fmt -- --check; then
    echo -e "${RED}❌ rustfmt check failed!${NC}"
    echo -e "${YELLOW}Running auto-format and re-staging files...${NC}"
    
    # Format the files
    cargo fmt
    
    # Re-stage the formatted files
    for FILE in $STAGED_RS_FILES; do
        if [ -f "$FILE" ]; then
            git add "$FILE"
            echo -e "${GREEN}✓ Formatted and re-staged: $FILE${NC}"
        fi
    done
    
    echo -e "${GREEN}✓ Code formatting fixed and staged${NC}"
else
    echo -e "${GREEN}✓ Formatting looks good!${NC}"
fi

# Run clippy on the codebase
echo -e "${BLUE}Running clippy...${NC}"
if ! cargo clippy -- -D warnings; then
    echo -e "${RED}❌ clippy found issues that need to be fixed!${NC}"
    echo -e "${YELLOW}Please fix the issues before committing.${NC}"
    echo -e "${YELLOW}You can run 'cargo clippy --fix' to automatically fix some issues.${NC}"
    exit 1
else
    echo -e "${GREEN}✓ No clippy warnings!${NC}"
fi

echo -e "${GREEN}✅ All checks passed!${NC}"
exit 0