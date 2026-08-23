//! Drives an interactive run: owns the terminal, fans probe results into the
//! live view, and hands back the state the exit summary needs.

use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::checks::{self, Check, Checks};
use crate::probe::{self, icmp::Icmp, tcp::Tcp, Failure, Prober, Sample};
use crate::stats::Tracker;
use crate::ui::format::{fmt_loss, fmt_ms, fmt_secs, loss_style};
use crate::ui::graph::graph;
use crate::ui::screen::Screen;
use crate::ui::spinner::SPIN_FRAMES;
use crate::ui::styles::{BAD, BRAND, DIM, OK, VALUE, WARN};
use crate::ui::term::{self, RawMode};

const GRAPH_HEIGHT: usize = 4;
const MAX_GRAPH_WIDTH: usize = 60;
const SPIN_INTERVAL: Duration = Duration::from_millis(80);

/// Configures a pulse run.
#[derive(Clone)]
pub struct Options {
    pub host: String,
    pub port: u16,
    pub interval: Duration,
    pub timeout: Duration,
    /// 0 = until interrupted.
    pub count: usize,
    /// Force TCP probes instead of ICMP.
    pub tcp: bool,
}

/// The state of an interactive run, kept for the exit summary.
pub struct Session {
    opts: Options,
    pub checks: Option<Checks>,
    pub tracker: Tracker,
    pub kind: String,
    note: Option<String>,
    spin: usize,
    width: usize,
    start: Instant,
    /// Fatal: checks could not produce an address.
    pub err: Option<String>,
}

impl Session {
    /// The wall-clock duration of the session, rounded to seconds.
    pub fn elapsed(&self) -> Duration {
        Duration::from_secs(self.start.elapsed().as_secs_f64().round() as u64)
    }
}

enum Event {
    Key(u8),
    Tick,
    Checks(Box<Checks>),
    Sample(u64, Sample),
    Done(u64),
}

/// Runs the session until the probe count is reached or the user quits. It
/// owns the terminal for its whole lifetime.
pub fn run(opts: Options) -> io::Result<Session> {
    let fd = io::stdin().as_raw_fd();
    if !term::is_terminal(fd) {
        return Err(io::Error::other("needs an interactive terminal"));
    }

    let mut s = Session {
        opts: opts.clone(),
        checks: None,
        tracker: Tracker::new(),
        kind: String::new(),
        note: None,
        spin: 0,
        width: term::width(fd).unwrap_or(80) as usize,
        start: Instant::now(),
        err: None,
    };

    let _raw = RawMode::enable(fd)?; // restored on drop
    term::notify_resize();
    print!("\x1b[?25l"); // hide cursor
    let _ = io::stdout().flush();

    let (tx, rx) = mpsc::channel();
    spawn_keys(tx.clone());
    spawn_ticks(tx.clone());
    spawn_checks(tx.clone(), &opts);

    let mut scr = Screen::new(io::stdout());
    scr.paint(&view(&s));

    // Cancels the running prober when dropped or replaced.
    let mut stop: Option<Sender<()>> = None;
    let mut gen: u64 = 0;

    while let Ok(ev) = rx.recv() {
        match ev {
            Event::Key(k) => {
                // q, Q, esc, ctrl+c
                if matches!(k, b'q' | b'Q' | 0x1b | 0x03) {
                    break;
                }
            }

            Event::Tick => {
                if term::take_resize() {
                    if let Some(w) = term::width(fd) {
                        s.width = w as usize;
                    }
                    scr.paint(&view(&s));
                } else if s.tracker.sent() == 0 {
                    s.spin = (s.spin + 1) % SPIN_FRAMES.len();
                    scr.paint(&view(&s));
                }
            }

            Event::Checks(c) => {
                let c = *c;
                let ip = c.ip;
                s.checks = Some(c);
                match ip {
                    None => {
                        s.err = Some(format!(
                            "cannot reach {}: {}",
                            opts.host,
                            last_detail(s.checks.as_ref().unwrap())
                        ));
                        break;
                    }
                    Some(ip) => {
                        let p: Box<dyn Prober> = if opts.tcp {
                            Box::new(Tcp::new(ip, opts.port, opts.timeout))
                        } else {
                            Box::new(Icmp::new(ip, opts.timeout))
                        };
                        gen += 1;
                        s.kind = p.kind();
                        stop = Some(spawn_probes(tx.clone(), p, &opts, gen));
                        scr.paint(&view(&s));
                    }
                }
            }

            Event::Sample(g, sample) => {
                if g != gen {
                    continue; // from a prober we already replaced
                }
                // ICMP sockets unavailable: fall back to TCP transparently.
                if sample.seq == 0
                    && sample.failure == Some(Failure::Permission)
                    && s.kind == "icmp"
                {
                    drop(stop.take()); // cancel the ICMP prober
                    s.note = Some("icmp unavailable · falling back to tcp".into());
                    s.tracker = Tracker::new();
                    let ip = s.checks.as_ref().and_then(|c| c.ip).expect("checked above");
                    let p: Box<dyn Prober> = Box::new(Tcp::new(ip, opts.port, opts.timeout));
                    gen += 1;
                    s.kind = p.kind();
                    stop = Some(spawn_probes(tx.clone(), p, &opts, gen));
                    continue;
                }
                s.tracker.add(sample);
                scr.paint(&view(&s));
            }

            Event::Done(g) => {
                if g == gen {
                    break; // probe count reached
                }
            }
        }
    }

    print!("\x1b[?25h"); // show cursor
    let _ = io::stdout().flush();
    Ok(s)
}

fn spawn_keys(tx: Sender<Event>) {
    thread::spawn(move || {
        let mut stdin = io::stdin();
        let mut buf = [0u8; 1];
        loop {
            match stdin.read(&mut buf) {
                Ok(1) => {
                    if tx.send(Event::Key(buf[0])).is_err() {
                        return;
                    }
                }
                Ok(_) => return,
                Err(_) => return,
            }
        }
    });
}

fn spawn_ticks(tx: Sender<Event>) {
    thread::spawn(move || loop {
        thread::sleep(SPIN_INTERVAL);
        if tx.send(Event::Tick).is_err() {
            return;
        }
    });
}

fn spawn_checks(tx: Sender<Event>, opts: &Options) {
    let (host, port, timeout) = (opts.host.clone(), opts.port, opts.timeout);
    thread::spawn(move || {
        let c = checks::run_checks(&host, port, timeout);
        let _ = tx.send(Event::Checks(Box::new(c)));
    });
}

/// Starts probing; dropping the returned sender cancels it.
fn spawn_probes(tx: Sender<Event>, p: Box<dyn Prober>, opts: &Options, gen: u64) -> Sender<()> {
    let (stop_tx, stop_rx) = mpsc::channel();
    let (interval, count) = (opts.interval, opts.count);
    thread::spawn(move || {
        let (out_tx, out_rx) = mpsc::channel();
        let forward = tx.clone();
        let pump = thread::spawn(move || {
            while let Ok(s) = out_rx.recv() {
                if forward.send(Event::Sample(gen, s)).is_err() {
                    return;
                }
            }
        });
        probe::run(&*p, interval, count, &stop_rx, &out_tx);
        drop(out_tx);
        let _ = pump.join();
        let _ = tx.send(Event::Done(gen));
    });
    stop_tx
}

fn last_detail(c: &Checks) -> String {
    match c.items.last() {
        None => "checks failed".into(),
        Some(it) => format!("{}: {}", it.name, it.detail),
    }
}

/// Prefixes every line with the standard two-space margin.
fn indent(s: &str) -> String {
    format!("  {}", s.replace('\n', "\n  "))
}

fn view(s: &Session) -> String {
    let mut b = String::new();

    // header
    b.push('\n');
    let mut header = format!("{}  {}", BRAND.render("pulse"), s.opts.host);
    if let Some(ip) = s.checks.as_ref().and_then(|c| c.ip) {
        let ip = ip.to_string();
        if ip != s.opts.host {
            header.push_str(&DIM.render(&format!(" ({ip})")));
        }
    }
    if !s.kind.is_empty() {
        header.push_str(&DIM.render(&format!(
            " · {} · {}",
            s.kind,
            fmt_interval(s.opts.interval)
        )));
    }
    b.push_str(&indent(&header));
    b.push_str("\n\n");

    let Some(checks) = s.checks.as_ref() else {
        b.push_str(&indent(
            &DIM.render(&format!("{} running checks…", SPIN_FRAMES[s.spin])),
        ));
        b.push('\n');
        return b;
    };

    // graph
    let gw = s.width.saturating_sub(4).clamp(10, MAX_GRAPH_WIDTH);
    if s.tracker.sent() > 0 {
        b.push_str(&indent(&graph(&s.tracker.samples, gw, GRAPH_HEIGHT)));
        b.push_str("\n\n");
        b.push_str(&indent(&stats_line(s)));
        b.push_str("\n\n");
    } else {
        b.push_str(&indent(
            &DIM.render(&format!("{} probing…", SPIN_FRAMES[s.spin])),
        ));
        b.push_str("\n\n");
    }

    // checks
    for it in &checks.items {
        b.push_str(&indent(&check_line(it)));
        b.push('\n');
    }

    // footer
    b.push('\n');
    b.push_str(&indent(&footer(s)));
    b.push('\n');
    b
}

fn stats_line(s: &Session) -> String {
    let t = &s.tracker;
    let parts = [
        format!("{}{}", DIM.render("last "), VALUE.render(&fmt_ms(t.last()))),
        format!("{}{}", DIM.render("min "), VALUE.render(&fmt_ms(t.min()))),
        format!("{}{}", DIM.render("avg "), VALUE.render(&fmt_ms(t.avg()))),
        format!("{}{}", DIM.render("max "), VALUE.render(&fmt_ms(t.max()))),
        format!(
            "{}{}",
            DIM.render("jitter "),
            VALUE.render(&fmt_ms(t.jitter()))
        ),
        format!(
            "{}{}",
            DIM.render("loss "),
            loss_style(t.loss()).render(&fmt_loss(t.loss()))
        ),
    ];
    parts.join("   ")
}

fn footer(s: &Session) -> String {
    let mut f = DIM.render("q quit");
    let n = s.tracker.sent();
    if n > 0 {
        let mut count = n.to_string();
        if s.opts.count > 0 {
            count.push_str(&format!("/{}", s.opts.count));
        }
        f.push_str(&DIM.render(&format!(" · {count} probes · {}", fmt_secs(s.elapsed()))));
    }
    if let Some(note) = &s.note {
        f.push('\n');
        f.push_str(&WARN.render(note));
    }
    f
}

fn check_line(it: &Check) -> String {
    let icon = if it.skipped {
        DIM.render("–")
    } else if it.ok {
        OK.render("✓")
    } else {
        BAD.render("✗")
    };
    let name = DIM.render(&format!("{:<5}", it.name));
    let dur = if it.skipped {
        "      ".to_string()
    } else {
        format!("{:>4}ms", it.dur.as_millis())
    };
    let detail = if !it.ok && !it.skipped {
        BAD.render(&it.detail)
    } else {
        DIM.render(&it.detail)
    };
    format!("{icon} {name} {}  {detail}", VALUE.render(&dur))
}

/// Formats the probe interval the way Go's Duration.String does.
fn fmt_interval(d: Duration) -> String {
    if d.subsec_nanos() == 0 && d.as_secs() > 0 {
        fmt_secs(d)
    } else if d.as_millis() > 0 && d.subsec_nanos() % 1_000_000 == 0 {
        format!("{}ms", d.as_millis())
    } else {
        format!("{d:?}")
    }
}
