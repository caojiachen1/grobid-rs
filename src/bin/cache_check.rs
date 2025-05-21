use grobid_rs::get_cache_dir;

fn main() {
    println!("Grobid-rs cache directory:");
    match get_cache_dir() {
        Ok(path) => {
            println!("Cache directory: {}", path.display());

            if path.exists() {
                println!("Directory exists.");

                // List contents
                if let Ok(entries) = std::fs::read_dir(&path) {
                    println!("\nContents:");
                    let mut has_files = false;

                    for entry in entries {
                        if let Ok(entry) = entry {
                            has_files = true;
                            println!("  {}", entry.path().display());
                        }
                    }

                    if !has_files {
                        println!("  (empty directory)");
                    }
                } else {
                    println!("Could not read directory contents.");
                }
            } else {
                println!("Directory does not exist yet.");
            }
        }
        Err(e) => println!("Error getting cache directory: {}", e),
    }
}
