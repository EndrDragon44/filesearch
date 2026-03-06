mod core;
mod model;

use crate::core::{SearchEngine, SearchConfig, SearchMode}; // for /CLI or -c
use std::env;
use std::result::Result;
// main.rs
// RECOMMEND USE CARGO 1.88! (sudo apt install rustc-1.88, cargo-1.88)
// or use 'rustup default 1.88' or 1.89 if unavalible.
// entrypoint
// calls the gui and runs it. If -c or /CLI is given, will directly call core
// this file is the entry point for VS Code that will build and run the gui by clicking the 
//run button that should be right around... Use this, not the default buttons up top.
// |||||
// VVVVV  here


fn main() {
    let args: Vec<String> = env::args().collect();

    // CLI behavior preserved
    if args.len() > 1 {
        match args[1].as_str() {
            "/?" | "--help" | "-h" => {
                print_help();
                return;
            }
            // later: cli switch; will require flag -c or /c or /cli or --cli to
            // be provided to actually run in this mode.
            "/CLI" | "-c" | "/cli" | "-C" | "/c" | "--cli" => {
            println!("CLI functionality! Not implemented yet :3 only dev should see this, shoo!")
            }
        }
    }

    // Default: launch GUI
    model::run_gui();

}

fn print_help() {
        let mut os = std::env::consts::OS;
        let mut arch = std::env::consts::ARCH;
        if arch == "aarch64" { //architechure aliasing (for an average user)
            if os == "macos" {
                arch = "M-series (ARM64)";
            } else {
                arch = "ARM64";
            }
        } else if arch == "x86_64" {
            arch = "x64";
        }
        // os name prettify
        if os == "macos" {
                os = "macOS";
        } else if os == "linux" {
                os = "Linux"
        } else if os == "windows" {
                os = "Windows";
        }

        println!("Filesearch - Revised");
        println!("Created by EndrDragon44");
        println!("{} {}",arch, os);
        println!("");
        println!("USAGE:");
        println!("  filesearch   Open the GUI Version");
        println!("  /CLI, -c     Run the Revised CLI version.");
        println!("");
        println!("  filesearch   Open the GUI Version");
        println!("  filesearch   Open the GUI Version");
        println!("  If you want to use the command-line version:");
        println!("  filesearch  [mode] [query] [search from dir] [flags]");
        println!("");
        println!("MODES:");
        println!("  /FM, -f      Search for files only");
        println!("  /SDM, -d     Search for directories only");
        println!("  /BOTH, -b    Search for both files and directories");
        println!("        This mode doesn't require a file extension to match an object.");
        println!("  /?, --help   Show this help message");
        println!("");
        println!("PATTERN SYNTAX:");
        println!("  *.txt              All text files (wildcards: *, ?)");
        println!("  report*.pdf        Files starting with 'report' and ending .pdf");
        println!("  image_??.jpg       Files like image_01.jpg, image_AB.jpg");
        println!("  document           Exact match 'document'");
        println!("");
        println!("OPTIONS:");
        println!("  --threads N, -t N    Number of threads (default: CPU cores)");
        println!("  --log FILE, -l FILE  Save results to log file");
        println!("  --case-insensitive, -i  Case-insensitive search");
        println!("  --depth N, -D N      Maximum directory depth");
        println!("  --dfs                Use Depth-First Search (default: BFS)");
        println!("  --no-wildcards, --nw Treat * and ? as literal characters");
        println!("");
        println!("EXAMPLES:");
        println!("  Basic usage:");
        println!("    filesearch /FM *.txt .");
        println!("    filesearch /SDM Documents ~");
        println!("");
        println!("  Multi-threaded searches:");
        println!("    filesearch /FM *.rs . --threads 8");
        println!("    filesearch /BOTH *config* . -i --threads 4");
        println!("");
        println!("  With logging and depth limits:");
        println!("    filesearch /FM *.tmp C:\\Users --depth 2 --log cleanup.txt");
        println!("    filesearch /SDM log /var --depth 3 --log system_logs.txt");
        println!("");
        println!("  Cross-platform examples:");
        println!("    filesearch /FM *.exe C:\\Windows");
        println!("    filesearch /SDM Applications /Applications");
        println!("    filesearch /BOTH backup . --log all_backups.txt");
}