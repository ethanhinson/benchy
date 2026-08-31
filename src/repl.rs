use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::error::EvaError;
use crate::eval::{AngleUnit, Evaluator};
use crate::format::format_output;
use crate::parser::evaluate_expression;

pub struct Config {
    pub fix: usize,
    pub base: u32,
    pub angle_unit: AngleUnit,
}

pub fn run_repl(config: &Config) -> Result<(), EvaError> {
    let history_path = history_file();
    ensure_history_dir(&history_path);

    if history_path.exists() {
        // history loaded implicitly on prior runs
    } else {
        println!("No previous history.");
    }

    let mut evaluator = Evaluator::new(config.angle_unit);
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        write!(stdout, "> ").map_err(|e| EvaError::parser(e.to_string()))?;
        stdout.flush().map_err(|e| EvaError::parser(e.to_string()))?;

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(err) => return Err(EvaError::parser(err.to_string())),
        }

        let trimmed = line.trim_end_matches(['\n', '\r']).to_string();
        append_history_line(&trimmed);

        match evaluate_expression(&trimmed, &mut evaluator) {
            Ok(value) => {
                let out = format_output(value, config.base, config.fix);
                writeln!(stdout, "{out}").map_err(|e| EvaError::parser(e.to_string()))?;
                evaluator.last_answer = value;
            }
            Err(err) => {
                writeln!(stdout, "{err}").map_err(|e| EvaError::parser(e.to_string()))?;
            }
        }
    }

    Ok(())
}

pub fn run_command(input: &str, config: &Config) -> Result<(), EvaError> {
    let mut evaluator = Evaluator::new(config.angle_unit);
    let value = evaluate_expression(input, &mut evaluator)?;
    println!("{}", format_output(value, config.base, config.fix));
    Ok(())
}

fn history_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".local/share/eva/history.txt")
}

fn ensure_history_dir(path: &PathBuf) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

fn append_history_line(line: &str) {
    let path = history_file();
    ensure_history_dir(&path);

    let mut contents = if path.exists() {
        fs::read_to_string(&path).unwrap_or_default()
    } else {
        "#V2\n".to_string()
    };

    if !contents.starts_with("#V2") {
        contents = format!("#V2\n{contents}");
    }
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(line);
    contents.push('\n');
    let _ = fs::write(path, contents);
}
