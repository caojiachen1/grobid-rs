#!/bin/bash
set -e

# grobid-rs pre-commit runner
# This script runs all pre-commit hooks in sequence

# ANSI color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔄 Running grobid-rs pre-commit hooks...${NC}"

# Get the directory of the hooks
HOOKS_DIR=$(dirname "$(realpath "$0")")

# Run Rust linting & formatting hook
if [ -x "${HOOKS_DIR}/pre-commit.sh" ]; then
    echo -e "${BLUE}📝 Running Rust linting & formatting...${NC}"
    "${HOOKS_DIR}/pre-commit.sh" || {
        echo -e "${RED}❌ Rust pre-commit hook failed${NC}"
        exit 1
    }
    echo
else
    echo -e "${YELLOW}⚠️ Rust pre-commit hook not found or not executable${NC}"
fi

# Run GitHub Actions workflow validation
if [ -x "${HOOKS_DIR}/pre-commit-actionlint.sh" ]; then
    echo -e "${BLUE}🔍 Running GitHub Actions workflow validation...${NC}"
    "${HOOKS_DIR}/pre-commit-actionlint.sh" || {
        echo -e "${RED}❌ GitHub Actions workflow validation failed${NC}"
        exit 1
    }
    echo
else
    echo -e "${YELLOW}⚠️ GitHub Actions workflow validation hook not found or not executable${NC}"
fi

echo -e "${GREEN}✅ All pre-commit hooks passed!${NC}"
exit 0