#!/bin/bash
set -e

# ANSI color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔄 Installing Git hooks for grobid-rs...${NC}"

# Check if we're in the right directory (repo root)
if [ ! -d ".git" ] && [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ Error: This script must be run from the repository root directory.${NC}"
    echo -e "${YELLOW}Please run: ./scripts/install-hooks.sh from the repository root.${NC}"
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p .git/hooks

# Copy the individual hooks to hooks directory
echo -e "${BLUE}Installing component hooks...${NC}"
cp scripts/pre-commit.sh .git/hooks/pre-commit.sh
cp scripts/pre-commit-actionlint.sh .git/hooks/pre-commit-actionlint.sh

# Install the pre-commit runner as the main pre-commit hook
echo -e "${BLUE}Installing pre-commit runner...${NC}"
cp scripts/pre-commit-runner.sh .git/hooks/pre-commit

# Make hooks executable
chmod +x .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit.sh
chmod +x .git/hooks/pre-commit-actionlint.sh

# Verify installation
if [ -x .git/hooks/pre-commit ] && [ -x .git/hooks/pre-commit.sh ] && [ -x .git/hooks/pre-commit-actionlint.sh ]; then
    echo -e "${GREEN}✅ Git hooks installed successfully!${NC}"
    echo -e "${BLUE}The following hooks are now active:${NC}"
    echo -e "  ${GREEN}• pre-commit${NC} - Runs all hooks in sequence"
    echo -e "    ${GREEN}↳ Rust formatting${NC} - Formats Rust code and runs clippy"
    echo -e "    ${GREEN}↳ Actions linting${NC} - Validates GitHub Actions workflow YAML files"
    echo -e "${YELLOW}Note: You can bypass hooks with git commit --no-verify if needed${NC}"
else
    echo -e "${RED}❌ Error: Failed to install git hooks.${NC}"
    echo -e "${YELLOW}Please check permissions and try again.${NC}"
    exit 1
fi

# Update documentation
echo -e "${BLUE}To learn more about available hooks, see:${NC}"
echo -e "${BLUE}docs/GIT_HOOKS.md${NC}"

echo -e "${GREEN}Happy coding!${NC}"
exit 0