# Git Hooks for grobid-rs

This document explains how to set up git hooks to automatically run `rustfmt` and other checks before commits, ensuring consistent code quality across all contributions.

## Ready-to-Use Hooks

The grobid-rs repository provides pre-configured Git hooks in the `scripts/` directory:

- `pre-commit.sh` - Automatically formats Rust code and runs clippy checks before each commit
- `install-hooks.sh` - Installs the hooks into your local repository

### Quick Setup (Recommended)

Run the provided installation script from the repository root:

```bash
# From the root of the grobid-rs repository
./scripts/install-hooks.sh
```

This will automatically install all available hooks and make them executable.

## Why Use Git Hooks?

Git hooks help automate quality assurance by running checks before git actions like commits or pushes. For Rust projects, they're especially useful for:

- Ensuring code follows the project's formatting standards
- Running clippy to catch common mistakes
- Checking that tests pass before code is committed
- Validating commit messages follow conventional format

## Setting Up Pre-commit Hooks for rustfmt (with Auto-formatting)

### Option 1: Manual Setup (Auto-fixing)

1. Use the pre-configured hook from the repository (recommended):

```bash
# Copy the repository's pre-commit hook
cp scripts/pre-commit.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

The included hook will automatically format your code with `rustfmt` and check for issues with `clippy` before each commit.

Or create your own simple hook:

```bash
#!/bin/sh
# Run rustfmt on all Rust files that are staged for commit
STAGED_RS_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$')

if [ -n "$STAGED_RS_FILES" ]; then
  echo "Running rustfmt on staged Rust files..."
  # Auto-format the files (without --check flag)
  cargo fmt -- $STAGED_RS_FILES
  # Re-stage the formatted files
  git add $STAGED_RS_FILES
  echo "✅ Code formatted and re-staged!"
fi

# Proceed with the commit
exit 0
```

2. Make the hook executable:

```bash
chmod +x .git/hooks/pre-commit
```

### Option 2: Using pre-commit Framework

1. Install the pre-commit framework:

```bash
# macOS
brew install pre-commit

# Linux
pip install pre-commit

# Windows
pip install pre-commit
```

2. Create a `.pre-commit-config.yaml` file in your repository root:

```yaml
repos:
-   repo: https://github.com/doublify/pre-commit-rust
    rev: v1.0
    hooks:
    -   id: fmt  # Auto-formats code without requiring --check
    -   id: cargo-check
    -   id: clippy
```

3. Install the hooks:

```bash
pre-commit install
```

### Option 3: Using cargo-husky

1. Add cargo-husky to your project:

```bash
cargo add --dev cargo-husky
```

2. Configure it in your Cargo.toml:

```toml
[dev-dependencies]
cargo-husky = { version = "1", features = ["precommit-hook", "run-cargo-fmt", "run-cargo-clippy"] }
```

With this configuration, cargo-husky will automatically format your code before each commit.

## Skipping Hooks When Necessary

Sometimes you may need to bypass hooks for a specific commit:

```bash
git commit --no-verify -m "Your commit message"
```

However, this should be used sparingly, as it circumvents the quality checks.

## Additional Useful Hooks

### Commit Message Validation

To enforce conventional commit messages:

1. Install commitlint:

```bash
npm install -g @commitlint/cli @commitlint/config-conventional
```

2. Create a `.git/hooks/commit-msg` file:

```bash
#!/bin/sh
npx --no -- commitlint --edit "$1"
```

3. Make it executable:

```bash
chmod +x .git/hooks/commit-msg
```

### Pre-push Hook for Tests

Create a `.git/hooks/pre-push` file to ensure all tests pass before pushing:

```bash
#!/bin/sh
echo "Running tests before push..."
cargo test || {
  echo "Error: Tests failed. Please fix failing tests before pushing."
  exit 1
}
exit 0
```

Make it executable:

```bash
chmod +x .git/hooks/pre-push
```

## Team Standardization

For team projects, consider:

1. Using the provided `install-hooks.sh` script to standardize hook setup
2. Committing the `.pre-commit-config.yaml` file to the repository
3. Adding documentation about hook setup in your README or CONTRIBUTING guide
4. Including hook setup in your onboarding process for new contributors
5. Using CI to verify that code meets the same standards enforced by hooks

## Troubleshooting

If you encounter issues with hooks:

1. Ensure you have the latest rustfmt installed: `rustup component add rustfmt`
2. Check that the hook files have execute permissions: `chmod +x .git/hooks/pre-commit`
3. Verify the path to executables in your hooks are correct for your system
4. On Windows, ensure scripts use the correct shebang and line endings
5. If auto-formatting isn't working, make sure you're not using the `--check` flag with `cargo fmt`
6. Try reinstalling the hooks using the `scripts/install-hooks.sh` script
7. Check the output of `git hook run pre-commit` for debugging information

## Alternative: Check-Only Pre-commit Hook

If you prefer to manually fix formatting issues rather than having them auto-fixed, you can use this check-only hook instead:

```bash
#!/bin/sh
# Run rustfmt check on all Rust files that are staged for commit
STAGED_RS_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$')

if [ -n "$STAGED_RS_FILES" ]; then
  echo "Checking rustfmt on staged Rust files..."
  cargo fmt -- --check $STAGED_RS_FILES || {
    echo "Error: rustfmt check failed. Please format your code with 'cargo fmt' before committing."
    exit 1
  }
fi

exit 0
```