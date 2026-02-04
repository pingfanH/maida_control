use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use log::{Level, LevelFilter, Log, Metadata, Record};
use chrono::Duration;
use sqlx::types::chrono::{Local, NaiveDate};

const LOG_DIR: &str = "./logs";
const MAX_SIZE_BYTES: u64 = 10 * 1024 * 1024;
const RETAIN_DAYS: i64 = 30;

pub fn init_logger() -> Result<(), log::SetLoggerError> {
    fs::create_dir_all(LOG_DIR).ok();
    let logger = FileLogger::new(PathBuf::from(LOG_DIR));
    logger.cleanup_old_files();
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(LevelFilter::Info);
    Ok(())
}

struct FileLogger {
    dir: PathBuf,
    state: Mutex<LogState>,
}

struct LogState {
    date: NaiveDate,
    index: u32,
    file: Option<File>,
    size: u64,
}

impl FileLogger {
    fn new(dir: PathBuf) -> Self {
        let today = Local::now().date_naive();
        Self {
            dir,
            state: Mutex::new(LogState {
                date: today,
                index: 0,
                file: None,
                size: 0,
            }),
        }
    }

    fn file_path(&self, date: NaiveDate, index: u32) -> PathBuf {
        let name = if index == 0 {
            format!("{}.log", date)
        } else {
            format!("{}.{}.log", date, index)
        };
        self.dir.join(name)
    }

    fn open_current(&self, state: &mut LogState) -> io::Result<()> {
        let path = self.file_path(state.date, state.index);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        state.file = Some(file);
        state.size = size;
        Ok(())
    }

    fn rotate_if_needed(&self, state: &mut LogState, add_len: u64) {
        let today = Local::now().date_naive();
        if state.date != today {
            state.date = today;
            state.index = 0;
            state.file = None;
            state.size = 0;
        }
        if state.file.is_none() {
            let _ = self.open_current(state);
        }
        if state.size + add_len > MAX_SIZE_BYTES {
            state.index += 1;
            state.file = None;
            state.size = 0;
            let _ = self.open_current(state);
        }
    }

    fn cleanup_old_files(&self) {
        let cutoff = Local::now().date_naive() - Duration::days(RETAIN_DAYS);
        let Ok(entries) = fs::read_dir(&self.dir) else { return; };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("log") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let date_part = stem.split('.').next().unwrap_or("");
            if let Ok(date) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
                if date < cutoff {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let now = Local::now();
        let line = format!(
            "[{}][{}] {}\n",
            now.format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.args()
        );
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        self.rotate_if_needed(&mut state, line.len() as u64);
        if let Some(file) = state.file.as_mut() {
            let _ = file.write_all(line.as_bytes());
            state.size += line.len() as u64;
        }
        let _ = io::stdout().write_all(line.as_bytes());
    }

    fn flush(&self) {}
}
