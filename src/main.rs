// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use clap::{ArgGroup, Parser, Subcommand};
use gdscheck::{load_gds, pdk::PdkConfig, report, run_drc};
use rayon::ThreadPoolBuilder;

/// gdscheck — Open Source DRC engine
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run DRC on a layout.
    Run(RunArgs),
    /// List the per-layer decks available in a PDK.
    ListDecks(PdkArgs),
    /// List the curated suites available in a PDK.
    ListSuites(PdkArgs),
    /// Print every rule defined in a deck.
    ShowDeck(ShowDeckArgs),
}

/// Arguments shared by the list-* commands.
#[derive(Parser, Debug)]
struct PdkArgs {
    /// PDK process name (e.g. ihp-sg13g2) or a path to a pdk.yml
    #[arg(short, long)]
    process: String,
}

#[derive(Parser, Debug)]
struct ShowDeckArgs {
    /// PDK process name (e.g. ihp-sg13g2) or a path to a pdk.yml
    #[arg(short, long)]
    process: String,

    /// Deck whose rules to print
    #[arg(short, long)]
    deck: String,
}

#[derive(Parser, Debug)]
#[command(group(ArgGroup::new("selection").required(true).multiple(false).args(["deck", "suite"])))]
struct RunArgs {
    /// Input GDS file to check (plain or gzip-compressed)
    #[arg(short, long)]
    input: String,

    /// PDK process name (e.g. ihp-sg13g2) or a path to a pdk.yml
    #[arg(short, long)]
    process: String,

    /// Deck(s) to run, comma-separated (e.g. `metal1,via1`); repeatable.
    /// Mutually exclusive with --suite.
    #[arg(short, long, value_delimiter = ',', group = "selection")]
    deck: Vec<String>,

    /// Suite to run — a curated rule selection (e.g. `main`, `precheck`).
    /// Mutually exclusive with --deck.
    #[arg(short, long, group = "selection")]
    suite: Option<String>,

    /// Top cell name to run DRC on
    #[arg(short, long)]
    topcell: String,

    /// Output DRC report file (.lyrdb)
    #[arg(short, long)]
    report: Option<String>,

    /// Number of threads to use (default: all available cores)
    #[arg(long, default_value_t = 0)]
    threads: usize,

    /// Disable electrical net extraction.  Net-aware checks (e.g. antenna ratios) are
    /// then skipped; geometry-only checks are unaffected.
    #[arg(long)]
    no_connectivity: bool,

    /// Print every violation's message, not just the per-rule counts.
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    match Cli::parse().command {
        Command::Run(args) => run(args),
        Command::ListDecks(args) => list(&args.process, ListKind::Decks),
        Command::ListSuites(args) => list(&args.process, ListKind::Suites),
        Command::ShowDeck(args) => show_deck(&args.process, &args.deck),
    }
}

enum ListKind {
    Decks,
    Suites,
}

fn load_pdk(process: &str) -> PdkConfig {
    match PdkConfig::for_process(process) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error loading PDK config: {e}");
            std::process::exit(1);
        }
    }
}

fn list(process: &str, kind: ListKind) {
    let pdk = load_pdk(process);
    let entries = match kind {
        ListKind::Decks => &pdk.decks,
        ListKind::Suites => &pdk.suites,
    };
    // Pad the name column so descriptions line up.
    let name_w = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    for e in entries {
        match &e.description {
            Some(desc) => println!("{:name_w$}  - {desc}", e.name),
            None => println!("{}", e.name),
        }
    }
}

fn show_deck(process: &str, deck: &str) {
    let pdk = load_pdk(process);
    let rules = match pdk.load_deck(deck) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error loading deck '{deck}': {e}");
            std::process::exit(1);
        }
    };

    let desc = pdk
        .decks
        .iter()
        .find(|d| d.name == deck)
        .and_then(|d| d.description.as_deref())
        .map(|d| format!(" — {d}"))
        .unwrap_or_default();
    println!("deck {deck}{desc}: {} rule(s)", rules.len());

    // Pre-format every field, then pad each column to its widest entry so values and
    // parameters line up regardless of how long the layer list is.
    let layer_list = |ls: &[gdscheck::pdk::Layer]| {
        ls.iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join(", ")
    };
    let cols: Vec<[String; 6]> = rules
        .iter()
        .map(|r| {
            let value = if r.value != 0.0 {
                format!("value={}", r.value)
            } else {
                String::new()
            };
            let params = if r.params.is_empty() {
                String::new()
            } else {
                // Sort params for stable output (HashMap order is non-deterministic).
                let mut p: Vec<(&String, &f64)> = r.params.iter().collect();
                p.sort_by(|a, b| a.0.cmp(b.0));
                let p: Vec<String> = p.iter().map(|(k, v)| format!("{k}={v}")).collect();
                format!("{{{}}}", p.join(", "))
            };
            let ignore = if r.ignore.is_empty() {
                String::new()
            } else {
                format!("ignore=[{}]", layer_list(&r.ignore))
            };
            [
                r.id.clone(),
                r.check.clone(),
                format!("[{}]", layer_list(&r.layers)),
                value,
                params,
                ignore,
            ]
        })
        .collect();

    // Width of each column except the last (trailing fields need no padding).
    let mut w = [0usize; 5];
    for row in &cols {
        for (i, width) in w.iter_mut().enumerate() {
            *width = (*width).max(row[i].len());
        }
    }
    for row in &cols {
        let line = format!(
            "  {:w0$}  {:w1$}  {:w2$}  {:w3$}  {:w4$}  {}",
            row[0], row[1], row[2], row[3], row[4], row[5],
            w0 = w[0], w1 = w[1], w2 = w[2], w3 = w[3], w4 = w[4],
        );
        println!("{}", line.trim_end());
    }
}

fn run(args: RunArgs) {
    ThreadPoolBuilder::new()
        .num_threads(args.threads) // 0 = rayon default (all logical cores)
        .build_global()
        .expect("Failed to build thread pool");

    let pdk = load_pdk(&args.process);
    println!("PDK: {} ({})", pdk.name, pdk.version);
    if let Some(suite) = &args.suite {
        println!("Suite: {suite}");
    } else {
        println!("Deck: {}", args.deck.join(","));
    }

    let lib = match load_gds(&args.input) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error reading GDS file: {e}");
            std::process::exit(1);
        }
    };

    println!("Library: {}", lib.name);

    let decks: Vec<&str> = args.deck.iter().map(String::as_str).collect();
    let start = std::time::Instant::now();
    let violations = match run_drc(
        &args.input,
        &args.process,
        &decks,
        args.suite.as_deref(),
        &args.topcell,
        !args.no_connectivity,
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error running DRC: {e}");
            std::process::exit(1);
        }
    };
    let elapsed = start.elapsed();

    println!("Topcell: {}", args.topcell);
    println!("DRC completed in {:.3}s", elapsed.as_secs_f64());

    if violations.is_empty() {
        println!("DRC clean.");
    } else if args.verbose {
        println!("{} violation(s):", violations.len());
        for v in &violations {
            println!("  [{}] {}", v.rule_id, v.message);
        }
    } else {
        println!("{} violation(s):", violations.len());
        let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for v in &violations {
            *counts.entry(v.rule_id.as_str()).or_insert(0) += 1;
        }
        for (rule_id, count) in counts {
            println!("  [{rule_id}] {count}");
        }
    }

    if let Some(report) = &args.report {
        match report::write_lyrdb(report, &args.topcell, &violations) {
            Ok(()) => println!("Report written to: {report}"),
            Err(e) => eprintln!("Error writing report: {e}"),
        }
    }
}
