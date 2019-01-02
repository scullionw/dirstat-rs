use colored::*;
use std::env;
use std::fs;
use std::path::Path;
use std::{error::Error, result};

type Result<T> = result::Result<T, Box<Error>>;

const MAX_DEPTH: usize = 3;

fn main() -> Result<()> {
    let current_dir = env::current_dir()?;
    dir_stats(&current_dir, 0)?;
    Ok(())
}

fn dir_stats(path: &Path, level: usize) -> Result<()> {
    let mut dirs = Vec::new();

    for entry in fs::read_dir(path)? {
        if let Ok(entry) = entry {
            let size = entry.metadata()?.len();
            if entry.file_type()?.is_dir() {
                dirs.push((entry, size, level));
            }
        }
    }

    let total_size = dirs.iter().map(|&(_, size, _)| size).sum::<u64>();
    dirs.sort_by_key(|&(_, size, _)| size);

    for (entry, size, level) in dirs.into_iter().rev() {
        let percent = (size as f64 / total_size as f64) * 100.0;

        let percent = if percent > 20.0 {
            format!("{:.1}%", percent).red().bold()
        } else {
            format!("{:.1}%", percent).blue()
        };

        let padding = "-".repeat(level * 3);

        println!("{}{} => {:?}", padding, percent, entry.file_name());

        if level < MAX_DEPTH {
            // Call recursively
            dir_stats(&entry.path(), level + 1)?;
        }
    }

    Ok(())
}
