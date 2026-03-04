use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

struct FileSearch {
    found_items: HashSet<PathBuf>,
}

impl FileSearch {
    fn new() -> Self {
        Self {
            found_items: HashSet::new(),
        }
    }

    fn search_file(&mut self, directory: &Path, filename: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("Error reading directory '{}': {}", directory.display(), e);
                return paths;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            let path = entry.path();

            if file_type.is_dir() {
                // Skip . and ..
                let file_name = match entry.file_name().into_string() {
                    Ok(name) => name,
                    Err(_) => continue,
                };

                if file_name == "." || file_name == ".." {
                    continue;
                }

                // Recursively search subdirectories
                let mut subpaths = self.search_file(&path, filename);
                paths.append(&mut subpaths);
            } else if file_type.is_file() {
                // Check if filename matches
                if entry.file_name() == filename {
                    if !self.found_items.contains(&path) {
                        paths.push(path.clone());
                        self.found_items.insert(path.clone());
                        println!("Found file: {}", path.display());
                    }
                }
            }
        }

        paths
    }

    fn search_subdir(&mut self, directory: &Path, subdirname: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("Error reading directory '{}': {}", directory.display(), e);
                return paths;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if file_type.is_dir() {
                let file_name = match entry.file_name().into_string() {
                    Ok(name) => name,
                    Err(_) => continue,
                };

                // Skip . and ..
                if file_name == "." || file_name == ".." {
                    continue;
                }

                let path = entry.path();

                // Check if directory name matches
                if file_name == subdirname {
                    if !self.found_items.contains(&path) {
                        paths.push(path.clone());
                        self.found_items.insert(path.clone());
                        println!("Found subdirectory: {}", path.display());
                    }
                }

                // Recursively search subdirectories
                let mut subpaths = self.search_subdir(&path, subdirname);
                paths.append(&mut subpaths);
            }
        }

        paths
    }

    fn file_mode(&mut self, filename: &str, directory: &Path) {
        let mut results = self.search_file(directory, filename);

        if results.is_empty() {
            eprintln!(
                "No file(s) in this directory tree matched your query. Make sure that you spelled the query filename correctly.\n\
                If your query filename contains spaces, you should have your query inside double-quotes (\") (e.g., \"Secret notes.txt\").\n\
                Note: Query is CASE-SENSITIVE"
            );
            return;
        }

        loop {
            print!("Would you like to continue searching for more of the same file? (Y/N) ");
            io::stdout().flush().unwrap();

            let mut response = String::new();
            io::stdin().read_line(&mut response).unwrap();
            let response = response.trim();

            if response.eq_ignore_ascii_case("Y") {
                let new_results = self.search_file(directory, filename);
                if new_results.is_empty() {
                    println!("No more files found.");
                    return;
                }
                results.extend(new_results);
            } else if response.eq_ignore_ascii_case("N") {
                return;
            } else {
                println!("Invalid response. Please enter 'Y' or 'N'.");
            }
        }
    }

    fn subdir_mode(&mut self, subdirname: &str, directory: &Path) {
        let results = self.search_subdir(directory, subdirname);

        if results.is_empty() {
            eprintln!(
                "No subdirectory in this directory tree matched your query. Make sure that you spelled the query subdirectory correctly.\n\
                If your query subdirectory contains spaces, you should have your query inside double-quotes (\") (e.g., \"My Documents\").\n\
                Note: Query is CASE-SENSITIVE"
            );
            return;
        }
    }
}

fn print_help() {
    println!(
        "Filesearch - By Endragon44 on GitHub
        Usage: FileSearch [ /FM | /SDM ] search_query [directory]\n\
        Switches:\n\
        \t\t Switch /SDM tells the program to execute in Sub-Directory Mode. This feature allows you to search for a directory (folder) rather than a file.\n\
        \tExample: FileSearch /SDM WorkDocuments [directory to search in, default is the directory this program is located in]\n\n\
        \t\tSwitch /FM tells the program to execute in FileMode. This feature allows you to search for your files anywhere on your computer.\n\
        \tExample: FileSearch /FM DocumentForWork.docx [directory to search in, default is the directory this program is located in]\n\
        \tThe query for this mode is CASE-SENSITIVE!\n"
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        print_help();
        std::process::exit(1);
    }

    let mode = &args[1];
    let query = &args[2];

    let directory = if args.len() >= 4 {
        PathBuf::from(&args[3])
    } else {
        match env::current_dir() {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Error getting current directory: {}", e);
                std::process::exit(1);
            }
        }
    };

    let mut searcher = FileSearch::new();

    match mode.as_str() {
        "/FM" => searcher.file_mode(query, &directory),
        "/SDM" => searcher.subdir_mode(query, &directory),
        "/?" | "/help" | "/h" => print_help(),
        _ => {
            eprintln!("Invalid mode. Use /FM for file mode or /SDM for subdirectory mode. Use /? for help");
            print_help();
            std::process::exit(1);
        }
    }
}
