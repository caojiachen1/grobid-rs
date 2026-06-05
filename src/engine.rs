use crate::{GrobidError, JniHandle};
use jni::objects::*;
use std::path::{Path, PathBuf};
use tracing::{debug, info, trace};

/// Helper function to execute a function with a JNI environment
pub(crate) fn with_env<F, R>(f: F) -> Result<R, GrobidError>
where
    F: FnOnce(&mut JniHandle) -> Result<R, GrobidError>,
{
    trace!("Entering JNI environment context");

    // Create a new JNI handle that will automatically clean up on drop
    let mut handle = JniHandle::attach()?;

    // Call the function with the handle
    let result = f(&mut handle);

    // Handle any exceptions that may have occurred
    handle.handle_exception()?;

    trace!("Exiting JNI environment context");

    // Return the result
    result
}

/// Call a Grobid engine method with a file input parameter
pub(crate) fn _call_engine_process_method_with_file_input(
    handle: &mut JniHandle,
    method_name: &str,
    pdf_path: &Path,
) -> Result<String, GrobidError> {
    let file_cls = handle.find_class("java/io/File")?;
    let j_path_str = handle.new_string(pdf_path.to_string_lossy())?;
    let j_file_obj = handle.new_object(
        file_cls,
        "(Ljava/lang/String;)V",
        &[JValue::from(&j_path_str)],
    )?;

    let engine = handle.engine()?;
    let j_result_string_obj = handle
        .call_method(
            engine,
            method_name,
            "(Ljava/io/File;)Ljava/lang/String;",
            &[JValue::from(&j_file_obj)],
        )?
        .l()
        .map_err(GrobidError::from)?;

    // Convert to Rust String internally
    let result_string: String = handle
        .get_string(&JString::from(j_result_string_obj))?
        .into();
    Ok(result_string)
}

/// Extract the full text of a document as TEI XML
pub(crate) fn call_engine_fulltext_to_tei(
    handle: &mut JniHandle,
    pdf_path: &Path,
) -> Result<String, GrobidError> {
    trace!("Creating Java File object for: {}", pdf_path.display());
    let file_cls = handle.find_class("java/io/File")?;
    let j_path_str = handle.new_string(pdf_path.to_string_lossy())?;
    let j_file_obj = handle.new_object(
        file_cls,
        "(Ljava/lang/String;)V",
        &[JValue::from(&j_path_str)],
    )?;

    trace!("Getting GrobidAnalysisConfig default instance");
    let cfg_cls = handle.find_class("org/grobid/core/engines/config/GrobidAnalysisConfig")?;
    let cfg_obj = handle
        .call_static_method(
            cfg_cls,
            "defaultInstance",
            "()Lorg/grobid/core/engines/config/GrobidAnalysisConfig;",
            &[],
        )?
        .l()?;

    debug!("Calling Grobid fullTextToTEI method");
    let engine = handle.engine()?;
    let j_tei_string_obj = handle.call_method(
        engine,
        "fullTextToTEI",
        "(Ljava/io/File;Lorg/grobid/core/engines/config/GrobidAnalysisConfig;)Ljava/lang/String;",
        &[JValue::from(&j_file_obj), JValue::from(&cfg_obj)],
    )?.l().map_err(GrobidError::from)?;

    // Convert to Rust String internally
    trace!("Converting Java String result to Rust String");
    let tei_string: String = handle.get_string(&JString::from(j_tei_string_obj))?.into();
    debug!(
        "Fulltext TEI extraction successful, TEI length: {} bytes",
        tei_string.len()
    );
    Ok(tei_string)
}

/// Extract header metadata from a document
pub(crate) fn call_engine_process_header(
    handle: &mut JniHandle,
    pdf_path: &Path,
) -> Result<String, GrobidError> {
    trace!("Creating Java String for PDF path");
    // Convert Rust Path to Java String
    let j_path_str = handle.new_string(pdf_path.to_string_lossy())?;

    trace!("Creating BiblioItem object");
    // Instantiate a new BiblioItem
    let biblio_cls = handle.find_class("org/grobid/core/data/BiblioItem")?;
    let biblio_obj = handle.new_object(biblio_cls, "()V", &[])?;

    debug!("Calling Grobid processHeader method");
    // Call Engine.processHeader(String, BiblioItem)
    let engine = handle.engine()?;
    let j_result_obj = handle
        .call_method(
            engine,
            "processHeader",
            "(Ljava/lang/String;Lorg/grobid/core/data/BiblioItem;)Ljava/lang/String;",
            &[JValue::from(&j_path_str), JValue::from(&biblio_obj)],
        )?
        .l()
        .map_err(GrobidError::from)?;

    // Convert Java String to Rust String
    trace!("Converting Java String result to Rust String");
    let result_string: String = handle.get_string(&JString::from(j_result_obj))?.into();
    debug!(
        "Header processing successful, TEI length: {} bytes",
        result_string.len()
    );
    Ok(result_string)
}

/// Extract bibliographic references from a document
pub(crate) fn call_engine_process_references(
    handle: &mut JniHandle,
    pdf_path: &Path,
) -> Result<String, GrobidError> {
    // Convert Rust Path to Java File object
    let file_cls = handle.find_class("java/io/File")?;
    let j_path_str = handle.new_string(pdf_path.to_string_lossy())?;
    let j_file = handle.new_object(
        file_cls,
        "(Ljava/lang/String;)V",
        &[JValue::from(&j_path_str)],
    )?;

    // Call Engine.processReferences(File, int)
    // The int parameter is the consolidation option (0 = no consolidation)
    let engine = handle.engine()?;
    let j_bib_list = handle
        .call_method(
            engine,
            "processReferences",
            "(Ljava/io/File;I)Ljava/util/List;",
            &[JValue::from(&j_file), JValue::from(0)],
        )?
        .l()
        .map_err(GrobidError::from)?;

    // Call the static method references2TEI to convert BibDataSet list to TEI String
    let empty_path = handle.new_string("")?;

    let j_result_obj = handle
        .call_static_method(
            "org/grobid/core/engines/Engine",
            "references2TEI",
            "(Ljava/lang/String;Ljava/util/List;)Ljava/lang/String;",
            &[JValue::from(&empty_path), JValue::from(&j_bib_list)],
        )?
        .l()
        .map_err(GrobidError::from)?;

    // Convert Java String to Rust String
    let result_string: String = handle.get_string(&JString::from(j_result_obj))?.into();
    Ok(result_string)
}

/// Run pdfalto and return the path to the generated ALTO XML.
pub fn run_pdfalto(pdf: &Path, grobid_home: &Path) -> Result<PathBuf, GrobidError> {
    let bin_name = match std::env::consts::OS {
        "windows" => "pdfalto.exe",
        _ => "pdfalto",
    };
    let platform_name = match std::env::consts::OS {
        "windows" => "win-64",
        "macos" => {
            if cfg!(target_arch = "aarch64") {
                "mac_arm-64"
            } else {
                "mac-64"
            }
        }
        _ => "lin-64",
    };

    // Grobid 0.9.1+ places pdfalto in a nested pdfalto/ subdirectory
    let bin = grobid_home
        .join("pdfalto")
        .join(platform_name)
        .join("pdfalto")
        .join(bin_name);

    if !bin.exists() {
        return Err(GrobidError::PdfAlto(format!(
            "pdfalto binary not found at {}",
            bin.display()
        )));
    }
    let out_xml = pdf.with_extension("alto.xml");
    let status = std::process::Command::new(&bin)
        .arg("--inputFile")
        .arg(pdf)
        .arg("--outputFile")
        .arg(&out_xml)
        .status()
        .map_err(|e| GrobidError::PdfAlto(format!("pdfalto call failed: {}", e)))?;
    if !status.success() {
        return Err(GrobidError::PdfAlto(format!(
            "pdfalto failed with status {:?}",
            status.code()
        )));
    }
    Ok(out_xml)
}

// ---------------- public API ------------------

/// Process a PDF document and extract its full text content as TEI XML
pub fn fulltext_to_tei(pdf: &Path) -> Result<String, GrobidError> {
    info!("Processing full text from PDF: {}", pdf.display());
    with_env(|handle| call_engine_fulltext_to_tei(handle, pdf))
}

/// Process a PDF document and extract its header metadata (title, authors, etc.)
pub fn process_header(pdf: &Path) -> Result<String, GrobidError> {
    info!("Processing header from PDF: {}", pdf.display());
    with_env(|handle| call_engine_process_header(handle, pdf))
}

/// Process a PDF document and extract its bibliographic references
pub fn process_references(pdf: &Path) -> Result<String, GrobidError> {
    info!("Extracting references from PDF: {}", pdf.display());
    with_env(|handle| call_engine_process_references(handle, pdf))
}
