use crate::build_modules::common::{
    bail, fs, header, io, print_cargo_info, print_cargo_warning, Client, Context, Duration, File,
    IntoParallelIterator, OpenOptions, ParallelIterator, Path, PathBuf, ProgressBar, ProgressState,
    ProgressStyle, Read, Result, Seek, SeekFrom, StatusCode, Write, ZipArchive,
    EXTRACTION_SUCCESS_MARKER_FILE, GROBID_DIR_NAME_PREFIX, GROBID_DOWNLOAD_URL_PREFIX,
    GROBID_SOURCE_SUBDIR_NAME, GROBID_VERSION, GROBID_ZIP_SHA256,
};
use crate::build_modules::utils::verify_sha256;
use anyhow::anyhow;
use bytes::Buf;
use std::sync::Mutex;

/// Optimized parallel download with range support and resume capability
fn parallel_download(url: &str, to: &Path, expect_sha256: &str) -> Result<()> {
    // If file exists & checksum matches → skip download
    if to.exists() && verify_sha256(to, expect_sha256).is_ok() {
        print_cargo_info(&format!(
            "ZIP already present and checksum OK ({})",
            to.display()
        ));
        return Ok(());
    }

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(900))
        .build()
        .context("building reqwest client")?;

    // HEAD to get size & range support
    let head_result = client.head(url).send();

    // Check if HEAD request was successful
    let (len, ranges_ok) = match head_result {
        Ok(head) => {
            if !head.status().is_success() {
                print_cargo_info(&format!(
                    "HEAD request to {} returned non-success status: {}. Using fallback download method.",
                    url, head.status()
                ));
                return download_file(url, to);
            }

            // Try to get content length
            let content_length = head
                .headers()
                .get(header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            if content_length.is_none() {
                print_cargo_info(&format!(
                    "Content-Length header missing from {}. Using fallback download method.",
                    url
                ));
                return download_file(url, to);
            }

            let ranges_supported = head.headers().get(header::ACCEPT_RANGES).is_some();
            (content_length.unwrap(), ranges_supported)
        }
        Err(e) => {
            print_cargo_info(&format!(
                "HEAD request to {} failed: {}. Using fallback download method.",
                url, e
            ));
            return download_file(url, to);
        }
    };

    print_cargo_info(&format!(
        "Parallel download: {} ({} MiB, ranges {})",
        url,
        len / 1_048_576,
        if ranges_ok { "YES" } else { "NO" }
    ));

    fs::create_dir_all(to.parent().unwrap())
        .with_context(|| format!("create parent dir for {}", to.display()))?;

    // Pre-allocate file once
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .read(true)
        .open(to)
        .with_context(|| format!("open {}", to.display()))?;
    file.set_len(len)?;

    // Progress bar
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template(
            "{msg} [{elapsed_precise}] {bytes}/{total_bytes} {bytes_per_sec} {wide_bar:.cyan/blue}",
        )
        .unwrap(),
    );
    pb.set_message("Downloading Grobid ZIP");

    if !ranges_ok || len < 50 * 1024 * 1024 {
        // fallback single stream
        let mut resp = client.get(url).send().context("stream GET")?;
        let mut writer = pb.wrap_write(file);
        std::io::copy(&mut resp, &mut writer)?;
    } else {
        // ranged – 8 × 8 MiB (or fewer)
        const CHUNK: u64 = 8 * 1024 * 1024;
        #[allow(clippy::manual_div_ceil)]
        let n_chunks = ((len + CHUNK - 1) / CHUNK) as usize;
        let file_mutex = Mutex::new(file);
        (0..n_chunks).into_par_iter().try_for_each(|i| {
            let start = i as u64 * CHUNK;
            let end = std::cmp::min(start + CHUNK - 1, len - 1);
            // skip if already on disk (resume)
            {
                let mut buf = [0u8; 1];
                let mut f = file_mutex.lock().unwrap();
                f.seek(SeekFrom::Start(end))?;
                if f.read(&mut buf).unwrap_or(0) == 1 {
                    pb.inc(CHUNK.min(len - start));
                    return Ok(());
                }
            }
            let resp = client
                .get(url)
                .header(header::RANGE, format!("bytes={}-{}", start, end))
                .send()?;
            if resp.status() != StatusCode::PARTIAL_CONTENT && resp.status() != StatusCode::OK {
                return Err(anyhow!(
                    "range GET {}-{} failed: {}",
                    start,
                    end,
                    resp.status()
                ));
            }
            let mut bytes = resp.bytes()?;
            let mut f = file_mutex.lock().unwrap();
            f.seek(SeekFrom::Start(start))?;
            while !bytes.is_empty() {
                let n = f.write(bytes.as_ref())?;
                bytes.advance(n);
            }
            pb.inc(end - start + 1);
            Ok(())
        })?;
    }
    pb.finish_with_message("downloaded");

    // Verify checksum
    if verify_sha256(to, expect_sha256).is_err() {
        bail!("SHA-256 mismatch after download");
    }
    Ok(())
}

/// Legacy download function for when parallel download cannot be used
#[allow(dead_code)]
fn download_file(url: &str, to: &Path) -> Result<()> {
    print_cargo_warning(&format!("Downloading {} to {}", url, to.display()));
    fs::create_dir_all(to.parent().unwrap()).with_context(|| {
        format!(
            "Failed to create parent dir for download: {}",
            to.parent().unwrap().display()
        )
    })?;

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(1800)) // 30 minutes overall timeout for large files
        .build()
        .context("Failed to build reqwest client")?;

    let response = client
        .get(url)
        .send()
        .with_context(|| format!("Failed to GET {}", url))?;

    if !response.status().is_success() {
        bail!(
            "Failed to download {}: status {} - {}",
            url,
            response.status(),
            response.text().unwrap_or_default()
        );
    }

    let total_size = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len_str| ct_len_str.parse::<u64>().ok())
        .unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
        )
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:?}", state.eta()).unwrap())
        .progress_chars("==>")
    );
    let download_file_name = Path::new(url)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    pb.set_message(format!("Downloading {}", download_file_name));

    let mut dest_file = fs::File::create(to)
        .with_context(|| format!("Failed to create destination file: {}", to.display()))?;

    let mut source = pb.wrap_read(response);

    io::copy(&mut source, &mut dest_file)
        .with_context(|| format!("Failed to write download to: {}", to.display()))?;

    pb.finish_with_message(format!(
        "Successfully downloaded {} to {}",
        download_file_name,
        to.display()
    ));

    Ok(())
}

fn extract_zip(zip_path: &Path, target_dir: &Path) -> Result<()> {
    print_cargo_info(&format!(
        "Extracting ZIP {} to {}",
        zip_path.display(),
        target_dir.display()
    ));
    if !target_dir.exists() {
        fs::create_dir_all(target_dir).with_context(|| {
            format!(
                "Failed to create target extraction dir: {}",
                target_dir.display()
            )
        })?;
    }

    // Open zip file directly instead of loading it all into memory
    let file = File::open(zip_path)
        .with_context(|| format!("Failed to open zip file: {}", zip_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("Failed to parse zip archive: {}", zip_path.display()))?;

    let pb_extract = ProgressBar::new(archive.len() as u64);
    pb_extract.set_style(
        ProgressStyle::with_template(
            "{msg} [{elapsed_precise}] [{wide_bar:.green/yellow}] {pos}/{len} files ",
        )
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("==>"),
    );
    pb_extract.set_message(format!(
        "Extracting {}",
        zip_path.file_name().unwrap_or_default().to_string_lossy()
    ));

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).with_context(|| {
            format!("Error reading entry {} from zip: {}", i, zip_path.display())
        })?;
        let outpath = match entry.enclosed_name() {
            Some(path) => target_dir.join(path),
            None => {
                print_cargo_info(&format!(
                    "Skipping entry with invalid path in zip: {}",
                    entry.name()
                ));
                continue;
            }
        };

        if (*entry.name()).ends_with('/') {
            fs::create_dir_all(&outpath).with_context(|| {
                format!(
                    "Failed to create directory from zip entry: {}",
                    outpath.display()
                )
            })?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).with_context(|| {
                        format!(
                            "Failed to create parent directory for zip entry: {}",
                            p.display()
                        )
                    })?;
                }
            }
            let mut outfile = fs::File::create(&outpath).with_context(|| {
                format!(
                    "Failed to create file from zip entry: {}",
                    outpath.display()
                )
            })?;
            io::copy(&mut entry, &mut outfile).with_context(|| {
                format!("Failed to copy zip entry data to: {}", outpath.display())
            })?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                if mode & 0o111 != 0 {
                    // Check if executable bit is set in zip
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).with_context(
                        || {
                            format!(
                                "Failed to set executable permissions on: {}",
                                outpath.display()
                            )
                        },
                    )?;
                }
            }
        }
        pb_extract.inc(1);
    }
    pb_extract.finish_with_message(format!(
        "Successfully extracted {} to {}",
        zip_path.file_name().unwrap_or_default().to_string_lossy(),
        target_dir.display()
    ));
    Ok(())
}

pub fn ensure_grobid_source_extracted(assets_dir: &Path) -> Result<PathBuf> {
    let grobid_source_checkout_dir_name = format!(
        "{}{}/{}",
        GROBID_DIR_NAME_PREFIX, GROBID_VERSION, GROBID_SOURCE_SUBDIR_NAME
    );
    let grobid_source_checkout_dir = assets_dir.join(grobid_source_checkout_dir_name);
    let success_marker = grobid_source_checkout_dir.join(EXTRACTION_SUCCESS_MARKER_FILE);

    // If we've already successfully extracted, skip download/extract
    if success_marker.exists() {
        print_cargo_info(&format!(
            "Found existing extracted Grobid source at {} (marker present).",
            grobid_source_checkout_dir.display()
        ));
        // Determine actual root: prefer nested directory if present, else use checkout dir
        let root = if grobid_source_checkout_dir
            .join(format!("grobid-{}", GROBID_VERSION))
            .exists()
        {
            grobid_source_checkout_dir
                .join(format!("grobid-{}", GROBID_VERSION))
                .clone()
        } else {
            grobid_source_checkout_dir.clone()
        };
        return Ok(root);
    }

    print_cargo_info(&format!(
        "Grobid source not found or extraction incomplete at {}. Will download and extract.",
        grobid_source_checkout_dir.display()
    ));

    if !grobid_source_checkout_dir.exists() {
        fs::create_dir_all(&grobid_source_checkout_dir).with_context(|| {
            format!(
                "Failed to create Grobid source directory: {}",
                grobid_source_checkout_dir.display()
            )
        })?;
    }

    // Download the GROBID source archive
    let zip_file_name = format!("{}.zip", GROBID_VERSION);
    let grobid_zip_path = assets_dir.join(&zip_file_name);
    let download_url = format!("{}{}.zip", GROBID_DOWNLOAD_URL_PREFIX, GROBID_VERSION);

    print_cargo_info(&format!(
        "[Debug source_ops] Checking for ZIP: {}. Exists? {}",
        grobid_zip_path.display(),
        grobid_zip_path.exists()
    ));

    // Use optimized parallel download which handles checksums and resuming
    parallel_download(&download_url, &grobid_zip_path, GROBID_ZIP_SHA256)?;
    extract_zip(&grobid_zip_path, &grobid_source_checkout_dir)?;

    // Create success marker file
    fs::File::create(&success_marker).with_context(|| {
        format!(
            "Failed to create success marker file: {}",
            success_marker.display()
        )
    })?;
    // Determine root: nested or checkout dir
    let expected_project_root = format!("grobid-{}", GROBID_VERSION);
    let root = if grobid_source_checkout_dir
        .join(&expected_project_root)
        .exists()
    {
        grobid_source_checkout_dir
            .join(expected_project_root)
            .clone()
    } else {
        grobid_source_checkout_dir.clone()
    };
    Ok(root)
}
