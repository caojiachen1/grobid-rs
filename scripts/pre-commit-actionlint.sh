#!/bin/bash
set -e

# grobid-rs pre-commit hook for linting GitHub Actions workflow files
# This script validates GitHub Actions workflow files against schema

# ANSI color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔍 Checking GitHub Actions workflow files...${NC}"

# Check if workflow files exist
WORKFLOW_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.github/workflows/.*\.ya?ml$' || true)

if [ -z "$WORKFLOW_FILES" ]; then
    echo -e "${GREEN}✓ No GitHub Actions workflow files to check${NC}"
    exit 0
fi

# Check if actionlint is installed
if ! command -v actionlint &> /dev/null; then
    echo -e "${YELLOW}⚠️ actionlint not found. Installing...${NC}"
    
    # Platform-specific installation
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        if command -v brew &> /dev/null; then
            brew install actionlint
        else
            echo -e "${YELLOW}Homebrew not found. Using go install...${NC}"
            if command -v go &> /dev/null; then
                go install github.com/rhysd/actionlint/cmd/actionlint@latest
            else
                echo -e "${RED}❌ Neither brew nor go found. Please install actionlint manually:${NC}"
                echo -e "${YELLOW}https://github.com/rhysd/actionlint#installation${NC}"
                exit 1
            fi
        fi
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux
        if command -v go &> /dev/null; then
            go install github.com/rhysd/actionlint/cmd/actionlint@latest
        else
            echo -e "${RED}❌ Go not found. Please install actionlint manually:${NC}"
            echo -e "${YELLOW}https://github.com/rhysd/actionlint#installation${NC}"
            exit 1
        fi
    elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        # Windows
        echo -e "${RED}❌ Automatic installation on Windows not supported.${NC}"
        echo -e "${YELLOW}Please install actionlint manually:${NC}"
        echo -e "${YELLOW}https://github.com/rhysd/actionlint#installation${NC}"
        exit 1
    else
        echo -e "${RED}❌ Unsupported platform. Please install actionlint manually:${NC}"
        echo -e "${YELLOW}https://github.com/rhysd/actionlint#installation${NC}"
        exit 1
    fi
fi

# Ensure PATH includes Go binaries
if [[ -d "$HOME/go/bin" ]]; then
    export PATH="$PATH:$HOME/go/bin"
fi

# Check if actionlint is now available
if ! command -v actionlint &> /dev/null; then
    echo -e "${RED}❌ Failed to install or locate actionlint. Please install it manually:${NC}"
    echo -e "${YELLOW}https://github.com/rhysd/actionlint#installation${NC}"
    exit 1
fi

echo -e "${BLUE}Linting ${#WORKFLOW_FILES[@]} GitHub Actions workflow files...${NC}"

# Check each workflow file
ERRORS_FOUND=0
for FILE in $WORKFLOW_FILES; do
    echo -e "${BLUE}Checking ${FILE}...${NC}"
    if ! actionlint "$FILE"; then
        echo -e "${RED}❌ Errors found in ${FILE}${NC}"
        ERRORS_FOUND=1
    else
        echo -e "${GREEN}✓ ${FILE} is valid${NC}"
    fi
done

if [ $ERRORS_FOUND -eq 0 ]; then
    echo -e "${GREEN}✅ All GitHub Actions workflow files are valid!${NC}"
    exit 0
else
    echo -e "${RED}❌ Errors found in GitHub Actions workflow files. Please fix them before committing.${NC}"
    exit 1
fi