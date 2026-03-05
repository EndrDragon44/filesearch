//! core.rs
//! FileSearch core engine modul, called by model.rs
//!
// READY FOR GUI USE! import SearchEngine, SearchConfig, and SearchMode functions
// to communicate
//!
//! This file exposes the search engine and data structures for the GUI to call.
//! It intentionally preserves the original logic and semantics; only visibility,
//! small runtime counters, and safe accessor methods were added for GUI use.

use std::collections::{HashSet, VecDeque};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::result::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

/// Internal engine state (thread-safe via Arc<Mutex<GlobalState>>).
struct GlobalState {
    found_items: HashSet<PathBuf>,
    searched_dirs: HashSet<PathBuf>,
    pending_dirs: VecDeque<PathBuf>,
    log_file: Option<fs::File>,
    start_time: SystemTime,
    dirs_searched: usize,
    files_scanned: usize,
    matches_found: usize,

    // NEW (non-invasive): how many worker threads are currently active.
    // This is updated by the search() function when threads are spawned and when they finish.
    active_threads: usize,
}

/// Public configuration the GUI should create and pass to `SearchEngine::new`.
#[derive(Clone, Debug)]
pub struct SearchConfig {
    pub query: String,
    pub start_dir: PathBuf,
    pub mode: SearchMode,
    pub max_threads: usize,
    pub use_wildcards: bool,
    pub case_sensitive: bool,
    pub log_path: Option<PathBuf>,
    pub max_depth: Option<usize>,
    pub breadth_first: bool,
}

/// Public enum for search mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMode {
    File,
    Directory,
    Both,
}

// -----------------------------
// Pattern matching (private)
// -----------------------------
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

    fn match_wildcard(
        &self,
        pattern: &[char],
        text: &[char],
        p_idx: usize,
        t_idx: usize,
    ) -> bool {
        let mut p = p_idx;
        let mut t = t_idx;
        let mut text_idx = 0;
        let mut star_idx = None;

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

// -----------------------------
// SearchEngine
// -----------------------------
/// The search engine type used by the GUI. Create via `SearchEngine::new(config)`.
/// Call `search()` (blocking) in a background thread (the GUI should spawn it).
///
/// The engine keeps internal state in `Arc<Mutex<GlobalState>>`. The GUI can
/// call `engine.stats()` etc. to read live counters while search runs.
pub struct SearchEngine {
    state: Arc<Mutex<GlobalState>>,
    pub config: SearchConfig,
}

impl SearchEngine {
    /// Construct a new engine from a `SearchConfig`.
    pub fn new(config: SearchConfig) -> Self {
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
            active_threads: 0,
        };

        Self {
            state: Arc::new(Mutex::new(state)),
            config,
        }
    }


    ///
 
    /// Live stats (active threads, files scanned, matches found, etc.) are available
    /// via the accessor methods: `stats()`, `files_scanned()`, `matches_found()`, etc.
    pub fn search(&mut self) -> Vec<PathBuf> {
        // Setup logging if requested
        let log_path_clone = self.config.log_path.clone();
        if let Some(ref log_path) = log_path_clone {
            self.setup_logging(log_path);
        }

        println!("Starting search with {} thread(s)...", self.config.max_threads);
        println!("Pattern: {}", self.config.query);
        println!("Directory: {}", self.config.start_dir.display());

        let mut handles = Vec::new();
        let pattern = Arc::new(Pattern::new(&self.config.query, self.config.case_sensitive));

        // Determine thread count (cap at available_parallelism)
        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let thread_count = self.config.max_threads.min(num_cpus);

        // Set active_threads in shared state before spawning
        {
            let mut s = self.state.lock().unwrap();
            s.active_threads = thread_count;
        }

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

        // join threads (we ignore individual join errors but join all)
        for handle in handles {
            let _ = handle.join();
        }

        let state = self.state.lock().unwrap();
        state.found_items.iter().cloned().collect()
    }

    /// The worker thread function, identical behavior to the original code
    fn worker_thread(
        thread_id: usize,
        state: Arc<Mutex<GlobalState>>,
        pattern: Arc<Pattern>,
        config: SearchConfig,
    ) {
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

            // Cross-platform directory reading with error handling
            let entries = match fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(e) => {
                    // Skip directories we can't access
                    if  e.kind() == io::ErrorKind::PermissionDenied {
                        // nothing, report nothing to stop spam or similar
                    } else if !cfg!(windows) || e.kind() != io::ErrorKind::NotFound {
                        eprintln!(
                            "[Thread {}] Warning: Cannot read directory '{}': {}",
                            thread_id,
                            dir.display(),
                            e
                        );
                    }
                    continue;
                }
            };

            let mut subdirs = Vec::new();

            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        eprintln!(
                            "[Thread {}] Warning: Cannot read entry in '{}': {}",
                            thread_id,
                            dir.display(),
                            e
                        );
                        continue;
                    }
                };

                let path = entry.path();

                // Cross-platform file type detection with fallback
                let is_dir = match entry.file_type() {
                    Ok(file_type) => file_type.is_dir(),
                    Err(_) => {
                        if let Ok(metadata) = fs::metadata(&path) {
                            metadata.is_dir()
                        } else {
                            continue;
                        }
                    }
                };

                let is_file = !is_dir;

                let name = entry.file_name();
                let name_str = name.to_string_lossy();

                if name_str == "." || name_str == ".." {
                    continue;
                }

                {
                    let mut state_lock = state.lock().unwrap();
                    state_lock.files_scanned += 1;
                }

                if is_dir {
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
                } else if is_file && config.mode != SearchMode::Directory {
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
                                format!(
                                    "{:02}:{:02}:{:02}",
                                    (secs / 3600) % 24,
                                    (secs / 60) % 60,
                                    secs % 60
                                )
                            })
                            .unwrap_or_else(|_| "00:00:00".to_string());

                        let _ = writeln!(log_file, "[{}] {}", timestamp, message);
                    }
                }
            }
        }

        // commit any remaining local_found/logs to global state
        {
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
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    let _ = writeln!(log_file, "[{}] {}", timestamp, message);
                }
            }

            // Worker finished: decrement active_threads
            if state_lock.active_threads > 0 {
                state_lock.active_threads -= 1;
            }
        }
    }

    /// Calculate depth relative to `start_dir`.
    fn calculate_depth(path: &Path, start_dir: &Path) -> usize {
        let mut depth = 0;
        let mut current = path;

        while let Some(parent) = current.parent() {
            if parent == start_dir || parent.starts_with(start_dir) {
                break;
            }
            depth += 1;
            current = parent;

            // Handle Windows drive roots
            if cfg!(windows) && parent.components().count() == 1 {
                break;
            }
        }

        depth
    }

    /// Setup logging file (if configured). Non-destructive to existing logic.
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

    /// Monitor progress
    fn monitor_progress(&self) {
        let start = Instant::now();
        let mut last_print = Instant::now();

        loop {
            thread::sleep(Duration::from_millis(100));

            let mut state = self.state.lock().unwrap();

            if state.pending_dirs.is_empty() {
                let elapsed = start.elapsed();
                println!("\nSearch completed in {:.2} seconds!", elapsed.as_secs_f32());
                println!("Directories searched: {}", state.dirs_searched);
                println!("Files scanned: {}", state.files_scanned);
                println!("Matches found: {}", state.matches_found);

                let dirs_searched = state.dirs_searched;
                let files_scanned = state.files_scanned;
                let matches_found = state.matches_found;

                if let Some(ref mut log_file) = state.log_file {
                    writeln!(log_file, "\n{}", "=".repeat(80)).ok();
                    writeln!(
                        log_file,
                        "Search completed in {:.2} seconds",
                        elapsed.as_secs_f32()
                    )
                    .ok();
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

                print!(
                    "\rProgress: {} dirs, {} files, {} matches, {:.1} files/sec",
                    state.dirs_searched, state.files_scanned, state.matches_found, speed
                );
                io::stdout().flush().ok();

                last_print = Instant::now();
            }
        }
    }

    /// Save results to file. This is used by the CLI; GUI should have a separate 
    // button that saves the content of the table instead. See concepts.
    pub fn save_results(&self, custom_path: Option<PathBuf>) -> io::Result<PathBuf> {
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

        writeln!(file, "FileSearch Results")?;
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

    /// Cross-platform desktop path helper.
    fn get_desktop_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            if let Ok(user_profile) = env::var("USERPROFILE") {
                PathBuf::from(user_profile).join("Desktop")
            } else if let Ok(public) = env::var("PUBLIC") {
                PathBuf::from(public).join("Desktop")
            } else {
                env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(home) = env::var("HOME") {
                PathBuf::from(home).join("Desktop")
            } else {
                env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            }
        }

        #[cfg(target_os = "linux")]
        { //configure a way to redirect if the home is '/root' in case of sudo being weird or root user
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

    // ---------------------------
    // Accessors for GUI / host
    // ---------------------------

    /// dirs_searched, files_scanned, matches_found, pending_dirs, active_threads
    /// intended for GUI polling every ~500ms, will return the value.
    pub fn stats(&self) -> (usize, usize, usize, usize, usize) {
        let s = self.state.lock().unwrap();
        (
            s.dirs_searched,
            s.files_scanned,
            s.matches_found,
            s.pending_dirs.len(),
            s.active_threads,
          //s.time_elapsed, // implement later in a '1:03:02' (h:mm:ss) format; hour, double digit minute and DD second, given to gui as a string.
        )
    }

    /// Return list of current matches (cloned). GUI can call to get the content of listview.
    pub fn current_matches(&self) -> Vec<PathBuf> {
        let s = self.state.lock().unwrap();
        s.found_items.iter().cloned().collect()
    }

    /// Return files scanned count.
    pub fn files_scanned(&self) -> usize {
        let s = self.state.lock().unwrap();
        s.files_scanned
    }

    /// Return matches count.
    pub fn matches_found(&self) -> usize {
        let s = self.state.lock().unwrap();
        s.matches_found
    }

    /// Return number of active worker threads (derived).
    pub fn active_threads(&self) -> usize {
        let s = self.state.lock().unwrap();
        s.active_threads
    }

    /// Return number of pending directories.
    pub fn pending_dirs(&self) -> usize {
        let s = self.state.lock().unwrap();
        s.pending_dirs.len()
    }
} // impl SearchEngine

