mod error;
mod eval;
mod format;
mod parser;

use std::env;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process;

use eval::{AngleUnit, Evaluator};
use parser::{parse, prepare_input};

use crate::error::EvaError;
use crate::format::format_output;

struct Config {
    fix: u32,
    base: u32,
    angle_unit: String,
    input: Option<String>,
    help: bool,
    version: bool,
}

fn parse_args() -> Result<Config, String> {
    let mut args = env::args().skip(1);
    let mut fix = 10u32;
    let mut base = 10u32;
    let mut angle_unit = "degree".to_string();
    let mut input = None;
    let mut help = false;
    let mut version = false;

    while let Some(arg) = args.next() {
        if arg == "--" {
            input = args.next();
            break;
        }
        match arg.as_str() {
            "-h" | "--help" => help = true,
            "-V" | "--version" => version = true,
            "-f" | "--fix" => {
                let v = args
                    .next()
                    .ok_or("missing value for --fix")?
                    .parse::<u32>()
                    .map_err(|_| "invalid fix".to_string())?;
                if !(1..=64).contains(&v) {
                    return Err(format!(
                        "invalid value '{v}' for '--fix <FIX>': {v} is not in 1..=64"
                    ));
                }
                fix = v;
            }
            "-b" | "--base" => {
                let v = args
                    .next()
                    .ok_or("missing value for --base")?
                    .parse::<u32>()
                    .map_err(|_| "invalid base".to_string())?;
                if !(1..=36).contains(&v) {
                    return Err(format!(
                        "invalid value '{v}' for '--base <RADIX>': {v} is not in 1..=36"
                    ));
                }
                base = v;
            }
            "-a" | "--angle_unit" => {
                angle_unit = args.next().ok_or("missing value for --angle_unit")?;
                if !matches!(angle_unit.as_str(), "degree" | "radian" | "gradian") {
                    return Err(format!("invalid angle unit: {angle_unit}"));
                }
            }
            other if other.starts_with('-') => {
                return Err(format!(
                    "error: unexpected argument '{other}' found\n\n  tip: to pass '{other}' as a value, use '-- {other}'\n\nUsage: executable [OPTIONS] [INPUT]\n\nFor more information, try '--help'."
                ));
            }
            other => input = Some(other.to_string()),
        }
    }

    Ok(Config {
        fix,
        base,
        angle_unit,
        input,
        help,
        version,
    })
}

fn history_path() -> PathBuf {
    home_dir().join(".local/share/eva/history.txt")
}

fn home_dir() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

fn load_history() -> (Vec<String>, bool) {
    let path = history_path();
    if !path.exists() {
        return (Vec::new(), true);
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    let lines: Vec<String> = content
        .lines()
        .skip_while(|l| l.starts_with('#'))
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    (lines.clone(), lines.is_empty())
}

fn save_history(entries: &[String]) {
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut content = String::from("#V2\n");
    for entry in entries {
        content.push_str(entry);
        content.push('\n');
    }
    let _ = fs::write(path, content);
}

fn evaluate_line(
    input: &str,
    eval: &mut Evaluator,
    base: u32,
    fix: u32,
    repl: bool,
) -> Result<String, EvaError> {
    let prepared = prepare_input(input, repl);
    if prepared.is_empty() {
        return Err(EvaError::Parser(
            "Too many operators, too few operands".into(),
        ));
    }
    let expr = parse(&prepared)?;
    let value = eval.eval(&expr)?;
    eval.prev_answer = value;
    Ok(format_output(value, base, fix as usize))
}

fn print_help() {
    println!("Calculator REPL similar to bc(1)");
    println!();
    println!("Usage: executable [OPTIONS] [INPUT]");
    println!();
    println!("Arguments:");
    println!("  [INPUT]  Optional expression string to run eva in command mode");
    println!();
    println!("Options:");
    println!("  -f, --fix <FIX>                Number of decimal places in output (1 - 64) [default: 10]");
    println!("  -b, --base <RADIX>             Radix of calculation output (1 - 36) [default: 10]");
    println!("  -a, --angle_unit <angle_unit>  Angle unit [default: degree] [possible values: degree, radian, gradian]");
    println!("  -h, --help                     Print help");
    println!("  -V, --version                  Print version");
}

fn run_command(cfg: &Config) -> i32 {
    let mut eval = Evaluator::new(AngleUnit::from_str(&cfg.angle_unit));
    match evaluate_line(cfg.input.as_ref().unwrap(), &mut eval, cfg.base, cfg.fix, false) {
        Ok(out) => {
            println!("{out}");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn run_repl(cfg: &Config) -> i32 {
    let mut eval = Evaluator::new(AngleUnit::from_str(&cfg.angle_unit));
    let (mut history_entries, no_history) = load_history();

    if no_history {
        println!("No previous history.");
    }

    let stdin = io::stdin();
    loop {
        print!("> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                history_entries.push(trimmed.to_string());
                save_history(&history_entries);
                match evaluate_line(trimmed, &mut eval, cfg.base, cfg.fix, true) {
                    Ok(out) => println!("{out}"),
                    Err(e) => eprintln!("{e}"),
                }
            }
            Err(_) => break,
        }
    }
    0
}

fn run_piped(cfg: &Config) -> i32 {
    let mut eval = Evaluator::new(AngleUnit::from_str(&cfg.angle_unit));
    let (_, no_history) = load_history();
    if no_history {
        println!("No previous history.");
    }

    let stdin = io::stdin();
    let mut code = 0;
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        print!("> ");
        match evaluate_line(trimmed, &mut eval, cfg.base, cfg.fix, true) {
            Ok(out) => println!("{out}"),
            Err(e) => {
                eprintln!("{e}");
                code = 1;
            }
        }
    }
    code
}

fn main() {
    let cfg = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            process::exit(2);
        }
    };

    if cfg.help {
        print_help();
        return;
    }

    if cfg.version {
        println!("eva 0.3.1");
        return;
    }

    let code = if let Some(_) = cfg.input {
        run_command(&cfg)
    } else if !io::stdin().is_terminal() {
        run_piped(&cfg)
    } else {
        run_repl(&cfg)
    };

    process::exit(code);
}
