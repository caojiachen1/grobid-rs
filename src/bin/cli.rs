use clap::Parser;
use std::path::PathBuf;

/// A CLI tool to interact with Grobid.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the PDF file to process.
    #[clap(short, long, value_parser)]
    pdf_file: PathBuf,

    /// Path to the Grobid base directory (containing grobid/ and runtime/).
    #[clap(short, long, value_parser, default_value_os_t = PathBuf::from(env!("GROBID_RS_ASSETS_PATH")))]
    grobid_base: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("Initializing Grobid from: {}", args.grobid_base.display());
    grobid_rs::init(&args.grobid_base).map_err(|e| anyhow::anyhow!("Grobid initialization failed: {}. Assets path was: {}", e, args.grobid_base.display()))?;

    println!("Processing header for: {}", args.pdf_file.display());
    match grobid_rs::process_header(&args.pdf_file) {
        Ok(header_xml) => {
            println!("\n--- Processed Header ---");
            println!("{}", header_xml);
        }
        Err(e) => {
            eprintln!("Error processing header: {}", e);
            return Err(anyhow::anyhow!("Header processing failed"));
        }
    }

    Ok(())
} 