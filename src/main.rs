use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

use args::Cli;
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use printer::HexPrinter;
use units::parse_byte_count;

mod args;
mod printer;
mod units;

fn preprocess_args(args: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    for arg in args {
        if arg == "-e" {
            out.push("--endianness=little".to_string());
        } else {
            out.push(arg);
        }
    }
    out
}

fn main() {
    let cli = Cli::parse_from(preprocess_args(std::env::args().collect()));

    if let Some(shell) = cli.completion {
        let mut cmd = Cli::command();
        generate(shell, &mut cmd, "hexyl", &mut io::stdout());
        return;
    }

    if cli.print_color_table {
        printer::print_color_table();
        return;
    }

    let length = cli
        .effective_length()
        .map(|s| parse_byte_count(s, cli.block_size))
        .transpose()
        .unwrap_or_else(|e| die(&e));

    let skip = cli
        .skip
        .as_deref()
        .map(|s| parse_byte_count(s, cli.block_size))
        .transpose()
        .unwrap_or_else(|e| die(&e))
        .unwrap_or(0);

    let display_offset = cli
        .display_offset
        .as_deref()
        .map(|s| parse_byte_count(s, cli.block_size))
        .transpose()
        .unwrap_or_else(|e| die(&e))
        .unwrap_or(0);

    let mut data = Vec::new();

    if let Some(path) = &cli.file {
        let mut file = File::open(path).unwrap_or_else(|e| die(&e.to_string()));
        let total = file.metadata().ok().map(|m| m.len() as i64).unwrap_or(0);
        apply_skip(&mut file, skip, Some(total));
        read_limited(&mut file, length, &mut data);
    } else {
        let mut stdin = io::stdin();
        if skip > 0 {
            discard_bytes(&mut stdin, skip as u64);
        }
        read_limited(&mut stdin, length, &mut data);
    }

    if cli.include {
        printer::print_include(&cli.file, &data);
        return;
    }

    let mut printer = HexPrinter::new(cli);
    printer.print(&data, display_offset);
}

fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

fn apply_skip(file: &mut File, skip: i64, total: Option<i64>) {
    if skip == 0 {
        return;
    }
    let pos = if skip < 0 {
        total.map(|t| (t + skip).max(0)).unwrap_or(0) as u64
    } else {
        skip as u64
    };
    file.seek(SeekFrom::Start(pos))
        .unwrap_or_else(|e| die(&e.to_string()));
}

fn discard_bytes(r: &mut impl Read, mut n: u64) {
    let mut buf = [0u8; 8192];
    while n > 0 {
        let take = n.min(buf.len() as u64) as usize;
        let got = r.read(&mut buf[..take]).unwrap_or(0);
        if got == 0 {
            break;
        }
        n -= got as u64;
    }
}

fn read_limited(r: &mut impl Read, length: Option<i64>, out: &mut Vec<u8>) {
    if let Some(len) = length {
        let mut limited = r.take(len.max(0) as u64);
        limited.read_to_end(out).unwrap_or_else(|e| die(&e.to_string()));
    } else {
        r.read_to_end(out).unwrap_or_else(|e| die(&e.to_string()));
    }
}
