// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod helpers;
mod ihp_sg13cmos5l;
mod ihp_sg13g2;

use clap::Parser;
use gdscheck::pdk::PdkConfig;

#[derive(Parser, Debug)]
#[command(about = "Generate DRC test pattern GDS files")]
struct Args {
    /// PDK file (YAML) — selects which patterns to generate
    #[arg(short, long)]
    pdk: String,
}

fn main() {
    let args = Args::parse();

    let pdk = match PdkConfig::load(&args.pdk) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error loading PDK: {e}");
            std::process::exit(1);
        }
    };

    println!("Generating test patterns for: {} ({})", pdk.name, pdk.version);

    match pdk.name.as_str() {
        "IHP SG13G2" => ihp_sg13g2::generate(&pdk),
        "IHP SG13CMOS5L" => ihp_sg13cmos5l::generate(&pdk),
        other => {
            eprintln!("No generator available for PDK '{other}'");
            std::process::exit(1);
        }
    }
}
