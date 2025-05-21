// build_modules/utils.rs
use crate::build_modules::common::*;

pub fn run_command(
    cmd_path: &Path,
    args: &[&str],
    current_dir: &Path,
    env_vars: Option<&[(&str, &Path)]>, 
) -> Result<()> {
    let cmd_name = cmd_path.file_name().unwrap_or_default().to_string_lossy();
    print_cargo_warning(&format!(
        "Running command: {} {:?} in dir: {}",
        cmd_path.display(),
        args,
        current_dir.display()
    ));

    let mut command = Command::new(cmd_path);
    command.args(args).current_dir(current_dir);

    if let Some(vars) = env_vars {
        for (key, value) in vars {
            command.env(key, value);
            print_cargo_warning(&format!("  with env: {}={}", key, value.display()));
        }
    }

    let status = command
        .status()
        .with_context(|| format!("Failed to execute command: {}", cmd_path.display()))?;

    if !status.success() {
        bail!(
            "Command \"{} {:?}\" failed with status: {}. Working directory: {}",
            cmd_path.display(),
            args,
            status,
            current_dir.display()
        );
    }
    print_cargo_warning(&format!("Command {} finished successfully.", cmd_name));
    Ok(())
}

pub fn verify_sha256(path: &Path, expected_sha256: &str) -> Result<()> {
    print_cargo_warning(&format!(
        "Verifying SHA256 checksum for {}...",
        path.display()
    ));
    let mut file = fs::File::open(path)
        .with_context(|| format!("Failed to open file for SHA256 verification: {}", path.display()))?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)
        .with_context(|| format!("Failed to read file for SHA256 verification: {}", path.display()))?;
    let hash = hasher.finalize();
    let hex_hash = format!("{:x}", hash);

    if hex_hash == expected_sha256 {
        print_cargo_warning(&format!(
            "SHA256 checksum VERIFIED for {}: {}",
            path.display(),
            hex_hash
        ));
        Ok(())
    } else {
        bail!(
            "SHA256 checksum MISMATCH for {}. Expected: {}, Got: {}",
            path.display(),
            expected_sha256,
            hex_hash
        );
    }
} 