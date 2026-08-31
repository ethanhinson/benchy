#[derive(Debug, Clone)]
pub struct Args {
    pub greedy_delimiter: bool,
    pub compress_delimiter: bool,
    pub only_delimited: bool,
    pub zero_terminated: bool,
    pub complement: bool,
    pub join: bool,
    pub no_join: bool,
    pub json: bool,
    pub no_mmap: bool,
    pub fields: Option<String>,
    pub bytes: Option<String>,
    pub characters: Option<String>,
    pub lines: Option<String>,
    pub delimiter: String,
    pub regex: Option<String>,
    pub replace_delimiter: Option<String>,
    pub trim: Option<char>,
    pub fallback_oob: Option<String>,
    pub fixed_memory: Option<u64>,
    pub file: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            greedy_delimiter: false,
            compress_delimiter: false,
            only_delimited: false,
            zero_terminated: false,
            complement: false,
            join: false,
            no_join: false,
            json: false,
            no_mmap: false,
            fields: None,
            bytes: None,
            characters: None,
            lines: None,
            delimiter: "\t".to_string(),
            regex: None,
            replace_delimiter: None,
            trim: None,
            fallback_oob: None,
            fixed_memory: None,
            file: None,
        }
    }
}

impl Args {
    pub fn parse(argv: &[String]) -> Result<Self, String> {
        let mut args = Args::default();
        let mut i = 1usize;
        while i < argv.len() {
            let a = &argv[i];
            match a.as_str() {
                "-g" | "--greedy-delimiter" => args.greedy_delimiter = true,
                "-p" | "--compress-delimiter" => args.compress_delimiter = true,
                "-s" | "--only-delimited" => args.only_delimited = true,
                "-z" | "--zero-terminated" => args.zero_terminated = true,
                "-m" | "--complement" => args.complement = true,
                "-j" | "--join" => args.join = true,
                "--no-join" => args.no_join = true,
                "--json" => args.json = true,
                "--no-mmap" => args.no_mmap = true,
                "-h" | "--help" => return Err("__HELP__".into()),
                "-V" | "--version" => return Err("__VERSION__".into()),
                "-f" | "--fields" => {
                    args.fields = Some(next_value(argv, &mut i)?);
                }
                "-b" | "--bytes" => {
                    args.bytes = Some(next_value(argv, &mut i)?);
                }
                "-c" | "--characters" => {
                    args.characters = Some(next_value(argv, &mut i)?);
                }
                "-l" | "--lines" => {
                    args.lines = Some(next_value(argv, &mut i)?);
                }
                "-d" | "--delimiter" => {
                    args.delimiter = next_value(argv, &mut i)?;
                }
                "-e" | "--regex" => {
                    args.regex = Some(next_value(argv, &mut i)?);
                }
                "-r" | "--replace-delimiter" => {
                    args.replace_delimiter = Some(next_value(argv, &mut i)?);
                }
                "-t" | "--trim" => {
                    let v = next_value(argv, &mut i)?;
                    let c = v.chars().next().ok_or("trim requires a value")?;
                    args.trim = Some(c);
                }
                "--fallback-oob" => {
                    args.fallback_oob = Some(next_value(argv, &mut i)?);
                }
                "-M" | "--fixed-memory" => {
                    let v = next_value(argv, &mut i)?;
                    args.fixed_memory = Some(v.parse().map_err(|_| "invalid fixed-memory")?);
                }
                other if other.starts_with('-') => {
                    return Err(format!("unexpected argument '{other}' found"));
                }
                other => {
                    args.file = Some(other.to_string());
                }
            }
            i += 1;
        }
        Ok(args)
    }

    pub fn mode_fields_spec(&self) -> Option<&String> {
        self.fields
            .as_ref()
            .or(self.bytes.as_ref())
            .or(self.characters.as_ref())
            .or(self.lines.as_ref())
    }

    pub fn is_lines(&self) -> bool {
        self.lines.is_some()
    }

    pub fn is_bytes(&self) -> bool {
        self.bytes.is_some()
    }

    pub fn is_characters(&self) -> bool {
        self.characters.is_some()
    }

    pub fn effective_join(&self) -> bool {
        if self.no_join {
            return false;
        }
        if self.join {
            return true;
        }
        if self.replace_delimiter.is_some() {
            return true;
        }
        if self.is_lines() {
            return true;
        }
        false
    }

    pub fn join_delimiter(&self) -> String {
        if self.is_lines() && self.effective_join() {
            return "\n".to_string();
        }
        if let Some(r) = &self.replace_delimiter {
            return r.clone();
        }
        self.delimiter.clone()
    }

    pub fn fields_spec_str(&self) -> String {
        if let Some(s) = self.mode_fields_spec() {
            return s.clone();
        }
        "1:".to_string()
    }
}

fn next_value(argv: &[String], i: &mut usize) -> Result<String, String> {
    if *i + 1 >= argv.len() {
        return Err("missing value for argument".into());
    }
    *i += 1;
    Ok(argv[*i].clone())
}
