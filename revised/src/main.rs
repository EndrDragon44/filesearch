#![allow(unused_imports)]

// Core
//extern crate std;  // Ensure std crate is linked; Commented as it prevents build on macOS.
// Speaking of macOS, High Sierra (macOS 10.13) is used to compile filesearch for Intel Macs.
// If anybody wants to build for M-Series Macs for the project and debug it, do contact me with an Issue on GitHub! https://www.github.com/endrdragon44/filesearch


use std::*;  // Wildcard import to get everything
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::result::Result;
use std::option::Option;

// Collections  
use std::collections::{HashSet, VecDeque};

// Sync
use std::sync::{Arc, Mutex};
use std::thread;

// Time
use std::time::{SystemTime, Instant, Duration};
// ==============================================
// DATA STRUCTURES
// ==============================================

struct GlobalState {
    found_items: HashSet<PathBuf>,
    searched_dirs: HashSet<PathBuf>,
    pending_dirs: VecDeque<PathBuf>,
    log_file: Option<fs::File>,
    start_time: SystemTime,
    dirs_searched: usize,
    files_scanned: usize,
    matches_found: usize,
}

#[derive(Clone)]
struct SearchConfig {
    query: String,
    start_dir: PathBuf,
    mode: SearchMode,
    max_threads: usize,
    use_wildcards: bool,
    case_sensitive: bool,
    log_path: Option<PathBuf>,
    max_depth: Option<usize>,
    breadth_first: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum SearchMode {
    File,
    Directory,
    Both,
}

// ==============================================
// PATTERN MATCHING
// ==============================================

struct Pattern {
    original: String,
    is_wildcard: bool,
    case_sensitive: bool,
}

impl Pattern {
    fn new(query: &str, case_sensitive: bool) -> Self {
        let is_wildcard = query.contains('*') || query.contains('?');
        Self {
            original: query.to_string(),
            is_wildcard,
            case_sensitive,
        }
    }
    
    fn matches(&self, text: &str) -> bool {
        if !self.is_wildcard {
            if self.case_sensitive {
                return text == self.original;
            } else {
                return text.eq_ignore_ascii_case(&self.original);
            }
        }
        
        let pattern_chars: Vec<char> = if self.case_sensitive {
            self.original.chars().collect()
        } else {
            self.original.to_lowercase().chars().collect()
        };
        
        let text_chars: Vec<char> = if self.case_sensitive {
            text.chars().collect()
        } else {
            text.to_lowercase().chars().collect()
        };
        
        self.match_wildcard(&pattern_chars, &text_chars, 0, 0)
    }
    
    fn match_wildcard(&self, pattern: &[char], text: &[char], p_idx: usize, t_idx: usize) -> bool {
        let mut p = p_idx;
        let mut t = t_idx;
        let mut star_idx = None;
        let mut text_idx = 0;
        
        while t < text.len() {
            if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
                p += 1;
                t += 1;
            } else if p < pattern.len() && pattern[p] == '*' {
                star_idx = Some(p);
                text_idx = t;
                p += 1;
            } else if let Some(si) = star_idx {
                p = si + 1;
                t = text_idx + 1;
                text_idx += 1;
            } else {
                return false;
            }
        }
        
        while p < pattern.len() && pattern[p] == '*' {
            p += 1;
        }
        
        p == pattern.len()
    }
}

// ==============================================
// THREAD-SAFE SEARCH ENGINE
// ==============================================

struct SearchEngine {
    state: Arc<Mutex<GlobalState>>,
    config: SearchConfig,
}

impl SearchEngine {
    fn new(config: SearchConfig) -> Self {
        let mut pending_dirs = VecDeque::new();
        pending_dirs.push_back(config.start_dir.clone());
        
        let state = GlobalState {
            found_items: HashSet::new(),
            searched_dirs: HashSet::new(),
            pending_dirs,
            log_file: None,
            start_time: SystemTime::now(),
            dirs_searched: 0,
            files_scanned: 0,
            matches_found: 0,
        };
        
        Self {
            state: Arc::new(Mutex::new(state)),
            config,
        }
    }
    
    fn search(&mut self) -> Vec<PathBuf> {
        // Setup logging if requested - fixed borrow issue
        let log_path_clone = self.config.log_path.clone();
        if let Some(ref log_path) = log_path_clone {
            self.setup_logging(log_path);
        }
        
        println!("Starting search with {} thread(s)...", self.config.max_threads);
        println!("Pattern: {}", self.config.query);
        println!("Directory: {}", self.config.start_dir.display());
        
        let mut handles = vec![];
        let pattern = Arc::new(Pattern::new(&self.config.query, self.config.case_sensitive));
        
        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let thread_count = self.config.max_threads.min(num_cpus);
        
        for thread_id in 0..thread_count {
            let state_clone = self.state.clone();
            let pattern_clone = pattern.clone();
            let config_clone = self.config.clone();
            
            let handle = thread::spawn(move || {
                Self::worker_thread(thread_id, state_clone, pattern_clone, config_clone);
            });
            
            handles.push(handle);
        }
        
        self.monitor_progress();
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        let state = self.state.lock().unwrap();
        state.found_items.iter().cloned().collect()
    }
    
    fn worker_thread(thread_id: usize, state: Arc<Mutex<GlobalState>>, pattern: Arc<Pattern>, config: SearchConfig) {
        let mut local_found = Vec::new();
        let mut local_logs = Vec::new();
        
        loop {
            let next_dir = {
                let mut state_lock = state.lock().unwrap();
                
                if state_lock.pending_dirs.is_empty() {
                    break;
                }
                
                state_lock.pending_dirs.pop_front()
            };
            
            let dir = match next_dir {
                Some(dir) => dir,
                None => continue,
            };
            
            {
                let mut state_lock = state.lock().unwrap();
                if state_lock.searched_dirs.contains(&dir) {
                    continue;
                }
                state_lock.searched_dirs.insert(dir.clone());
                state_lock.dirs_searched += 1;
            }
            
            if let Ok(entries) = fs::read_dir(&dir) {
                let mut subdirs = Vec::new();
                
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    
                    let file_type = match entry.file_type() {
                        Ok(ft) => ft,
                        Err(_) => continue,
                    };
                    
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    
                    if name_str == "." || name_str == ".." {
                        continue;
                    }
                    
                    {
                        let mut state_lock = state.lock().unwrap();
                        state_lock.files_scanned += 1;
                    }
                    
                    if file_type.is_dir() {
                        if config.mode != SearchMode::File && pattern.matches(&name_str) {
                            let message = format!("Found directory: {}", path.display());
                            local_found.push(path.clone());
                            local_logs.push(message);
                        }
                        
                        if let Some(max_depth) = config.max_depth {
                            let depth = Self::calculate_depth(&path, &config.start_dir);
                            if depth <= max_depth {
                                subdirs.push(path);
                            }
                        } else {
                            subdirs.push(path);
                        }
                        
                    } else if file_type.is_file() && config.mode != SearchMode::Directory {
                        if pattern.matches(&name_str) {
                            let message = format!("Found file: {}", path.display());
                            local_found.push(path.clone());
                            local_logs.push(message);
                        }
                    }
                }
                
                {
                    let mut state_lock = state.lock().unwrap();
                    if config.breadth_first {
                        state_lock.pending_dirs.extend(subdirs);
                    } else {
                        for subdir in subdirs.into_iter().rev() {
                            state_lock.pending_dirs.push_front(subdir);
                        }
                    }
                }
            }
            
            if local_found.len() >= 100 || local_logs.len() >= 100 {
                let mut state_lock = state.lock().unwrap();
                for path in local_found.drain(..) {
                    state_lock.found_items.insert(path);
                    state_lock.matches_found += 1;
                }
                for message in local_logs.drain(..) {
                    println!("[Thread {}] {}", thread_id, message);
                    
                    if let Some(ref mut log_file) = state_lock.log_file {
                        let timestamp = SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .map(|d| {
                                let secs = d.as_secs();
                                format!("{:02}:{:02}:{:02}", 
                                    (secs / 3600) % 24,
                                    (secs / 60) % 60,
                                    secs % 60)
                            })
                            .unwrap_or_else(|_| "00:00:00".to_string());
                        
                        let _ = writeln!(log_file, "[{}] {}", timestamp, message);
                    }
                }
            }
        }
        
        let mut state_lock = state.lock().unwrap();
        for path in local_found {
            state_lock.found_items.insert(path);
            state_lock.matches_found += 1;
        }
        for message in local_logs {
            println!("[Thread {}] {}", thread_id, message);
            
            if let Some(ref mut log_file) = state_lock.log_file {
                let timestamp = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| {
                        let secs = d.as_secs();
                        format!("{:02}:{:02}:{:02}", 
                            (secs / 3600) % 24,
                            (secs / 60) % 60,
                            secs % 60)
                    })
                    .unwrap_or_else(|_| "00:00:00".to_string());
                
                let _ = writeln!(log_file, "[{}] {}", timestamp, message);
            }
        }
    }
    
    fn calculate_depth(path: &Path, start_dir: &Path) -> usize {
        let mut depth = 0;
        let mut current = path;
        
        while let Some(parent) = current.parent() {
            if parent == start_dir || parent.starts_with(start_dir) {
                break;
            }
            depth += 1;
            current = parent;
        }
        
        depth
    }
    
    fn setup_logging(&mut self, log_path: &Path) {
        let mut state = self.state.lock().unwrap();
        
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(log_path)
        {
            writeln!(file, "{}", "=".repeat(80)).ok();
            writeln!(file, "FileSearch Session").ok();
            writeln!(file, "Started: {:?}", SystemTime::now()).ok();
            writeln!(file, "Query: {}", self.config.query).ok();
            writeln!(file, "Directory: {}", self.config.start_dir.display()).ok();
            writeln!(file, "{}", "=".repeat(80)).ok();
            
            state.log_file = Some(file);
            println!("Logging to: {}", log_path.display());
        }
    }
    
        fn monitor_progress(&self) {
        let start = Instant::now();
        let mut last_print = Instant::now();
        
        loop {
            thread::sleep(Duration::from_millis(100));
            
            let mut state = self.state.lock().unwrap();  // CHANGED: 'mut' added here
            
            if state.pending_dirs.is_empty() {
                let elapsed = start.elapsed();
                println!("\nSearch completed in {:.2} seconds!", elapsed.as_secs_f32());
                println!("Directories searched: {}", state.dirs_searched);
                println!("Files scanned: {}", state.files_scanned);
                println!("Matches found: {}", state.matches_found);
                
                // Fixed borrow issue: extract values before mutable borrow
                let dirs_searched = state.dirs_searched;
                let files_scanned = state.files_scanned;
                let matches_found = state.matches_found;
                
                if let Some(ref mut log_file) = state.log_file {
                    writeln!(log_file, "\n{}", "=".repeat(80)).ok();
                    writeln!(log_file, "Search completed in {:.2} seconds", elapsed.as_secs_f32()).ok();
                    writeln!(log_file, "Directories searched: {}", dirs_searched).ok();
                    writeln!(log_file, "Files scanned: {}", files_scanned).ok();
                    writeln!(log_file, "Matches found: {}", matches_found).ok();
                    writeln!(log_file, "{}", "=".repeat(80)).ok();
                }
                
                break;
            }
            
            if last_print.elapsed() > Duration::from_secs(2) {
                let elapsed = start.elapsed();
                let speed = state.files_scanned as f32 / elapsed.as_secs_f32().max(0.1);
                
                print!("\rProgress: {} dirs, {} files, {} matches, {:.1} files/sec", 
                    state.dirs_searched,
                    state.files_scanned,
                    state.matches_found,
                    speed
                );
                io::stdout().flush().ok();
                
                last_print = Instant::now();
            }
        }
    }
    
    fn save_results(&self, custom_path: Option<PathBuf>) -> io::Result<PathBuf> {
        let state = self.state.lock().unwrap();
        
        let save_path = match custom_path {
            Some(path) => path,
            None => {
                let desktop = Self::get_desktop_path();
                
                let timestamp = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                
                desktop.join(format!("FileSearch_Results_{}.log", timestamp))
            }
        };
        
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&save_path)?;

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
      
        writeln!(file, "FileSearch Results")?;
        writeln!(file, "System: {} {}",arch, os)?;
        writeln!(file, "Generated: {:?}", SystemTime::now())?;
        writeln!(file, "Query: {}", self.config.query)?;
        writeln!(file, "Search directory: {}", self.config.start_dir.display())?;
        writeln!(file, "{}", "-".repeat(80))?;
        
        let mut sorted_paths: Vec<_> = state.found_items.iter().collect();
        sorted_paths.sort();
        
        for path in sorted_paths {
            let metadata = fs::metadata(path);
            let size_info = match metadata {
                Ok(md) => format!(" ({})", Self::human_readable_size(md.len())),
                Err(_) => String::new(),
            };
            
            let item_type = if path.is_dir() { "[DIR] " } else { "[FILE]" };
            
            writeln!(file, "{} {}{}", item_type, path.display(), size_info)?;
        }
        
        writeln!(file, "{}", "=".repeat(80))?;
        writeln!(file, "Summary:")?;
        writeln!(file, "  Total matches: {}", state.matches_found)?;
        writeln!(file, "  Directories searched: {}", state.dirs_searched)?;
        writeln!(file, "  Files scanned: {}", state.files_scanned)?;
        
        Ok(save_path)
    }
    
    fn get_desktop_path() -> PathBuf {
        if cfg!(target_os = "windows") {
            if let Ok(user_profile) = env::var("USERPROFILE") {
                PathBuf::from(user_profile).join("Desktop")
            } else {
                env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            }
        } else if cfg!(target_os = "macos") {
            if let Ok(home) = env::var("HOME") {
                PathBuf::from(home).join("Desktop")
            } else {
                env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            }
        } else {
            if let Ok(home) = env::var("HOME") {
                PathBuf::from(home).join("Desktop")
            } else if let Ok(xdg_desktop) = env::var("XDG_DESKTOP_DIR") {
                PathBuf::from(xdg_desktop)
            } else {
                env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            }
        }
    }
    
    fn human_readable_size(bytes: u64) -> String {
        const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
        
        let mut size = bytes as f64;
        let mut unit_idx = 0;
        
        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }
        
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

// ==============================================
// COMMAND LINE INTERFACE
// ==============================================

fn parse_arguments() -> Result<SearchConfig, String> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        return Err("Insufficient arguments".to_string());
    }
    
    let mode = match args[1].as_str() {
        "/FM" | "/fm" | "-f" => SearchMode::File,
        "/SDM" | "/sdm" | "-d" => SearchMode::Directory,
        "/BOTH" | "/both" | "-b" => SearchMode::Both,
        "/?" | "/help" | "-h" | "--help" => {
            print_help();
            std::process::exit(0);
        }
        _ => return Err(format!("Unknown mode: {}", args[1])),
    };
    
    let query = args[2].clone();
    let start_dir = if args.len() >= 4 {
        PathBuf::from(&args[3])
    } else {
        env::current_dir().map_err(|e| e.to_string())?
    };
    
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    
    let mut config = SearchConfig {
        query,
        start_dir,
        mode,
        max_threads: num_cpus,
        use_wildcards: true,
        case_sensitive: true,
        log_path: None,
        max_depth: None,
        breadth_first: true,
    };
    
    let mut i = 4;
    while i < args.len() {
        match args[i].as_str() {
            "--threads" | "-t" => {
                if i + 1 < args.len() {
                    config.max_threads = args[i + 1].parse().unwrap_or(num_cpus);
                    i += 1;
                }
            }
            "--log" | "-l" => {
                if i + 1 < args.len() {
                    config.log_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--case-insensitive" | "-i" => {
                config.case_sensitive = false;
            }
            "--depth" | "-D" => {
                if i + 1 < args.len() {
                    config.max_depth = Some(args[i + 1].parse().unwrap_or(usize::MAX));
                    i += 1;
                }
            }
            "--dfs" => {
                config.breadth_first = false;
            }
            "--no-wildcards" | "-nw" => {
                config.use_wildcards = false;
            }
            _ => {
                if args[i].starts_with('-') {
                    return Err(format!("Unknown option: {}", args[i]));
                }
            }
        }
        i += 1;
    }
    
    Ok(config)
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
    println!("  filesearch [mode] [query] [search from dir] [flags]");
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
    println!("    filesearch /SDM ImageFiles /Volumes/Backups/");
    println!("    filesearch /BOTH backup . --log /home/dave/Desktop/all_backups.txt");
}

fn main() {
    match parse_arguments() {
        Ok(config) => {
            if !config.start_dir.exists() {
                eprintln!("Error: Start directory does not exist: {}", 
                         config.start_dir.display());
                std::process::exit(1);
            }
            
            let mut engine = SearchEngine::new(config);
            
            

            let results = engine.search();
            
            if !results.is_empty() {
                println!("\nFound {} matches.", results.len());
                
                if engine.config.log_path.is_none() {
                    println!("Save results to desktop? [Y/n]");
                    let mut response = String::new();
                    io::stdin().read_line(&mut response).ok();
                    
                    if response.trim().is_empty() || response.trim().to_lowercase() == "y" {
                        match engine.save_results(None) {
                            Ok(path) => println!("Results saved to: {}", path.display()),
                            Err(e) => eprintln!("Error saving results: {}", e),
                        }
                    }
                } else {
                    println!("Results logged to specified file.");
                }
            } else {
                println!("No matches found.");
            }
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            print_help();
            std::process::exit(1);
        }
    }
}
