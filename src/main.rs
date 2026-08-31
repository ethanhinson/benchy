use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use clap::Parser as ClapParser;
use eva::{
    balance_parentheses, eval, format_value, parse_and_validate, AngleUnit, Context,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, ClapParser)]
#[command(
    name = "eva",
    version = VERSION,
    about = "Calculator REPL similar to bc(1)",
    disable_version_flag = true
)]
struct Cli {
    #[arg(short, long, default_value_t = 10, value_parser = clap::value_parser!(u8).range(1..=64))]
    fix: u8,

    #[arg(short, long, default_value_t = 10, value_parser = clap::value_parser!(u8).range(1..=36))]
    base: u8,

    #[arg(short = 'a', long = "angle_unit", default_value = "degree")]
    angle_unit: AngleUnitArg,

    #[arg(short = 'h', long = "help")]
    help: bool,

    #[arg(short = 'V', long = "version")]
    version: bool,

    #[arg(value_name = "INPUT")]
    input: Option<String>,
}

#[derive(Clone, Debug)]
enum AngleUnitArg {
    Degree,
    Radian,
    Gradian,
}

impl std::str::FromStr for AngleUnitArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "degree" => Ok(Self::Degree),
            "radian" => Ok(Self::Radian),
            "gradian" => Ok(Self::Gradian),
            other => Err(format!("invalid value '{other}'")),
        }
    }
}

impl From<AngleUnitArg> for AngleUnit {
    fn from(value: AngleUnitArg) -> Self {
        match value {
            AngleUnitArg::Degree => AngleUnit::Degree,
            AngleUnitArg::Radian => AngleUnit::Radian,
            AngleUnitArg::Gradian => AngleUnit::Gradian,
        }
    }
}

fn history_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("eva")
            .join("history.txt")
    })
}

fn load_history(path: &PathBuf) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|content| {
            content
                .lines()
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn save_history(path: &PathBuf, history: &[String]) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::File::create(path) {
        for line in history {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn evaluate_expression(
    input: &str,
    ctx: &mut Context,
    fix: u8,
    base: u8,
    balance: bool,
) -> Result<String, String> {
    let prepared = if balance {
        balance_parentheses(input)
    } else {
        input.to_string()
    };

    let expr = parse_and_validate(&prepared).map_err(|e| format!("Parser Error: {e}"))?;
    let value = eval(&expr, ctx).map_err(|e| e.to_string())?;
    ctx.previous = value;
    Ok(format_value(value, fix as usize, base))
}

fn run_repl(fix: u8, base: u8, angle_unit: AngleUnit) -> io::Result<()> {
    let mut ctx = Context::new(angle_unit);
    let mut history = Vec::new();
    let history_file = history_path();

    if let Some(path) = &history_file {
        if path.exists() {
            history = load_history(path);
        } else {
            println!("No previous history.");
        }
    } else {
        println!("No previous history.");
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush()?;
        let mut line = String::new();
        let bytes = stdin.lock().read_line(&mut line)?;
        if bytes == 0 {
            println!();
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        history.push(trimmed.to_string());

        match evaluate_expression(trimmed, &mut ctx, fix, base, true) {
            Ok(output) => println!("{output}"),
            Err(err) => eprintln!("{err}"),
        }
    }

    if let Some(path) = history_file {
        save_history(&path, &history);
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    if cli.version {
        println!("eva {VERSION}");
        return;
    }

    if cli.help {
        let mut cmd = <Cli as clap::CommandFactory>::command();
        let mut help = Vec::new();
        cmd.write_help(&mut help).unwrap();
        let help_str = String::from_utf8(help).unwrap();
        let help_str = help_str.replace("eva", "executable");
        print!("{help_str}");
        return;
    }

    let angle_unit = AngleUnit::from(cli.angle_unit);

    if let Some(input) = cli.input {
        let mut ctx = Context::new(angle_unit);
        match evaluate_expression(&input, &mut ctx, cli.fix, cli.base, false) {
            Ok(output) => println!("{output}"),
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
    } else if let Err(err) = run_repl(cli.fix, cli.base, angle_unit) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod integration {
    use super::*;

    #[test]
    fn command_mode_addition() {
        let mut ctx = Context::new(AngleUnit::Degree);
        let out = evaluate_expression("1+1", &mut ctx, 10, 10, false).unwrap();
        assert_eq!(out, "2.0000000000");
    }
}
