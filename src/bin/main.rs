use colored::*;
use dirstat_rs::{DiskItem, FileInfo};
use pretty_bytes::converter::convert as pretty_bytes;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

mod shape {
    pub const INDENT: &str = "│";
    pub const _LAST_WITH_CHILDREN: &str = "└─┬";
    pub const LAST: &str = "└──";
    pub const ITEM: &str = "├──";
    pub const _ITEM_WITH_CHILDREN: &str = "├─┬";
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_args();
    let current_dir = env::current_dir()?;
    let target_dir = config.target_dir.as_ref().unwrap_or(&current_dir);
    let file_info = FileInfo::from_path(&target_dir, config.apparent)?;

    match file_info {
        FileInfo::Directory { volume_id } => {
            println!("\n🔧  Analysing dir: {:?}\n", target_dir);
            let analysed = DiskItem::from_analyze(&target_dir, config.apparent, volume_id)?;
            show(&analysed, &config, DisplayInfo::new());
            Ok(())
        }
        _ => Err(format!("{} is not a directory!", target_dir.display()).into()),
    }
}

#[derive(Debug, Clone)]
struct DisplayInfo {
    parent_size: Option<u64>,
    level: usize,
    last: bool,
    indents: String,
}

impl DisplayInfo {
    fn new() -> Self {
        Self {
            parent_size: None,
            level: 0,
            last: false,
            indents: String::new(),
        }
    }
    // TODO: Consume or mut instead of cloning
    fn add_item(&self, parent_size: u64) -> Self {
        let indent = if self.last { " " } else { shape::INDENT };
        Self {
            parent_size: Some(parent_size),
            level: self.level + 1,
            last: false,
            indents: self.indents.clone() + indent + "  ",
        }
    }

    fn add_last(&self, parent_size: u64) -> Self {
        let indent = if self.last { " " } else { shape::INDENT };
        Self {
            parent_size: Some(parent_size),
            level: self.level + 1,
            last: true,
            indents: self.indents.clone() + indent + "  ",
        }
    }
}

fn show(item: &DiskItem, conf: &Config, info: DisplayInfo) {
    let percent = match info.parent_size {
        Some(size) => (item.disk_size as f64 / size as f64) * 100.0,
        None => 100.0
    };

    let percent_repr = if info.level == 0 {
        format!("{:.2}%", percent).green().bold()
    } else if percent > 20.0 {
        format!("{:.2}%", percent).red().bold()
    } else {
        format!("{:.2}%", percent).cyan()
    };

    if percent > conf.min_percent {
        println!(
            "{}{} {} [{}] => {:?}",
            //padding,
            info.indents,
            if info.last { shape::LAST } else { shape::ITEM },
            percent_repr,
            pretty_bytes(item.disk_size as f64),
            item.name
        );
        if info.level < conf.max_depth {
            if let Some(disk_items) = &item.children {
                if let Some((last_item, disk_items)) = disk_items.split_first() {
                    for disk_item in disk_items.iter().rev() {
                        show(disk_item, conf, info.add_item(item.disk_size))
                    }
                    show(last_item, conf, info.add_last(item.disk_size))
                }
            }
        }
    }
}

#[derive(StructOpt)]
struct Config {
    #[structopt(short = "d", default_value = "1")]
    /// Maximum recursion depth in directory
    max_depth: usize,

    #[structopt(
        short = "m",
        default_value = "0.1",
        parse(try_from_str = "parse_percent")
    )]
    /// Threshold that determines if entry is worth
    /// being shown. Between 0-100 % of dir size.
    min_percent: f64,

    #[structopt(parse(from_os_str))]
    target_dir: Option<PathBuf>,

    #[structopt(short = "a")]
    /// Apparent size on disk.
    apparent: bool,
}

fn parse_percent(src: &str) -> Result<f64, Box<dyn Error>> {
    let num = src.parse::<f64>()?;
    if num >= 0.0 && num <= 100.0 {
        Ok(num)
    } else {
        Err("Percentage must be in range [0, 100].".into())
    }
}
