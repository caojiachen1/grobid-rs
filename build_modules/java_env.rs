use crate::build_modules::common::*;

pub fn locate_java_home() -> Result<PathBuf> {
    print_cargo_warning("Attempting to locate JAVA_HOME...");
    // 1. Check environment variable
    if let Ok(java_home_env) = env::var(JAVA_HOME_ENV_VAR) {
        // Parse environment JAVA_HOME and adjust for macOS .jdk packaging if necessary
        let mut path = PathBuf::from(&java_home_env);
        let mac_jdk_home = path.join("Contents").join("Home");
        if !path.join("bin/javac").exists() && mac_jdk_home.join("bin/javac").exists() {
            print_cargo_warning(&format!(
                "Using JAVA_HOME from environment with macOS .jdk structure: {}",
                mac_jdk_home.display()
            ));
            path = mac_jdk_home;
        }
        if path.exists() && path.join("bin/javac").exists() {
            print_cargo_warning(&format!("Using JAVA_HOME from environment: {}", path.display()));
            return Ok(path);
        } else {
            print_cargo_warning(&format!(
                "Warning: JAVA_HOME environment variable set to '{}', but it doesn't seem to be a valid JDK path (missing bin/javac). Trying auto-detection...",
                path.display()
            ));
        }
    }

    // 2. Use java_locator if environment variable is not set or invalid
    print_cargo_warning("JAVA_HOME not found or invalid in environment. Trying java_locator...");
    match java_locator::locate_java_home() {
        Ok(path_str) => {
            // Adjust for macOS .jdk packaging if necessary
            let mut path = PathBuf::from(&path_str);
            let mac_jdk_home = path.join("Contents").join("Home");
            if !path.join("bin/javac").exists() && mac_jdk_home.join("bin/javac").exists() {
                print_cargo_warning(&format!(
                    "Using JAVA_HOME from java_locator with macOS .jdk structure: {}",
                    mac_jdk_home.display()
                ));
                path = mac_jdk_home;
            }
            if path.exists() && path.join("bin/javac").exists() {
                print_cargo_warning(&format!(
                    "Located JAVA_HOME using java_locator: {}",
                    path.display()
                ));
                Ok(path)
            } else {
                bail!(
                    "java_locator found path '{}', but it does not appear to be a valid JDK (missing bin/javac).", 
                    path.display()
                );
            }
        }
        Err(e) => {
            bail!("Failed to locate JAVA_HOME using java_locator: {}. Please set JAVA_HOME environment variable.", e);
        }
    }
} 