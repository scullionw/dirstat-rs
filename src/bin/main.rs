use colored::*;
use dirstat_rs::DiskItem;
use pretty_bytes::converter::convert as pretty_bytes;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

const _SHAPES: [&str; 6] = [
    "└─┬",
    "└──",
    "├──",
    "├─┬",
    "─┬",
    "│",
];


fn main() -> Result<(), Box<Error>> {
    let config = Config::from_args();
    let current_dir = env::current_dir()?;
    let target_dir = config.target_dir.as_ref().unwrap_or(&current_dir);
    println!("\n🔧  Analysing dir: {:?}\n", target_dir);
    let analysed = DiskItem::from_analyze(&target_dir, config.apparent)?;
    show(&analysed, &config, None, 0);
    Ok(())
}

fn show(item: &DiskItem, conf: &Config, parent_size: Option<u64>, level: usize) {
    let padding = "-".repeat(level * 3);
    let percent = parent_size.map_or(100.0, |p_s| (item.disk_size as f64 / p_s as f64) * 100.0);

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
            pretty_bytes(item.disk_size as f64),
            item.name
        );
        if level < conf.max_depth {
            if let Some(disk_items) = &item.children {
                for disk_item in disk_items.iter().rev() {
                    show(disk_item, conf, Some(item.disk_size), level + 1)
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
        default_value = "1",
        parse(try_from_str = "parse_percent")
    )]
    /// Threshold that determines if entry is worth
    /// being shown. Between 0-100 % of dir size.
    min_percent: f64,

    #[structopt(parse(from_os_str))]
    target_dir: Option<PathBuf>,

    #[structopt(short = "a")]
    /// Activates apparent size using blocks on uniz-based systems
    apparent: bool,
}

fn parse_percent(src: &str) -> Result<f64, Box<Error>> {
    let num = src.parse::<f64>()?;
    if num >= 0.0 && num <= 100.0 {
        Ok(num)
    } else {
        Err("Percentage must be in range [0, 100].".into())
    }
}
