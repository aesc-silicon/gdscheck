// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod cache;
pub mod checks;
pub mod connectivity;
pub mod flatten;
pub mod layout;
pub mod merge;
pub mod pdk;
pub mod report;
pub mod violation;

use flate2::read::GzDecoder;
use gds21::GdsLibrary;
use std::io::Read;

pub use violation::Violation;

pub fn load_gds(path: &str) -> Result<GdsLibrary, Box<dyn std::error::Error>> {
    let raw = std::fs::read(path)?;

    let bytes = if raw.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(raw.as_slice());
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        decompressed
    } else {
        raw
    };

    Ok(GdsLibrary::from_bytes(&bytes)?)
}

/// Checks that need electrical connectivity (net extraction).  When connectivity is
/// disabled (`connectivity == false`) these are skipped rather than run on no nets.
/// Populated as net-aware checks land (e.g. the antenna ratio rules).
pub const NET_AWARE_CHECKS: &[&str] = &["antenna_ratio", "gate_connected_min_area"];

/// Parse a lazy virtual layer's `op` string to a [`merge::VirtualOp`], converting its
/// radius (µm) to DBU where the op takes one.  An unsupported op or a missing radius is
/// an error: the layer would otherwise silently register as empty and every rule
/// referencing it would become a no-op false-clean.
fn parse_virtual_op(
    op: &str,
    radius: Option<f64>,
    dbu_to_um: f64,
) -> Result<merge::VirtualOp, String> {
    use merge::VirtualOp::*;
    let radius_dbu = || {
        radius
            .map(|r| (r / dbu_to_um).round() as i32)
            .ok_or_else(|| format!("op '{op}' requires a radius"))
    };
    Ok(match op {
        "union" => Union,
        "intersection" | "and" => Intersection,
        "difference" | "not" => Difference,
        "square" => Square,
        "not_square" => NotSquare,
        "interacting" => Interacting,
        "not_interacting" => NotInteracting,
        "covering" => Covering,
        "not_circle_or_octagon" => NotCircleOrOctagon,
        "not_circle" => NotCircle,
        "holes" => Holes,
        "with_holes" => WithHoles,
        "with_text" => WithText,
        "close" => Close(radius_dbu()?),
        "open" => Open(radius_dbu()?),
        "grow" => Grow(radius_dbu()?),
        other => return Err(format!("unsupported op '{other}'")),
    })
}

/// Run DRC for a selection of rules: either one or more decks (`decks`, suite-free
/// per-layer rule files) or exactly one `suite` (a curated rule selection). The two are
/// mutually exclusive — `suite` takes precedence if both are somehow supplied, and it is
/// an error to supply neither.
pub fn run_drc(
    gds_path: &str,
    process: &str,
    decks: &[&str],
    suite: Option<&str>,
    topcell: &str,
    connectivity: bool,
) -> Result<Vec<Violation>, String> {
    let pdk = pdk::PdkConfig::for_process(process).map_err(|e| e.to_string())?;
    let rules = if let Some(suite) = suite {
        pdk.load_suite(suite).map_err(|e| e.to_string())?
    } else if decks.is_empty() {
        return Err("no deck or suite selected".into());
    } else {
        let mut rules = Vec::new();
        for deck in decks {
            rules.extend(pdk.load_deck(deck).map_err(|e| e.to_string())?);
        }
        rules
    };

    // Lazy (tiled) virtual layers: built per tile in the merge cache rather than
    // materialised in the layout.  A whole-layout check (inside_boundary) therefore
    // cannot see them, so reject that combination up front rather than report wrong.
    let tiled_virtuals = pdk.tiled_virtual_layers();
    let lazy_keys: std::collections::HashSet<(i16, i16)> =
        tiled_virtuals.iter().map(|v| v.key).collect();
    for rule in &rules {
        if ALL_LAYER_CHECKS.contains(&rule.check.as_str()) {
            for l in rule.layers.iter().chain(rule.ignore.iter()) {
                if lazy_keys.contains(&(l.gds_layer as i16, l.gds_datatype as i16)) {
                    return Err(format!(
                        "Rule '{}' ({}) references a lazy virtual layer, which is not \
                         materialised for whole-layout checks; mark it `mode: global`",
                        rule.id, rule.check
                    ));
                }
            }
        }
    }

    let lib = load_gds(gds_path).map_err(|e| e.to_string())?;

    if !lib.structs.iter().any(|s| s.name == topcell) {
        return Err(format!("Topcell '{topcell}' not found in library"));
    }

    let dbu_to_um = lib.units.1 * 1e6;

    // Resolve each lazy virtual's op string once (radii converted to DBU); a typo'd
    // op or a missing radius is a config error, not a silently empty layer.
    let tiled_virtuals: Vec<(pdk::TiledVirtualSpec, merge::VirtualOp)> = tiled_virtuals
        .into_iter()
        .map(|spec| {
            let op = parse_virtual_op(&spec.op, spec.radius, dbu_to_um)
                .map_err(|e| format!("Lazy virtual layer '{}': {e}", spec.name))?;
            Ok((spec, op))
        })
        .collect::<Result<_, String>>()?;

    // Flatten the cell hierarchy into a FlatLayout indexed by layer/datatype so
    // GdsStructRef/GdsArrayRef instances are visible to every check.  Restrict the
    // flatten to the layers the deck actually touches — a large hierarchy is far
    // too big to instantiate in full.  `inside_boundary` inspects *every* layer,
    // so any deck using it must flatten everything.
    const ALL_LAYER_CHECKS: &[&str] = &["inside_boundary"];
    let needed: Option<std::collections::HashSet<(i16, i16)>> =
        if rules.iter().any(|r| ALL_LAYER_CHECKS.contains(&r.check.as_str())) {
            None
        } else {
            let mut n: std::collections::HashSet<(i16, i16)> = std::collections::HashSet::new();
            for rule in &rules {
                for l in rule.layers.iter().chain(rule.ignore.iter()) {
                    n.insert((l.gds_layer as i16, l.gds_datatype as i16));
                }
                if let Some(&bl) = rule.params.get("boundary_layer") {
                    let dt = rule.params.get("boundary_datatype").copied().unwrap_or(0.0);
                    n.insert((bl as i16, dt as i16));
                }
            }
            // Net extraction (if it will run) reads the connect-graph layers, which the
            // rules themselves may not name — pull them in so they are flattened too.
            if connectivity && rules.iter().any(|r| NET_AWARE_CHECKS.contains(&r.check.as_str())) {
                for spec in &pdk.connectivity {
                    n.insert(spec.connector);
                    n.extend(spec.layers.iter().copied());
                }
            }
            // A referenced virtual layer is built from its source layers, which must
            // therefore be flattened too — transitively, since a virtual layer may feed
            // another (e.g. ContOnActiv → ContSquare → ContNoSealring → Cont/EdgeSeal).
            // Iterate to a fixpoint so every layer in the chain is pulled in.
            loop {
                let mut added = false;
                for vl in &pdk.virtual_layers {
                    let Some(vlayer) = pdk.layer(&vl.name) else { continue };
                    if !n.contains(&(vlayer.gds_layer as i16, vlayer.gds_datatype as i16)) {
                        continue;
                    }
                    for src in &vl.layers {
                        if let Some(s) = pdk.layer(src) {
                            added |= n.insert((s.gds_layer as i16, s.gds_datatype as i16));
                        }
                    }
                }
                if !added {
                    break;
                }
            }
            Some(n)
        };

    let mut layout = flatten::flatten_to_elems(topcell, &lib, needed.as_ref());
    pdk.compute_virtual_layers(&mut layout, dbu_to_um);

    // One tiled-merge cache shared by all geometric checks.  The halo must cover
    // the largest geometric rule distance in the deck so a single cached merge
    // serves every width/space/notch/etc. rule.
    const DIST_CHECKS: &[&str] =
        &["min_width", "max_width", "exact_width", "min_space", "min_notch", "min_enclosure", "max_enclosure"];
    let tile_dbu = (merge::TILE_UM / dbu_to_um).round() as i32;
    let halo_dbu = (merge::MIN_HALO_UM / dbu_to_um).ceil() as i32;

    // Halo is computed per layer: each layer only needs to see neighbour geometry
    // out to the largest distance rule that references *it*.  A deck-wide halo
    // would let one coarse rule (e.g. LBE `max_width` at 1500 µm) inflate the
    // merge of every fine layer (the metals) and exhaust memory.
    //
    // A spacing rule between two layers can only fire if *both* are present, so an
    // empty partner must not inflate the other's halo.  In a combined run (the full
    // suite) `min_space [LBE, Activ] = 30 µm` would otherwise give the dense Activ a
    // 30 µm halo on a chip that has no LBE at all — a ~100 GB tiled merge for a rule
    // that cannot produce a single violation.  Skip such rules here.  (Only base/global
    // layers are checked; a lazy virtual isn't materialised yet, so it is conservatively
    // treated as non-empty — the inflating rules in practice reference base layers.)
    let is_empty_base = |l: &pdk::Layer| {
        let key = (l.gds_layer as i16, l.gds_datatype as i16);
        !lazy_keys.contains(&key) && layout.get(key.0, key.1).is_empty()
    };
    let mut halo_by_layer: std::collections::HashMap<(i16, i16), i32> = std::collections::HashMap::new();
    for rule in rules.iter().filter(|r| DIST_CHECKS.contains(&r.check.as_str())) {
        if matches!(rule.check.as_str(), "min_space" | "min_notch")
            && rule.layers.iter().any(is_empty_base)
        {
            continue;
        }
        let h = (merge::MIN_HALO_UM.max(rule.value) / dbu_to_um).ceil() as i32;
        for l in &rule.layers {
            let key = (l.gds_layer as i16, l.gds_datatype as i16);
            let e = halo_by_layer.entry(key).or_insert(0);
            *e = (*e).max(h);
        }
    }


    // A lazy virtual layer is composed from its sources' tiles, so each source must
    // tile with a halo at least as large as the virtual layer's own (the result
    // keeps only what the sources covered).  A `close` additionally dilates-then-erodes
    // by its radius, so its source needs an extra 2·radius of halo to be exact in the core;
    // a `grow` only dilates (no erode-back), so 1·radius suffices.  The radius part is
    // seeded even when no distance rule references the virtual (e.g. a grow feeding a
    // `nonempty` chain, like Padc.d's pad-anchored 30 µm reach): a morphological op
    // intrinsically needs source geometry within its radius to be correct per tile.
    for (spec, op) in &tiled_virtuals {
        let extra = match op {
            merge::VirtualOp::Close(r) | merge::VirtualOp::Open(r) => 2 * r,
            merge::VirtualOp::Grow(r) => *r,
            // For `holes`/`with_holes`, radius declares the maximum expected ring
            // extent: a hole only materialises in a tile whose bucket assembles the
            // WHOLE ring, so the source needs the full ring within reach.
            merge::VirtualOp::Holes | merge::VirtualOp::WithHoles => {
                spec.radius.map(|r| (r / dbu_to_um).ceil() as i32).unwrap_or(0)
            }
            _ => 0,
        };
        let need = halo_by_layer.get(&spec.key).copied().unwrap_or(0) + extra;
        if need > 0 {
            for s in &spec.sources {
                let e = halo_by_layer.entry(*s).or_insert(0);
                *e = (*e).max(need);
            }
        }
    }

    if std::env::var("GDSCHECK_DUMP_HALO").is_ok() {
        let mut hv: Vec<_> = halo_by_layer.iter().collect();
        hv.sort_by_key(|(_, h)| std::cmp::Reverse(**h));
        eprintln!("--- per-layer halo (dbu), top 30 ---");
        for ((l, d), h) in hv.iter().take(30) {
            eprintln!("  halo {:>9} dbu ({:.1} um)  layer {}/{}", h, **h as f64 * dbu_to_um, l, d);
        }
    }

    let mut cache = cache::Cache::new();
    let mut merged = merge::MergedCache::new(tile_dbu, halo_dbu, halo_by_layer);
    for (spec, op) in tiled_virtuals {
        merged.register_virtual(spec.key, op, spec.sources, spec.text);
    }
    let mut violations = vec![];

    // A deck may touch dozens of layers; the merged geometry of all of them at
    // once does not fit in memory.  Record the last rule index that references
    // each layer, then free that layer's cached tiles/regions once the deck moves
    // past it.  Decks are grouped by layer, so only a few stay resident at a time.
    let mut last_use: std::collections::HashMap<(i16, i16), usize> = std::collections::HashMap::new();
    for (i, rule) in rules.iter().enumerate() {
        for l in &rule.layers {
            last_use.insert((l.gds_layer as i16, l.gds_datatype as i16), i);
        }
    }

    // Net extraction is lazy: build it once, only if the deck actually has a net-aware
    // check and connectivity is enabled.  A geometry-only deck never pays for it.
    let net = if connectivity
        && rules.iter().any(|r| NET_AWARE_CHECKS.contains(&r.check.as_str()))
        && !pdk.connectivity.is_empty()
    {
        use std::io::Write;
        print!("Connecting nets ... ");
        std::io::stdout().flush().ok();
        let t = std::time::Instant::now();
        let c = connectivity::Connectivity::build(&mut merged, &layout, &pdk.connectivity);
        println!("done ({:.1}s)", t.elapsed().as_secs_f64());
        Some(c)
    } else {
        None
    };

    for (i, rule) in rules.iter().enumerate() {
        if NET_AWARE_CHECKS.contains(&rule.check.as_str()) && net.is_none() {
            println!(
                "[{}] Skipping net-aware check '{}' (connectivity disabled)",
                rule.id, rule.check
            );
            continue;
        }
        violations.append(&mut checks::run_rule(
            rule,
            &layout,
            dbu_to_um,
            &mut cache,
            &mut merged,
            net.as_ref(),
        ));

        for l in &rule.layers {
            let key = (l.gds_layer as i16, l.gds_datatype as i16);
            if last_use.get(&key) == Some(&i) {
                merged.evict(key.0, key.1);
            }
        }
    }

    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A typo'd op or a missing radius must be a hard error, not a silently empty
    /// layer (which would turn every rule referencing it into a false-clean).
    #[test]
    fn parse_virtual_op_rejects_bad_config() {
        assert!(parse_virtual_op("interacting", None, 0.001).is_ok());
        assert!(parse_virtual_op("grow", Some(0.5), 0.001).is_ok());
        let e = parse_virtual_op("interactign", None, 0.001).unwrap_err();
        assert!(e.contains("unsupported op"), "{e}");
        let e = parse_virtual_op("close", None, 0.001).unwrap_err();
        assert!(e.contains("requires a radius"), "{e}");
    }
}
