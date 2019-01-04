use colored::*;
use pretty_bytes::converter::convert as pretty_bytes;
use rayon::prelude::*;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use structopt::StructOpt;

fn main() -> Result<(), Box<Error>> {
    let config = Config::from_args();
    let current_dir = env::current_dir()?;
    let analysed = DiskItem::from_analyze(&current_dir)?;
    analysed.show(&config, None, 0);
    Ok(())
}

struct DiskItem {
    name: std::ffi::OsString,
    disk_size: u64,
    children: Option<Vec<DiskItem>>,
}

impl DiskItem {
    fn from_analyze(path: &Path) -> Result<Self, Box<Error>> {
        let name = path
            .file_name()
            .ok_or("Filename could not be read.")?
            .to_os_string();

        if path.is_dir() {
            let sub_entries = fs::read_dir(path)?
                .filter_map(Result::ok)
                .collect::<Vec<_>>();

            let mut sub_items = sub_entries
                .par_iter()
                .filter_map(|entry| DiskItem::from_analyze(&entry.path()).ok())
                .collect::<Vec<_>>();

            sub_items.sort_unstable_by_key(|di| di.disk_size);

            Ok(DiskItem {
                name,
                disk_size: sub_items.iter().map(|di| di.disk_size).sum(),
                children: Some(sub_items),
            })
        } else {
            Ok(DiskItem {
                name,
                // If we can't read meta_data, set size to 0.
                disk_size: path.metadata().map(|m| m.len()).unwrap_or(0),
                children: None,
            })
        }
    }

    fn show(&self, conf: &Config, parent_size: Option<u64>, level: usize) {
        let padding = "-".repeat(level * 3);
        let percent = parent_size.map_or(100.0, |p_s| (self.disk_size as f64 / p_s as f64) * 100.0);

        // Select color
        let percent_repr = if level == 0 {
            format!("{:.2}%", percent).green().bold()
        } else if percent > 20.0 {
            format!("{:.2}%", percent).red().bold()
        } else {
            format!("{:.2}%", percent).cyan()
        };

        if percent > conf.min_percent {
            println!(
                "{}{} [{}] => {:?}",
                padding,
                percent_repr,
                pretty_bytes(self.disk_size as f64),
                self.name
            );
            if level < conf.max_depth {
                if let Some(disk_items) = &self.children {
                    for disk_item in disk_items.iter().rev() {
                        disk_item.show(conf, Some(self.disk_size), level + 1)
                    }
                }
            }
        }
    }
}

#[derive(StructOpt)]
struct Config {
    #[structopt(short = "d", default_value = "3")]
    /// Maximum recursion depth in directory
    max_depth: usize,

    #[structopt(
        short = "m",
        default_value = "1",
        parse(try_from_str = "parse_percent")
    )]
    /// Threshold that determines if entry is worth
    /// being shown. Between 0-100 % of dir size.
    min_percent: f64,
}

fn parse_percent(src: &str) -> Result<f64, Box<Error>> {
    let num = src.parse::<f64>()?;
    if num > 0.0 && num < 100.0 {
        Ok(num)
    } else {
        Err("Percentage must be in range [0, 100].".into())
    }
}
