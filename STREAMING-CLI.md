# Streaming / Pipe-in Pipe-out CLI for Grobid-RS

## Motivation

Supporting UNIX-style streaming and pipe-based workflows enables Grobid-RS to fit seamlessly into data engineering, ETL, and batch processing pipelines. Users can process large numbers of PDFs without intermediate files, using standard UNIX tools and idioms.

## Technical Requirements
- Support reading PDF bytes from stdin
- Output results (JSON, TEI, etc.) to stdout
- Handle multiple PDFs in a single stream (optional, for advanced use)
- Async I/O for performance and responsiveness
- Robust error handling and clear exit codes

## Implementation Plan

### 1. CLI Design
- Add `--stdin` flag to accept PDF input from stdin
- Add `--stdout` flag to write output to stdout (default if not writing files)
- Support `--input-format` and `--output-format` flags for flexibility

Example usage:
```sh
cat paper.pdf | grobid-cli header --stdin --output-format json > out.json
cat *.pdf | grobid-cli header --stdin --output-format tei > all.tei
```

### 2. Framing Protocol
- For single-PDF: read all bytes from stdin until EOF
- For multi-PDF: use a framing protocol (e.g., length-prefixed, or MIME multipart)
- Document framing in CLI help and README

### 3. Async I/O Implementation
- Use `tokio` or `async-std` for async reading/writing
- Buffer input to avoid partial reads
- Stream output as soon as available

Example Rust snippet:
```rust
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut stdin = io::stdin();
    let mut buffer = Vec::new();
    stdin.read_to_end(&mut buffer).await?;
    // Process PDF bytes in buffer...
    io::stdout().write_all(b"{\"result\": ...}").await?;
    Ok(())
}
```

### 4. Error Handling
- Print errors to stderr with clear messages
- Exit with non-zero code on failure
- Optionally emit error JSON to stdout for batch jobs

### 5. Testing & Documentation
- Add integration tests for pipe mode
- Document usage in README and CLI help
- Provide example shell scripts for batch processing

## Quick Wins
- Single-PDF stdin/stdout support
- Documented framing protocol for multi-PDF (future work)

## References
- [tokio async I/O](https://docs.rs/tokio/latest/tokio/io/index.html)
- [UNIX pipes](https://en.wikipedia.org/wiki/Pipeline_(Unix)) 