use std::io::Read;
use std::{collections::HashMap, fs, io};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

const CTX_NL: &str = "🔬";
const CTX_EOL: &str = "💉";
const CTX_MID: &str = "🔭";
// const CTX_NL: &str = ":";
// const CTX_EOL: &str = ";";
// const CTX_MID: &str = "::";

#[derive(Debug, Parser)]
#[command(name = "ffwx")]
#[command(author = "Nullenbox")]
#[command(version = "0.1.0")]
#[command(
    about = "diff and rebuild files",
    long_about = "Computes ffwx between two files or rebuild the modified file from ffwx and source file; ffwx (diFF With conteXt) is a simplified diff format"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Computes the ffwx between the source file and the modified file;
    /// use "df" for short
    #[command(name = "diff", alias = "df")]
    Diff(DiffArgs),

    /// Rebuilds the modified file using the ffwx and source file
    #[command(name = "rb")]
    Rebuild(RebuildArgs),
}

#[derive(Debug, Args)]
struct DiffArgs {
    ///Path to the source file
    #[arg(short)]
    source_file: String,

    ///Path to the modified file
    #[arg(short)]
    modified_file: String,

	///Revert the diff list before writing to file
	#[arg(short='R')]
	revert: bool,

	///Write context lines separately
	#[arg(short='H')]
	human_readable: bool,
}

#[derive(Debug, Args)]
struct RebuildArgs {
    ///Path to the source file
    #[arg(short)]
    source_file: String,

    ///Path to the diff file
    #[arg(short)]
    ffwx_file: String,

    ///Path to output file
    #[arg(short)]
    output_file: String,
}

#[derive(Debug, Clone)]
struct DelimiterGenerator {
	readable: bool,
}
impl DelimiterGenerator {
	fn new(readable: bool) -> Self {
		Self {
			readable,
		}
	}

	fn new_line(&self) -> &str {
		if self.readable {"\n"} else {CTX_NL}
	}

	fn end_of_line(&self) -> &str {
		if self.readable {""} else {CTX_EOL}
	}

	fn halfway(&self) -> &str {
		if self.readable {"\n"} else {CTX_MID}
	}
}

fn get_lines_from_file(path: &PathBuf) -> Result<Vec<String>, io::Error> {
    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
	file.read_to_string(&mut contents)?;
    Ok(contents.lines().map(|s| s.to_string()).collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DiffKind {
    Added,
    Removed,
    Modified,
}
impl DiffKind {
    fn to_header(&self) -> String {
        match self {
            DiffKind::Added => "+ ".to_string(),
            DiffKind::Removed => "- ".to_string(),
            DiffKind::Modified => "~ ".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CtxKind {
    Before,
    After,
}

#[derive(Debug, Clone)]
struct LineCtx<'a> {
	d: &'a DelimiterGenerator,
	before: Vec<String>,
	after: Vec<String>,
}
impl<'a> LineCtx<'a> {
	fn new(d: &'a DelimiterGenerator) -> Self {
		Self {
			d,
			before: Vec::new(),
			after: Vec::new(),
		}
	}

	fn push(&mut self, v: &Vec<String>, i: usize, amount: usize) {
		for j in 0..amount {
			let before = v.get(i - j - 2);
            let after = v.get(i + j);
			match (before, after) {
                (Some(b), Some(a)) => {
                    self.before.push(b.clone());
                    self.after.push(a.clone());
                }
                (Some(b), None) => {
                    self.before.push(b.clone());
                }
                (None, Some(a)) => {
                    self.after.push(a.clone());
                }
                _ => continue,
            }
		}
	}

	fn compare(&self, other: &LineCtx) -> bool {
		if self.after.len() != other.after.len()
            || self.before.len() != other.before.len()
        {
            return false;
        }

		for i in 0..self.after.len() {
			let (a, b) = (&self.after[i], &other.after[i]);
			if a.len() != b.len() || a != b {
				return false;
			}
		}

        return true;
	}

	fn before_str(&self) -> String {
		self.before.join(self.d.new_line())
	}

	fn after_str(&self) -> String {
		self.after.join(self.d.new_line())
	}
}

#[derive(Debug, Clone)]
struct DiffLine<'a> {
    kind: DiffKind,
    value: String,
    ctx: LineCtx<'a>,
}
impl<'a> DiffLine<'a> {
    fn new(kind: DiffKind, value: String, d: &'a DelimiterGenerator) -> Self {
        Self {
            kind,
            value,
            ctx: LineCtx::new(d),
        }
    }
}

fn compute_diff<'a>(source: Vec<String>, modified: Vec<String>, d: &'a DelimiterGenerator) -> Vec<DiffLine<'a>> {
    let mut lines: Vec<DiffLine> = Vec::new();

    let mut modified_map: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, line) in modified.iter().enumerate() {
        modified_map.entry(line).or_insert_with(Vec::new).push(i);
    }

    let mut lcs: Vec<Vec<usize>> = vec![vec![0; modified.len() + 1]; source.len() + 1];

    for i in 1..=source.len() {
        for j in 1..=modified.len() {
            if source[i - 1] == modified[j - 1] {
                lcs[i][j] = lcs[i - 1][j - 1] + 1;
            } else {
                lcs[i][j] = lcs[i - 1][j].max(lcs[i][j - 1]);
            }
        }
    }

    let mut i = source.len();
    let mut j = modified.len();
    while i > 0 && j > 0 {
        if source[i - 1] == modified[j - 1] {
            i -= 1;
            j -= 1;
        } else if lcs[i][j] == lcs[i - 1][j] {
            let mut line = DiffLine::new(DiffKind::Removed, source[i - 1].clone(), d);
            line.ctx.push(&source, i, 1);
            lines.push(line);
            i -= 1;
        } else {
            // println!("s: {}, m: {}", source[i - 1], modified[j - 1]);

            //TODO: compare context to determine if line was modified or deleted
			
            let mut line = DiffLine::new(DiffKind::Added, modified[j - 1].clone(), d);
            line.ctx.push(&modified, j, 1);
			
			for (x, l) in lines.iter().enumerate() {
				if l.ctx.compare(&line.ctx) {
					line.kind = DiffKind::Modified;
					lines.remove(x);
					break;
				}
			}

            lines.push(line);
            j -= 1;
        }
    }

    while i > 0 {
        let mut line = DiffLine::new(DiffKind::Removed, source[i - 1].clone(), d);
        line.ctx.push(&source, i, 1);
        lines.push(line);
        i -= 1;
    }

    while j > 0 {
        let mut line = DiffLine::new(DiffKind::Added, modified[j - 1].clone(), d);
        line.ctx.push(&modified, j, 1);
        lines.push(line);
        j -= 1;
    }
    return lines;
}

fn write_output<'a, W>(mut w: W, lines: Vec<DiffLine>, d: &'a DelimiterGenerator) -> Result<usize, io::Error>
where W: io::Write {
    let mut buffer = String::new();
    for line in lines {
		let h = line.kind.to_header();
        if d.readable {
			buffer.push_str(&line.ctx.before_str());
	        buffer.push_str(d.new_line());
	        buffer.push_str(&h);
	        buffer.push_str(&line.value);
	        buffer.push_str(d.new_line());
			buffer.push_str(&line.ctx.after_str());
			buffer.push('\n');
		} else {
			buffer.push_str(&h);
	        buffer.push_str(&line.value);
	        buffer.push_str(d.end_of_line());
			buffer.push_str(&line.ctx.before_str());
	        buffer.push_str(d.halfway());
			buffer.push_str(&line.ctx.after_str());
			buffer.push('\n');
		}
        
    }
	w.write(buffer.as_bytes())
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Command::Diff(args) => {
            let source = PathBuf::from(&args.source_file);
            let modified = PathBuf::from(&args.modified_file);

            let slines = get_lines_from_file(&source);
            let mlines = get_lines_from_file(&modified);
            if slines.is_err() {
                eprintln!("Error reading source file: {}", slines.unwrap_err());
                return;
            }
            if mlines.is_err() {
                eprintln!("Error reading modified file: {}", mlines.unwrap_err());
                return;
            }
            let slines = slines.unwrap();
            let mlines = mlines.unwrap();

			let delimiter_gen = DelimiterGenerator::new(args.human_readable);

            let mut diff = compute_diff(slines, mlines, &delimiter_gen);
			if args.revert {
				diff.reverse();
			}

            match write_output(std::io::stdout(), diff, &delimiter_gen) {
				Err(e) => eprintln!("Error writing output: {}", e),
				_ => (),
			}
        }
        Command::Rebuild(args) => todo!("Rebuild"),
    }
}
