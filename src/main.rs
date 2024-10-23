use std::io::Read;
use std::path::PathBuf;
use std::{collections::HashMap, fs, io};

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
    #[arg(short = 'R')]
    revert: bool,

    ///Write context lines separately
    #[arg(short = 'H')]
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
        if self.after.len() != other.after.len() || self.before.len() != other.before.len() {
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
        let end = if self.before.is_empty() { "" } else { CTX_NL };
        format!("{}{}", self.before.join(CTX_NL), end)
    }

    fn after_str(&self) -> String {
        let start = if self.before.is_empty() { "" } else { CTX_NL };
        format!("{}{}", start, self.after.join(CTX_NL))
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
    /*
    if lines are equal skip
    else cross compare the current line of each file until find a match
        while doing so collect all lines in separate buffers (one for each file)
        stop after finding a match or hit end of both files
    at the end if both buffers have the same number of lines
    then all lines on the modified buffer are "changed"
    else if match was found on the source file then lines are "removed" and if match was found on the modified file then lines are "added"
    */

    let mut i: usize = 0;
    let mut j: usize = 0;

    loop {
        if i >= source.len() && j >= modified.len() {
            break;
        }
        let (sline, mline) = (source.get(i), modified.get(j));
        match (sline, mline) {
            (Some(s), Some(m)) => {
                if s == m {
                    i += 1;
                    j += 1;
                    continue;
                }
                let mut src_buf: Vec<String> = vec![s.clone()];
                let mut mod_buf: Vec<String> = vec![m.clone()];

                let mut x: usize = 1;
                let mut y: usize = 1;

                loop {
                    let ns = source.get(i + x);
                    let nm = modified.get(j + y);

                    match (ns, nm) {
                        (Some(ns), Some(nm)) => {
							if ns == nm {
								break;
							} else {
								src_buf.push(ns.clone());
								mod_buf.push(nm.clone());
							}
							if m == ns {
								src_buf.pop();
								mod_buf.pop();
								mod_buf.pop();
                                break;
                            }
							if s == nm {
								src_buf.pop();
								src_buf.pop();
								mod_buf.pop();
                                break;
                            }
                        }
                        (Some(ns), None) => {
                            if m == ns {
                                break;
                            }
                            src_buf.push(ns.clone());
                        }
                        (None, Some(nm)) => {
                            if s == nm {
                                break;
                            }
                            mod_buf.push(nm.clone());
                        }
                        (None, None) => break,
                    }

                    x += 1;
                    y += 1;
                }
                if mod_buf.len() == src_buf.len() && !mod_buf.is_empty() {
                    for str in mod_buf {
                        let line = DiffLine::changed(str);
                        lines.push(line);
                    }
                } else {
                    if mod_buf.len() > src_buf.len() {
						j += mod_buf.len();
                        for str in mod_buf {
                            lines.push(DiffLine::added(str));
                        }
                    } else {
						i += src_buf.len();
                        for str in src_buf {
                            lines.push(DiffLine::removed(str));
                        }
                    }
                }
            }
            (None, Some(m)) => {
                let line = DiffLine::added(m.to_string());
                lines.push(line);
            }
            (Some(s), None) => {
                let line = DiffLine::removed(s.to_string());
                lines.push(line);
            }
            (None, None) => {}
        }
        i += 1;
        j += 1;
    }

    return lines;
}

fn write_output<'a, W>(mut w: W, lines: Vec<DiffLine>) -> Result<usize, io::Error>
where
    W: io::Write,
{
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

            let diff = compute_diff(slines, mlines);

            println!("\ndiff: {:#?}", diff);
            let file = fs::File::create("./out.ffwx");
            match write_output(file.expect("Error creating output file"), diff) {
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
