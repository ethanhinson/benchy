mod error;
mod eval;
mod format;
mod parser;
mod repl;

use clap::Parser as ClapParser;

use error::EvaError;
use eval::AngleUnit;
use repl::{run_command, run_repl, Config};

#[derive(ClapParser, Debug)]
#[command(
    name = "executable",
    about = "Calculator REPL similar to bc(1)",
    disable_version_flag = true,
    disable_help_flag = true
)]
struct Args {
    #[arg(
        value_name = "INPUT",
        help = "Optional expression string to run eva in command mode"
    )]
    input: Option<String>,

    #[arg(
        short = 'f',
        long = "fix",
        default_value_t = 10,
        value_name = "FIX",
        help = "Number of decimal places in output (1 - 64)"
    )]
    fix: usize,

    #[arg(
        short = 'b',
        long = "base",
        default_value_t = 10,
        value_name = "RADIX",
        help = "Radix of calculation output (1 - 36)"
    )]
    base: u32,

    #[arg(
        short = 'a',
        long = "angle_unit",
        default_value = "degree",
        value_name = "angle_unit",
        help = "Angle unit",
        value_parser = ["degree", "radian", "gradian"]
    )]
    angle_unit: String,

    #[arg(short = 'h', long = "help", action = clap::ArgAction::Help, help = "Print help")]
    help: Option<bool>,

    #[arg(short = 'V', long = "version", help = "Print version")]
    version: bool,
}

fn parse_angle_unit(value: &str) -> Result<AngleUnit, EvaError> {
    match value {
        "degree" => Ok(AngleUnit::Degree),
        "radian" => Ok(AngleUnit::Radian),
        "gradian" => Ok(AngleUnit::Gradian),
        _ => Err(EvaError::parser(format!("Invalid angle unit {value}"))),
    }
}

fn main() {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(err) => {
            err.print().expect("print clap error");
            std::process::exit(err.exit_code());
        }
    };

    if args.version {
        println!("eva 0.3.1");
        return;
    }

    if args.fix < 1 || args.fix > 64 {
        eprintln!("fix must be between 1 and 64");
        std::process::exit(1);
    }
    if args.base < 1 || args.base > 36 {
        eprintln!("base must be between 1 and 36");
        std::process::exit(1);
    }

    let angle_unit = match parse_angle_unit(&args.angle_unit) {
        Ok(unit) => unit,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    let config = Config {
        fix: args.fix,
        base: args.base,
        angle_unit,
    };

    let result = if let Some(input) = args.input {
        run_command(&input, &config)
    } else {
        run_repl(&config)
    };

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
