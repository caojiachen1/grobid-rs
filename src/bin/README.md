# GROBID CLI Tool

This command-line interface provides access to GROBID functionality for extracting structured information from scholarly documents in PDF format.

## Features

- **Document Processing:** Extract headers, full text, or references from scholarly PDFs
- **Multiple Output Formats:** Get results in TEI XML, JSON, plain text, or BibTeX
- **Configurable:** Control JVM settings, memory allocation, and more

## Installation

The CLI is built when compiling the `grobid-rs` library with the `cli` feature enabled:

```sh
cargo build --release --features=cli
```

## Basic Usage

```sh
# Process a document header
grobid-cli header paper.pdf

# Extract references as BibTeX
grobid-cli references paper.pdf

# Process full text and save as JSON
grobid-cli fulltext paper.pdf --output-format=json -o paper.json
```

## Commands

### Header

Extract metadata from document header (title, authors, abstract, etc.)

```sh
grobid-cli header <pdf_file> [options]
```

Options:
- `--output-format=<format>` - Output format: `tei` (default), `json`, or `text`

### Fulltext

Process the entire document including structure, text, figures, and tables.

```sh
grobid-cli fulltext <pdf_file> [options]
```

Options:
- `--output-format=<format>` - Output format: `tei` (default), `json`, or `text`

### References

Extract and process bibliographic references from the document.

```sh
grobid-cli references <pdf_file> [options]
```

Options:
- `--output-format=<format>` - Output format: `bibtex` (default), `tei`, `json`, or `text`

## Global Options

These options apply to all commands:

- `-g, --grobid-base <path>` - Path to GROBID base directory
- `--max-memory <size>` - Maximum memory allocation for JVM (e.g., "2G")
- `--log-level <level>` - Log verbosity: `off`, `error`, `warn`, `info` (default), `debug`, or `trace`
- `-D, --property <key=value>` - Add system property (can be used multiple times)
- `-J, --jvm-option <option>` - Add JVM option (can be used multiple times)
- `-o, --output <file>` - Write output to a file instead of stdout
- `-h, --help` - Display help information
- `-V, --version` - Display version information

## Output Formats

- **TEI:** Native XML format from GROBID, containing all extracted information
- **JSON:** Structured data extracted from the TEI
- **Text:** Plain text extraction from structured elements
- **BibTeX:** Standard citation format (references command only)

## Examples

```sh
# Basic header extraction
grobid-cli header article.pdf

# Get bibliographic references in BibTeX format
grobid-cli references thesis.pdf --output-format=bibtex -o citations.bib

# Process full text with increased memory
grobid-cli --max-memory=2G fulltext large_document.pdf 

# Extract plain text from a document
grobid-cli fulltext paper.pdf --output-format=text -o paper.txt

# Using a custom GROBID installation
grobid-cli --grobid-base=/path/to/grobid header paper.pdf
```