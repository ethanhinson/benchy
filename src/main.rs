mod bounds;
mod cutter;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use bounds::{parse_bounds, parse_format_spec};
use cutter::{process_bytes, process_characters, process_fields, process_lines, CutConfig};

const VERSION: &str = "1.3.0";

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

const HELP: &str = "\
tuc 1.3.0
Cut text (or bytes) where a delimiter matches, then keep the desired parts.

USAGE:
    tuc [FLAGS] [OPTIONS] < input
    tuc [FLAGS] [OPTIONS] filepath

FLAGS:
    -g, --greedy-delimiter        Match consecutive delimiters as if it was one
    -p, --compress-delimiter      Print only the first delimiter of a sequence
    -s, --only-delimited          Print only lines containing the delimiter
    -V, --version                 Print version information
    -z, --zero-terminated         Line delimiter is NUL (\\0), not LF (\\n)
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
                                  [default: \\t]
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
    - when the TERM environment variable is not set or set to \"dumb\"
    - when the NO_COLOR environment variable is set (regardless of value)
";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CutMode {
    Fields,
    Bytes,
    Characters,
    Lines,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrimMode {
    Left,
    Right,
    Both,
}

struct Options {
    cut_mode: CutMode,
    bounds_raw: String,
    delimiter: String,
    regex: Option<String>,
    join: bool,
    no_join: bool,
    complement: bool,
    greedy: bool,
    compress: bool,
    only_delimited: bool,
    zero_terminated: bool,
    replace_delimiter: Option<String>,
    trim: Option<TrimMode>,
    json: bool,
    fallback_oob: Option<String>,
    no_mmap: bool,
    fixed_memory: Option<usize>,
    filepath: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            cut_mode: CutMode::Fields,
            bounds_raw: "1:".to_string(),
            delimiter: "\t".to_string(),
            regex: None,
            join: false,
            no_join: false,
            complement: false,
            greedy: false,
            compress: false,
            only_delimited: false,
            zero_terminated: false,
            replace_delimiter: None,
            trim: None,
            json: false,
            fallback_oob: None,
            no_mmap: false,
            fixed_memory: None,
            filepath: None,
        }
    }
}

fn decode_delimiter(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
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
            out.push(ch);
        }
    }
    out
}

fn parse_args() -> Result<Options, String> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        print!("{BANNER}");
        process::exit(0);
    }

    let mut opts = Options::default();
    let mut bounds_set = false;
    let mut i = 1;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                process::exit(0);
            }
            "-V" | "--version" => {
                println!("tuc {VERSION}");
                process::exit(0);
            }
            "-g" | "--greedy-delimiter" => opts.greedy = true,
            "-p" | "--compress-delimiter" => opts.compress = true,
            "-s" | "--only-delimited" => opts.only_delimited = true,
            "-z" | "--zero-terminated" => opts.zero_terminated = true,
            "-m" | "--complement" => opts.complement = true,
            "-j" | "--join" => opts.join = true,
            "--no-join" => opts.no_join = true,
            "--json" => opts.json = true,
            "--no-mmap" => opts.no_mmap = true,
            "-f" | "--fields" => {
                opts.cut_mode = CutMode::Fields;
                opts.bounds_raw = next_value(&args, &mut i, arg)?;
                bounds_set = true;
            }
            "-b" | "--bytes" => {
                opts.cut_mode = CutMode::Bytes;
                opts.bounds_raw = next_value(&args, &mut i, arg)?;
                bounds_set = true;
            }
            "-c" | "--characters" => {
                opts.cut_mode = CutMode::Characters;
                opts.bounds_raw = next_value(&args, &mut i, arg)?;
                bounds_set = true;
            }
            "-l" | "--lines" => {
                opts.cut_mode = CutMode::Lines;
                opts.bounds_raw = next_value(&args, &mut i, arg)?;
                bounds_set = true;
                if !opts.no_join {
                    opts.join = true;
                }
            }
            "-d" | "--delimiter" => {
                opts.delimiter = decode_delimiter(&next_value(&args, &mut i, arg)?);
            }
            "-e" | "--regex" => {
                opts.regex = Some(next_value(&args, &mut i, arg)?);
            }
            "-r" | "--replace-delimiter" => {
                opts.replace_delimiter = Some(decode_delimiter(&next_value(&args, &mut i, arg)?));
                opts.join = true;
            }
            "-t" | "--trim" => {
                let value = next_value(&args, &mut i, arg)?;
                opts.trim = Some(match value.as_str() {
                    "l" | "L" => TrimMode::Left,
                    "r" | "R" => TrimMode::Right,
                    "b" | "B" => TrimMode::Both,
                    _ => return Err(format!("Invalid trim type: {value}")),
                });
            }
            "--fallback-oob" => {
                opts.fallback_oob = Some(next_value(&args, &mut i, arg)?);
            }
            "-M" | "--fixed-memory" => {
                let value = next_value(&args, &mut i, arg)?;
                opts.fixed_memory = Some(
                    value
                        .parse()
                        .map_err(|_| format!("Invalid fixed memory size: {value}"))?,
                );
            }
            other if other.starts_with('-') => {
                return Err(format!("Unknown flag: {other}"));
            }
            other => {
                if opts.filepath.is_some() {
                    return Err(format!("Unexpected argument: {other}"));
                }
                opts.filepath = Some(other.to_string());
            }
        }
        i += 1;
    }

    let _ = bounds_set;
    Ok(opts)
}

fn next_value(args: &[String], i: &mut usize, flag: &str) -> Result<String, String> {
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| format!("Missing value for {flag}"))
}

fn read_input(opts: &Options) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    if let Some(path) = &opts.filepath {
        let mut file = fs::File::open(path)?;
        file.read_to_end(&mut data)?;
    } else {
        io::stdin().read_to_end(&mut data)?;
    }
    Ok(data)
}

fn main() {
    let opts = match parse_args() {
        Ok(o) => o,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    };

    if opts.regex.is_some() && opts.join && opts.replace_delimiter.is_none() {
        eprintln!("tuc: runtime error. Cannot use --regex and --join without --replace-delimiter");
        process::exit(1);
    }

    let data = match read_input(&opts) {
        Ok(d) => d,
        Err(err) => {
            eprintln!("Error: {err}");
            process::exit(1);
        }
    };

    let is_format = opts.bounds_raw.contains('{');
    let bounds = if is_format {
        None
    } else {
        match parse_bounds(&opts.bounds_raw) {
            Ok(b) => Some(b),
            Err(err) => {
                eprintln!("{err}");
                process::exit(1);
            }
        }
    };

    let format_spec = if is_format {
        match parse_format_spec(&opts.bounds_raw) {
            Ok(f) => Some(f),
            Err(err) => {
                eprintln!("{err}");
                process::exit(1);
            }
        }
    } else {
        None
    };

    let join = if opts.no_join {
        false
    } else {
        opts.join
    };

    let config = CutConfig {
        delimiter: opts.delimiter.clone(),
        regex: opts.regex.clone(),
        join,
        line_mode: opts.cut_mode == CutMode::Lines,
        complement: opts.complement,
        greedy: opts.greedy,
        compress: opts.compress,
        only_delimited: opts.only_delimited,
        replace_delimiter: opts.replace_delimiter.clone(),
        trim: opts.trim,
        json: opts.json,
        fallback_oob: opts.fallback_oob.clone(),
        bounds: bounds.clone(),
        format_spec: format_spec.clone(),
    };

    let result = match opts.cut_mode {
        CutMode::Fields => process_fields(&data, &config, opts.zero_terminated),
        CutMode::Bytes => process_bytes(&data, &config),
        CutMode::Characters => process_characters(&data, &config, opts.zero_terminated),
        CutMode::Lines => process_lines(&data, &config, opts.zero_terminated),
    };

    match result {
        Ok(output) => {
            print!("{output}");
        }
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    }
}
