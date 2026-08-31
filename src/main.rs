mod bounds;
mod cut;
mod format;
mod split;

use bounds::parse_bounds;
use cut::{run, CutMode, Options, TrimMode};
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::process;
use termcolor::{ColorChoice, StandardStream};

const VERSION: &str = "1.3.0";

const HELP: &str = r#"tuc 1.3.0
Cut text (or bytes) where a delimiter matches, then keep the desired parts.

USAGE:
    tuc [FLAGS] [OPTIONS] < input
    tuc [FLAGS] [OPTIONS] filepath

FLAGS:
    -g, --greedy-delimiter        Match consecutive delimiters as if it was one
    -p, --compress-delimiter      Print only the first delimiter of a sequence
    -s, --only-delimited          Print only lines containing the delimiter
    -V, --version                 Print version information
    -z, --zero-terminated         Line delimiter is NUL (\0), not LF (\n)
    -h, --help                    Print this help and exit
    -m, --complement              Invert fields (e.g. '2' becomes '1,3:')
    -j, --(no-)join               Print selected parts with delimiter in between
    --json                        Print fields as a JSON array of strings
    --no-mmap                     Disable memory mapping

OPTIONS:
    -f, --fields <bounds>         Fields to keep, 1-indexed, comma separated.
                                  Use colon (:) to match a range (inclusive).
                                  Use equal (=) to apply out of bound fallback.
                                  Fields can be negative (-1 is the last field).
                                  [default: 1:]

                                  e.g. cutting the string 'a-b-c-d' on '-'
                                    -f 1     => a
                                    -f 1:    => a-b-c-d
                                    -f 1:3   => a-b-c
                                    -f 3,2   => cb
                                    -f 3,1:2 => ca-b
                                    -f -3:-2 => b-c
                                    -f 1,8=fallback => afallback

                                  To re-apply the delimiter add -j, to replace
                                  it add -r (followed by the new delimiter).

                                  You can also format the output using {} syntax
                                  e.g.
                                    -f '({1}, {2})' => (a, b)

                                  You can escape { and } using {{ and }}.

    -b, --bytes <bounds>          Same as --fields, but it keeps bytes
    -c, --characters <bounds>     Same as --fields, but it keeps characters
    -l, --lines <bounds>          Same as --fields, but it keeps lines
                                  Implies --join. To merge lines, use --no-join
    -d, --delimiter <delimiter>   Delimiter used by --fields to cut the text
                                  [default: \t]
    -e, --regex <some regex>      Use a regular expression as delimiter
    -r, --replace-delimiter <new> Replace the delimiter with the provided text.
                                  Implies --join
    -t, --trim <type>             Trim the delimiter (greedy). Valid values are
                                  (l|L)eft, (r|R)ight, (b|B)oth
        --fallback-oob <fallback> Generic fallback output for any field that
                                  cannot be found (oob stands for out of bound).
                                  It's overridden by any fallback assigned to a
                                  specific field (see -f for help)
    -M, --fixed-memory <size>     Read the input in chunks of <size> kilobytes.
                                  This allows to read lines arbitrarily large.
                                  Works only with single-byte delimiters,
                                  fields in ascending order, -z, -j, -r

Options precedence:
    --trim and --compress-delimiter are applied before --fields or similar

Memory consumption:
    --characters and --fields read and allocate memory one line at a time

    --lines allocate memory one line at a time as long as the requested fields
    are ordered and non-negative (e.g. -l 1,3:4,4,7), otherwise it allocates
    the whole input in memory (it also happens when -p or -m are being used)

    --bytes allocate the whole input in memory

    --fixed-memory will read the input in chunks of <size> kilobytes. This
    allows to read lines arbitrarily large. Works only with single-byte
    delimiters, fields in ascending order, -z, -j, -r

Colors:
    Help is displayed using colors. Colors will be suppressed in the
    following circumstances:
    - when the TERM environment variable is not set or set to "dumb"
    - when the NO_COLOR environment variable is set (regardless of value)
"#;

const BANNER: &str = "\
tuc 1.3.0 - Created by Riccardo Attilio Galli

Cut text (or bytes) where a delimiter matches, then keep the desired parts.

Some examples:

    $ echo \"a/b/c\" | tuc -d / -f 1,-1
    ac

    $ echo \"a/b/c\" | tuc -d / -f 2:
    b/c

    $ echo \"hello.bak\" | tuc -d . -f 'mv {1:} {1}'
    mv hello.bak hello

    $ printf \"a\\nb\\nc\\nd\\ne\" | tuc -l 2:-2
    b
    c
    d

Run `tuc --help` for more detailed information.
Send bug reports to: https://github.com/riquito/tuc/issues
";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        print_banner();
        return;
    }

    let mut opts = ParsedArgs::default();
    let mut positional: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "-h" || arg == "--help" {
            print_help();
            return;
        }
        if arg == "-V" || arg == "--version" {
            println!("tuc {VERSION}");
            return;
        }
        if arg.starts_with('-') {
            if let Some(err) = parse_flag(arg, &args, &mut i, &mut opts) {
                eprintln!("{err}");
                process::exit(1);
            }
        } else {
            positional = Some(arg.clone());
            i += 1;
        }
    }

    finalize_opts(&mut opts);

    let bounds_str = opts
        .bounds_raw
        .clone()
        .unwrap_or_else(|| "1:".to_string());
    let bounds = match parse_bounds(&bounds_str) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to parse '{bounds_str}': {e}");
            process::exit(1);
        }
    };

    let regex = if let Some(ref pat) = opts.regex {
        match Regex::new(pat) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
    } else {
        None
    };

    let options = Options {
        delimiter: opts.delimiter,
        regex,
        greedy: opts.greedy,
        compress: opts.compress,
        trim: opts.trim,
        complement: opts.complement,
        join: opts.join,
        replace_delimiter: opts.replace_delimiter,
        only_delimited: opts.only_delimited,
        zero_terminated: opts.zero_terminated,
        json: opts.json,
        fallback_oob: opts.fallback_oob,
        bounds,
        bounds_raw: bounds_str,
        mode: opts.mode.unwrap_or(CutMode::Fields),
        lines_no_join: opts.lines_no_join,
    };

    let mut input: Box<dyn Read> = if let Some(path) = positional {
        match File::open(&path) {
            Ok(f) => Box::new(f),
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
    } else {
        Box::new(io::stdin())
    };

    let mut reader = BufReader::new(&mut input);
    let mut stdout = io::stdout();
    if let Err(e) = run(&mut reader, &mut stdout, &options) {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

#[derive(Default)]
struct ParsedArgs {
    delimiter: String,
    regex: Option<String>,
    greedy: bool,
    compress: bool,
    trim: Option<TrimMode>,
    complement: bool,
    join: bool,
    lines_no_join: bool,
    replace_delimiter: Option<String>,
    only_delimited: bool,
    zero_terminated: bool,
    json: bool,
    fallback_oob: Option<String>,
    bounds_raw: Option<String>,
    mode: Option<CutMode>,
}

fn parse_flag(arg: &str, args: &[String], i: &mut usize, opts: &mut ParsedArgs) -> Option<String> {
    match arg {
        "-g" | "--greedy-delimiter" => opts.greedy = true,
        "-p" | "--compress-delimiter" => opts.compress = true,
        "-s" | "--only-delimited" => opts.only_delimited = true,
        "-z" | "--zero-terminated" => opts.zero_terminated = true,
        "-m" | "--complement" => opts.complement = true,
        "-j" | "--join" => opts.join = true,
        "--no-join" => opts.lines_no_join = true,
        "--json" => opts.json = true,
        "--no-mmap" => {}
        "-f" | "--fields" => {
            *i += 1;
            opts.bounds_raw = Some(args.get(*i)?.clone());
            opts.mode = Some(CutMode::Fields);
        }
        "-b" | "--bytes" => {
            *i += 1;
            opts.bounds_raw = Some(args.get(*i)?.clone());
            opts.mode = Some(CutMode::Bytes);
        }
        "-c" | "--characters" => {
            *i += 1;
            opts.bounds_raw = Some(args.get(*i)?.clone());
            opts.mode = Some(CutMode::Characters);
        }
        "-l" | "--lines" => {
            *i += 1;
            opts.bounds_raw = Some(args.get(*i)?.clone());
            opts.mode = Some(CutMode::Lines);
            opts.join = true;
        }
        "-d" | "--delimiter" => {
            *i += 1;
            opts.delimiter = unescape(args.get(*i)?);
        }
        "-e" | "--regex" => {
            *i += 1;
            opts.regex = Some(args.get(*i)?.clone());
        }
        "-r" | "--replace-delimiter" => {
            *i += 1;
            opts.replace_delimiter = Some(unescape(args.get(*i)?));
            opts.join = true;
        }
        "-t" | "--trim" => {
            *i += 1;
            let v = args.get(*i)?;
            opts.trim = Some(match v.as_str() {
                "l" | "L" | "left" | "Left" => TrimMode::Left,
                "r" | "R" | "right" | "Right" => TrimMode::Right,
                "b" | "B" | "both" | "Both" => TrimMode::Both,
                _ => {
                    return Some(format!(
                        "failed to parse '{v}': Valid trim values are (l|L)eft, (r|R)ight, (b|B)oth"
                    ));
                }
            });
        }
        "--fallback-oob" => {
            *i += 1;
            opts.fallback_oob = Some(args.get(*i)?.clone());
        }
        "-M" | "--fixed-memory" => {
            *i += 1;
        }
        other if other.starts_with("--fields=") => {
            opts.bounds_raw = Some(other["--fields=".len()..].to_string());
            opts.mode = Some(CutMode::Fields);
        }
        other if other.starts_with("-f=") => {
            opts.bounds_raw = Some(other[3..].to_string());
            opts.mode = Some(CutMode::Fields);
        }
        other if other.starts_with("--delimiter=") => {
            opts.delimiter = unescape(&other["--delimiter=".len()..]);
        }
        other if other.starts_with("-d") && other.len() > 2 => {
            opts.delimiter = unescape(&other[2..]);
        }
        other if other.starts_with("-l=") => {
            opts.bounds_raw = Some(other[3..].to_string());
            opts.mode = Some(CutMode::Lines);
            opts.join = true;
        }
        other if other.starts_with("--lines=") => {
            opts.bounds_raw = Some(other["--lines=".len()..].to_string());
            opts.mode = Some(CutMode::Lines);
            opts.join = true;
        }
        _ => {}
    }
    *i += 1;
    None
}

fn finalize_opts(opts: &mut ParsedArgs) {
    if opts.delimiter.is_empty() && opts.regex.is_none() {
        opts.delimiter = "\t".to_string();
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('0') => out.push('\0'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn should_use_color() -> bool {
    if env::var("NO_COLOR").is_ok() {
        return false;
    }
    match env::var("TERM") {
        Ok(v) if v == "dumb" => false,
        Ok(_) => io::stdout().is_terminal(),
        Err(_) => false,
    }
}

fn print_help() {
    if should_use_color() {
        let mut out = StandardStream::stdout(ColorChoice::Always);
        let _ = write!(out, "{}", colorize_help(HELP));
    } else {
        print!("{HELP}");
    }
}

fn colorize_help(help: &str) -> String {
    help.to_string()
}

fn print_banner() {
    print!("{BANNER}");
}

trait IsTerminal {
    fn is_terminal(&self) -> bool;
}

impl IsTerminal for io::Stdout {
    fn is_terminal(&self) -> bool {
        atty::is(atty::Stream::Stdout)
    }
}

mod atty {
    pub enum Stream {
        Stdout,
    }
    pub fn is(_: Stream) -> bool {
        std::env::var("TERM").map(|t| t != "dumb").unwrap_or(false)
    }
}
