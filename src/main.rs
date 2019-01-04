use colored::*;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use structopt::StructOpt;

fn main() -> Result<(), Box<Error>> {
    let config = Config::from_args();
    let current_dir = env::current_dir()?;
    let analysed = DiskItem::new(&current_dir)?;
    analysed.show(0, config.max_depth);
    Ok(())
}

struct DiskItem {
    name: std::ffi::OsString,
    disk_size: u64,
    children: Option<Vec<DiskItem>>,
}

impl DiskItem {
    fn new(path: &Path) -> Result<Self, Box<Error>> {
        if path.is_dir() {
            let mut sub_dirs = vec![];
            for entry in fs::read_dir(path)? {
                let disk_item = DiskItem::new(&entry?.path())?;
                sub_dirs.push(disk_item);
            }
            sub_dirs.sort_unstable_by_key(|di| di.disk_size);
            Ok(DiskItem {
                name: path.file_name().unwrap().to_os_string(),
                disk_size: sub_dirs.iter().map(|di| di.disk_size).sum(),
                children: Some(sub_dirs),
            })
        } else {
            Ok(DiskItem {
                name: path.file_name().unwrap().to_os_string(),
                disk_size: path.metadata()?.len(),
                children: None,
            })
        }
    }

    fn show(&self, level: usize, max_depth: usize) {
        let padding = "-".repeat(level * 3);
        match level {
            0 => self.show_children(level + 1, max_depth),
            d if d < max_depth => {
                println!("{}{} => {:?}", padding, self.disk_size, self.name);
                self.show_children(level + 1, max_depth)
            }
            _ => println!("{}{} => {:?}", padding, self.disk_size, self.name),
        }
    }

    fn show_children(&self, level: usize, max_depth: usize) {
        if let Some(disk_items) = &self.children {
            for disk_item in disk_items.iter().rev() {
                disk_item.show(level + 1, max_depth)
            }
        }
    }
}

#[derive(StructOpt, Debug)]
struct Config {
    // Maximum recursion depth in directory
    #[structopt(short = "d", default_value = "3")]
    max_depth: usize,

    // Threshold that determines if entry is worth
    // being shown. Between 0-100 % of dir size.
    #[structopt(
        short = "m",
        default_value = "1",
        parse(try_from_str = "parse_percent")
    )]
    min_percent: f64,
}

// Custom function to validate input
fn parse_percent(src: &str) -> Result<f64, Box<Error>> {
    let num = src.parse::<f64>()?;
    if num > 0.0 && num < 100.0 {
        Ok(num)
    } else {
        Err("Percentage must be in range [0, 100].".into())
    }
}