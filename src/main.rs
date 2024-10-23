use std::io::Read;
use std::{collections::HashMap, fs, io};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

// const CTX_NL: &str = "🔬";
// const CTX_EOL: &str = "💉";
// const CTX_MID: &str = "🔭";
const CTX_NL: &str = "\n";
const CTX_EOL: &str = "\n";
const CTX_MID: &str = "\n";

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

    /// Apply ffwx diffs to a source file; note that the diff file can not be in human readable format to be applied
    #[command(name = "apply", alias = "ap")]
    Apply(ApplyArgs),
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
struct ApplyArgs {
    ///Path to the diff file
    #[arg(short)]
    ffwx_file: String,

    ///Path to the source file
    #[arg(short)]
    source_file: String,
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
    Changed,
}
impl DiffKind {
    fn to_header(&self) -> String {
        match self {
            DiffKind::Added => "+ ".to_string(),
            DiffKind::Removed => "- ".to_string(),
            DiffKind::Changed => "~ ".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CtxKind {
    Before,
    After,
}

#[derive(Debug, Clone)]
struct LineCtx {
	before: Vec<String>,
	after: Vec<String>,
}
impl LineCtx {
	fn new() -> Self {
		Self {
			before: Vec::new(),
			after: Vec::new(),
		}
	}

	fn push(&mut self, v: &Vec<String>, i: usize, amount: usize) {
		for j in 0..amount {
			let before = if i >= j + 1 { v.get(i - j - 1) } else { None };
            let after = if i + j + 1 < v.len() {
                v.get(i + j + 1)
            } else {
                None
            };
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
		format!("{}{}", self.before.join(CTX_NL), self.before.is_empty() ? "" : CTX_NL)
	}

	fn after_str(&self) -> String {
		format!("{}{}", CTX_NL, self.after.join(CTX_NL))
	}
}

#[derive(Debug, Clone)]
struct DiffLine {
    kind: DiffKind,
    value: String,
    ctx: LineCtx,
}
impl DiffLine {
    fn new(kind: DiffKind, value: String) -> Self {
        Self {
            kind,
            value,
            ctx: LineCtx::new(),
        }
    }

	fn added(value: String) -> Self {
		DiffLine::new(DiffKind::Added, value)
	}

	fn removed(value: String) -> Self {
		DiffLine::new(DiffKind::Removed, value)
	}

	fn changed(value: String) -> Self {
		DiffLine::new(DiffKind::Changed, value)
	}
}

fn compute_diff(source: Vec<String>, modified: Vec<String>) -> Vec<DiffLine> {
    let mut lines: Vec<DiffLine> = Vec::new();

	let mut i: usize = 0;
	let mut j: usize = 0;

	loop {
		if i >= source.len() && j >= modified.len() {
			println!("end of both files");
			break;
		}
		let (sline, mline) = (
			source.get(i),
			modified.get(j),
		);
		match (sline, mline) {
			(Some(s), Some(m)) => {
				println!("s = {s}, m = {m}");
			},
			(None, Some(m)) => {
				let line = DiffLine::added(m.to_string());
				lines.push(line);
			},
			(Some(s), None) => {
				let line = DiffLine::removed(s.to_string());
				lines.push(line);
			},
			(None, None) => {},
		}
		i += 1;
		j += 1;
	}

    return lines;
}

fn write_output<'a, W>(mut w: W, lines: Vec<DiffLine>) -> Result<usize, io::Error>
where W: io::Write {
    let mut buffer = String::new();
    for line in lines {
		let h = line.kind.to_header();
        buffer.push_str(&line.ctx.before_str());
        // buffer.push_str(CTX_NL);
        buffer.push_str(&h);
        buffer.push_str(&line.value);
        // buffer.push_str(CTX_NL);
		buffer.push_str(&line.ctx.after_str());
		buffer.push('\n');
        
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

            let mut diff = compute_diff(slines, mlines);
			if args.revert {
				diff.reverse();
			}

            match write_output(std::io::stdout(), diff) {
				Err(e) => eprintln!("Error writing output: {}", e),
				_ => (),
			}
        }
        Command::Apply(args) => todo!("Apply"),
    }
}

/*
TODO: list

- strip in between context lines when lines are in sequence; if before ctx of current line is equal to the line before this one, then this line is a sequence of the previous line
*/

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_lines(s: &str) -> Vec<String> {
        s.split(',').map(|x| x.to_string()).collect()
    }

    #[test]
    fn middle_context_of_1() {
        let lines = gen_lines("a,c,c,d");

        let mut l1 = DiffLine::new(DiffKind::Changed, "b".to_string());
        l1.ctx.push(&lines, 1, 1);
        assert_eq!(l1.ctx.before.len(), 1);
        assert_eq!(l1.ctx.before.get(0), Some(&"a".to_string()));
        assert_eq!(l1.ctx.after.len(), 1);
        assert_eq!(l1.ctx.after.get(0), Some(&"c".to_string()));
    }

    #[test]
    fn head_context_of_1() {
        let lines = gen_lines("a,c,c,d");

        let mut l1 = DiffLine::new(DiffKind::Changed, "b".to_string());
        l1.ctx.push(&lines, 0, 1);
        assert_eq!(l1.ctx.before.len(), 0);
        assert_eq!(l1.ctx.before.get(0), None);
        assert_eq!(l1.ctx.after.len(), 1);
        assert_eq!(l1.ctx.after.get(0), Some(&"c".to_string()));
    }

    #[test]
    fn tails_context_of_1() {
        let lines = gen_lines("a,c,c,d");

        let mut l1 = DiffLine::new(DiffKind::Changed, "b".to_string());
        l1.ctx.push(&lines, 3, 1);
        assert_eq!(l1.ctx.before.len(), 1);
        assert_eq!(l1.ctx.before.get(0), Some(&"c".to_string()));
        assert_eq!(l1.ctx.after.len(), 0);
        assert_eq!(l1.ctx.after.get(0), None);
    }

    #[test]
    fn should_trim_contigous_ctx_lines() {
        assert_eq!(true, true);
    }
}