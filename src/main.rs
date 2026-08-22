// Command pulse validates a network path with live probes, a compact RTT
// graph and an end-of-run health verdict.

mod checks;
mod probe;
mod stats;
mod ui;
mod verdict;

use std::process::ExitCode;
use std::time::Duration;

use ui::session::{self, Options};
use ui::summary::summary;

const VERSION: &str = match option_env!("PULSE_VERSION") {
    Some(v) => v,
    None => "dev",
};

fn main() -> ExitCode {
    let opts = match parse_args() {
        Ok(Some(opts)) => opts,
        Ok(None) => return ExitCode::SUCCESS, // --version
        Err(e) => {
            // Go's flag package prints these bare, then the usage text.
            if !e.is_empty() {
                eprintln!("{e}");
            }
            usage();
            return ExitCode::from(2);
        }
    };

    let s = match session::run(opts) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pulse: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(err) = &s.err {
        eprintln!("pulse: {err}");
        return ExitCode::FAILURE;
    }
    if let Some(checks) = &s.checks {
        print!("{}", summary(checks, &s.tracker, &s.kind, s.elapsed()));
    }
    ExitCode::SUCCESS
}

fn parse_args() -> Result<Option<Options>, String> {
    let mut count = 0usize;
    let mut interval = Duration::from_secs(1);
    let mut timeout = Duration::from_secs(2);
    let mut port: u16 = 443;
    let mut tcp = false;
    let mut host: Option<String> = None;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        // Accept Go's flag spellings: -x val, -x=val, --x val, --x=val.
        let (name, inline) = match arg.strip_prefix('-') {
            None => {
                if host.replace(arg).is_some() {
                    return Err(String::new());
                }
                i += 1;
                continue;
            }
            Some(rest) => {
                let rest = rest.strip_prefix('-').unwrap_or(rest);
                match rest.split_once('=') {
                    Some((n, v)) => (n.to_string(), Some(v.to_string())),
                    None => (rest.to_string(), None),
                }
            }
        };

        macro_rules! value {
            () => {{
                match inline {
                    Some(v) => v,
                    None => {
                        i += 1;
                        args.get(i)
                            .cloned()
                            .ok_or_else(|| format!("flag needs an argument: -{name}"))?
                    }
                }
            }};
        }

        match name.as_str() {
            "tcp" => tcp = true,
            "version" => {
                println!("pulse {VERSION}");
                return Ok(None);
            }
            "h" | "help" => return Err(String::new()),
            "c" => {
                let v = value!();
                count = v.parse().map_err(|_| bad_value(&v, "c"))?;
            }
            "p" => {
                let v = value!();
                port = v.parse().map_err(|_| bad_value(&v, "p"))?;
            }
            "i" => {
                let v = value!();
                interval = parse_duration(&v).ok_or_else(|| bad_value(&v, "i"))?;
            }
            "t" => {
                let v = value!();
                timeout = parse_duration(&v).ok_or_else(|| bad_value(&v, "t"))?;
            }
            other => return Err(format!("flag provided but not defined: -{other}")),
        }
        i += 1;
    }

    let host = host.ok_or_else(String::new)?;
    Ok(Some(Options {
        host,
        port,
        interval,
        timeout,
        count,
        tcp,
    }))
}

/// Phrased like Go's flag package.
fn bad_value(v: &str, flag: &str) -> String {
    format!("invalid value {v:?} for flag -{flag}: parse error")
}

/// Parses a Go-style duration such as "200ms", "1s" or "1.5s".
fn parse_duration(s: &str) -> Option<Duration> {
    let split = s.find(|c: char| c.is_ascii_alphabetic() || c == 'µ')?;
    let (num, unit) = s.split_at(split);
    let n: f64 = num.parse().ok()?;
    if n < 0.0 {
        return None;
    }
    let secs = match unit {
        "ns" => n / 1e9,
        "us" | "µs" => n / 1e6,
        "ms" => n / 1e3,
        "s" => n,
        "m" => n * 60.0,
        "h" => n * 3600.0,
        _ => return None,
    };
    Some(Duration::from_secs_f64(secs))
}

fn usage() {
    eprint!(
        r#"pulse — validate a connection, watch it live, get a verdict

usage:
  pulse [flags] <host>

examples:
  pulse google.com          probe with icmp, checks on :443
  pulse -c 60 1.1.1.1       stop after 60 probes
  pulse --tcp -p 22 host    probe a specific tcp port

flags:
  -c int
    	stop after this many probes (default: until interrupted)
  -i duration
    	time between probes (default 1s)
  -p string
    	port for tcp probes and checks (default "443")
  -t duration
    	per-probe timeout (default 2s)
  -tcp
    	probe with tcp connects instead of icmp
  -version
    	print version and exit
"#
    );
}

#[cfg(test)]
mod tests {
    use super::parse_duration;
    use std::time::Duration;

    #[test]
    fn parses_go_durations() {
        assert_eq!(parse_duration("1s"), Some(Duration::from_secs(1)));
        assert_eq!(parse_duration("200ms"), Some(Duration::from_millis(200)));
        assert_eq!(parse_duration("1.5s"), Some(Duration::from_millis(1500)));
        assert_eq!(parse_duration("2m"), Some(Duration::from_secs(120)));
        assert_eq!(parse_duration("500us"), Some(Duration::from_micros(500)));
        assert_eq!(parse_duration("5"), None, "unit is required");
        assert_eq!(parse_duration("abc"), None);
        assert_eq!(parse_duration("-1s"), None);
    }
}
