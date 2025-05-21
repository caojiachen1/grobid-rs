# grobid-rs Examples

This directory contains example applications that demonstrate how to use the grobid-rs library for various document processing tasks.

## Available Examples

1. **Basic Extraction** (`basic_extraction.rs`)
   - Demonstrates the core functionality of extracting header information, full text, and references from a PDF document
   - Shows the basic initialization and API usage pattern

2. **Batch Processing** (`batch_processing.rs`)
   - Shows how to process multiple PDF documents in parallel
   - Demonstrates using rayon for parallel processing with a configurable thread count
   - Includes progress reporting and error handling

3. **Custom Configuration** (`custom_configuration.rs`)
   - Demonstrates how to customize Grobid processing with different configuration options
   - Shows examples of citation consolidation, TEI coordinates, and page range processing

4. **Progress Reporting** (`progress_reporting.rs`)
   - Illustrates how to implement progress feedback during document processing
   - Shows how to implement cancellation support for long-running operations
   - Includes a simple progress bar implementation

## Running the Examples

Each example can be run with Cargo, providing the required arguments:

```bash
# For basic extraction
cargo run --example basic_extraction /path/to/grobid/home /path/to/document.pdf

# For batch processing
cargo run --example batch_processing /path/to/grobid/home /path/to/input/directory /path/to/output/directory [num_threads]

# For custom configuration
cargo run --example custom_configuration /path/to/grobid/home /path/to/document.pdf

# For progress reporting
cargo run --example progress_reporting /path/to/grobid/home /path/to/document.pdf
```

## Requirements

- A compiled version of grobid-rs
- A properly set up Grobid home directory with models and resources
- PDF documents for testing

## Learning Path

If you're new to grobid-rs, we recommend going through the examples in this order:

1. Start with `basic_extraction.rs` to understand the core functionality
2. Explore `custom_configuration.rs` to learn about configuring the processing
3. Try `progress_reporting.rs` to see how to implement feedback for users
4. Finally, check out `batch_processing.rs` for production-style document processing

## Creating Your Own Examples

When creating your own applications with grobid-rs, you can use these examples as starting points. The key patterns to follow are:

1. Initialize Grobid with a valid path to the resources
2. Create appropriate error handling for all operations
3. Consider using configuration options for specific needs
4. Implement progress reporting for better user experience in long-running tasks
5. Use parallel processing when handling multiple documents

## Additional Resources

For more detailed information, refer to the documentation in the `docs/` directory of the grobid-rs project.