mod bounds;
mod cli;
mod format;
mod engine;
mod split;

use cli::Args;
use engine::Processor;
use std::io::{self, BufRead, Read};
use std::process::exit;

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

const HELP: &str = include_str!("help.txt");

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() == 1 {
        print!("{BANNER}");
        return;
    }

    let parsed = match Args::parse(&argv) {
        Ok(a) => a,
        Err(e) if e == "__HELP__" => {
            print_help();
            return;
        }
        Err(e) if e == "__VERSION__" => {
            println!("tuc 1.3.0");
            return;
        }
        Err(e) => {
            eprintln!("error: {e}");
            exit(2);
        }
    };

    let processor = match Processor::new(parsed) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            exit(1);
        }
    };

    if let Err(e) = run(&processor) {
        eprintln!("Error: {e}");
        exit(1);
    }
}

fn print_help() {
    if should_color() {
        print!("{HELP}");
    } else {
        print!("{}", strip_ansi(HELP));
    }
}

fn should_color() -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    match std::env::var("TERM") {
        Ok(v) if v == "dumb" => false,
        Ok(_) => true,
        Err(_) => false,
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            while chars.next().is_some() {
                if chars.peek() == Some(&'m') {
                    chars.next();
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn run(processor: &Processor) -> Result<(), String> {
    let args = &processor.args;

    if let Some(path) = &args.file {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        if args.is_lines() || args.is_bytes() || args.is_characters() {
            if args.is_lines() {
                return write_whole_input(processor, &content);
            }
            for line in content.split('\n') {
                let line = line.strip_suffix('\r').unwrap_or(line);
                emit_line(processor, line)?;
            }
            return Ok(());
        }
        for line in content.lines() {
            emit_line(processor, line)?;
        }
        return Ok(());
    }

    let stdin = io::stdin();
    if args.is_lines() {
        let mut buf = String::new();
        stdin.lock().read_to_string(&mut buf).map_err(|e| e.to_string())?;
        return write_whole_input(processor, &buf);
    }

    if args.is_bytes() || args.is_characters() {
        let mut buf = String::new();
        stdin.lock().read_to_string(&mut buf).map_err(|e| e.to_string())?;
        let mut first = true;
        for line in buf.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if !first || !line.is_empty() || buf.ends_with('\n') {
                for out in processor.process_input(line)? {
                    println!("{out}");
                }
            }
            first = false;
        }
        if !buf.contains('\n') {
            for out in processor.process_input(buf.trim_end_matches('\n'))? {
                println!("{out}");
            }
        }
        return Ok(());
    }

    let reader = stdin.lock();
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        emit_line(processor, &line)?;
    }
    Ok(())
}

fn write_whole_input(processor: &Processor, content: &str) -> Result<(), String> {
    let args = &processor.args;
    let trimmed = content.strip_suffix('\n').unwrap_or(content);
    for out in processor.process_input(trimmed)? {
        if args.no_join {
            print!("{out}");
        } else {
            println!("{out}");
        }
    }
    Ok(())
}

fn emit_line(processor: &Processor, line: &str) -> Result<(), String> {
    let out = processor.process_line_stream(line)?;
    if !out.is_empty() || !processor.args.only_delimited {
        println!("{out}");
    }
    Ok(())
}
