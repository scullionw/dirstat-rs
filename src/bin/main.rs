// use ansi_term::Colour;
use atty::Stream;
use dirstat_rs::{DiskItem, FileInfo};
use pretty_bytes::converter::convert as pretty_bytes;
use std::env;
use std::error::Error;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;
use termcolor::{Buffer, BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

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

    let color_choice = if atty::is(Stream::Stdout) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };

    let stdout = BufferWriter::stdout(color_choice);
    let mut buffer = stdout.buffer();

    match file_info {
        FileInfo::Directory { volume_id } => {
            println!("\n🔧  Analysing dir: {:?}\n", target_dir);
            let analysed = DiskItem::from_analyze(&target_dir, config.apparent, volume_id)?;
            show(&analysed, &config, DisplayInfo::new(), &mut buffer)?;
            stdout.print(&buffer)?;
            Ok(())
        }
        _ => Err(format!("{} is not a directory!", target_dir.display()).into()),
    }
}

fn show(item: &DiskItem, conf: &Config, info: DisplayInfo, buffer: &mut Buffer) -> io::Result<()> {
    // Show self
    show_item(item, &info, buffer)?;
    // Recursively show children
    if info.level < conf.max_depth {
        if let Some(children) = &item.children {
            let children = children
                .iter()
                .map(|child| (child, size_fraction(child, item)))
                .filter(|&(_, fraction)| fraction > conf.min_percent)
                .collect::<Vec<_>>();

            if let Some((last_child, children)) = children.split_first() {
                for &(child, fraction) in children.iter().rev() {
                    show(child, conf, info.add_item(fraction), buffer)?;
                }
                let &(child, fraction) = last_child;
                show(child, conf, info.add_last(fraction), buffer)?;
            }
        }
    }
    Ok(())
}

fn show_item(item: &DiskItem, info: &DisplayInfo, buffer: &mut Buffer) -> io::Result<()> {
    write!(buffer, "{}{}", info.indents, info.prefix())?;

    let color = Some(info.color());
    buffer.set_color(ColorSpec::new().set_fg(color))?;
    write!(buffer, " {} ", format!("{:.2}%", info.fraction))?;
    buffer.reset()?; // or set to Color::white, or None?

    writeln!(
        buffer,
        "[{}] => {:?}",
        pretty_bytes(item.disk_size as f64),
        item.name
    )?;
    Ok(())
}

fn size_fraction(child: &DiskItem, parent: &DiskItem) -> f64 {
    100.0 * (child.disk_size as f64 / parent.disk_size as f64)
}

#[derive(Debug, Clone)]
struct DisplayInfo {
    fraction: f64,
    level: usize,
    last: bool,
    indents: String,
}

impl DisplayInfo {
    fn new() -> Self {
        Self {
            fraction: 100.0,
            level: 0,
            last: true,
            indents: String::new(),
        }
    }
    // TODO: Consume or mut instead of cloning
    fn add_item(&self, fraction: f64) -> Self {
        Self {
            fraction,
            level: self.level + 1,
            last: false,
            indents: self.indents.clone() + self.indent() + "  ",
        }
    }

    fn add_last(&self, fraction: f64) -> Self {
        Self {
            fraction,
            level: self.level + 1,
            last: true,
            indents: self.indents.clone() + self.indent() + "  ",
        }
    }

    fn indent(&self) -> &'static str {
        if self.last {
            " "
        } else {
            shape::INDENT
        }
    }

    fn prefix(&self) -> &'static str {
        if self.last {
            shape::LAST
        } else {
            shape::ITEM
        }
    }

    fn color(&self) -> Color {
        if self.level == 0 {
            Color::Green
        } else if self.fraction > 20.0 {
            Color::Red
        } else {
            Color::Cyan
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
