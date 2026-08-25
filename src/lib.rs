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

/// GDSII record types whose payload gds21 decodes as a string.  Each is the high byte of
/// the record header; the low byte is the data type, `0x06` for a string.
const STRING_RECORD_TYPES: [u8; 10] = [
    0x02, // LIBNAME
    0x06, // STRNAME
    0x12, // SNAME
    0x19, // STRING
    0x1f, // REFLIBS
    0x20, // FONTS
    0x23, // ATTRTABLE
    0x2c, // PROPVALUE
    0x37, // MASK
    0x3a, // SRFNAME
];

/// Give every zero-length string record a one-character payload, and report how many.
///
/// gds21 3.0.0-pre.2 panics on an empty string: `read_str` strips an optional trailing
/// NUL with `data[data.len() - 1]`, which underflows to `usize::MAX` when the payload is
/// empty. An empty label is unusual but perfectly legal, and real designs contain them —
/// so a whole DRC run dies on a text element that carries no DRC meaning at all.
///
/// The repair is the smallest one that survives that code path: a two-byte NUL payload,
/// which gds21 reads back as a single NUL character. It cannot be read back as the empty
/// string, because the only payload length that would produce one is the length that
/// panics. Labels are matched against patterns, and no pattern matches a NUL, so this
/// cannot turn a rule on or off — but it is a change to the input, so it is announced
/// rather than done quietly.
///
/// Anything that does not parse as a clean record stream is passed through untouched, so
/// a malformed file still gets gds21's own error rather than one from here.
fn repair_empty_strings(bytes: Vec<u8>) -> (Vec<u8>, usize) {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    let mut repaired = 0usize;
    while i + 4 <= bytes.len() {
        let len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        let (rtype, dtype) = (bytes[i + 2], bytes[i + 3]);
        if len < 4 || i + len > bytes.len() {
            // Not a record stream we understand: hand the original to gds21 unchanged.
            return (bytes, 0);
        }
        if len == 4 && dtype == 0x06 && STRING_RECORD_TYPES.contains(&rtype) {
            out.extend_from_slice(&6u16.to_be_bytes());
            out.extend_from_slice(&[rtype, dtype, 0x00, 0x00]);
            repaired += 1;
        } else {
            out.extend_from_slice(&bytes[i..i + len]);
        }
        i += len;
    }
    if i != bytes.len() {
        return (bytes, 0); // trailing bytes that are not a record: leave it alone
    }
    (out, repaired)
}

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

    let (bytes, repaired) = repair_empty_strings(bytes);
    if repaired > 0 {
        eprintln!(
            "warning: {path}: {repaired} empty text label(s) rewritten to a single NUL \
             character - gds21 cannot read a zero-length string and would panic. No \
             geometry is affected."
        );
    }

    Ok(GdsLibrary::from_bytes(&bytes)?)
}

/// Checks that need electrical connectivity (net extraction).  When connectivity is
/// disabled (`connectivity == false`) these are skipped rather than run on no nets.
/// Populated as net-aware checks land (e.g. the antenna ratio rules).
pub const NET_AWARE_CHECKS: &[&str] = &[
    "antenna_ratio",
    "gate_connected_min_area",
    "min_space_different_net",
    "min_space_same_net",
];

/// Parse a lazy virtual layer's `op` string to a [`merge::VirtualOp`], converting its
/// distances (µm) to DBU where the op takes them.  An unsupported op or a missing radius
/// is an error: the layer would otherwise silently register as empty and every rule
/// referencing it would become a no-op false-clean.
///
/// Op names follow KLayout's, including its distinction between `overlapping` (shares
/// positive area) and `interacting` (shares area *or* merely touches) — they differ only
/// on zero-area contact, and picking the wrong one is a silent correctness bug, so both
/// exist under the names a rule author reading the foundry deck will expect.
pub fn parse_virtual_op(
    op: &str,
    radius: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    dbu_to_um: f64,
) -> Result<merge::VirtualOp, String> {
    use merge::VirtualOp::*;
    let radius_dbu = || {
        radius
            .map(|r| (r / dbu_to_um).round() as i32)
            .ok_or_else(|| format!("op '{op}' requires a radius"))
    };
    let to_dbu = |v: Option<f64>| v.map(|x| (x / dbu_to_um).round() as i32);
    let bounds = || -> Result<(Option<i32>, Option<i32>), String> {
        if min.is_none() && max.is_none() {
            return Err(format!("op '{op}' requires a `min` and/or `max` bound"));
        }
        Ok((to_dbu(min), to_dbu(max)))
    };
    Ok(match op {
        "union" => Union,
        "intersection" | "and" => Intersection,
        "difference" | "not" => Difference,
        "square" => Square,
        "not_square" => NotSquare,
        "rectangle" => Rectangle,
        "not_rectangle" => NotRectangle,
        // KLayout `overlapping` / `not_outside` — positive shared area only.
        "overlapping" | "not_outside" => Overlapping,
        "not_overlapping" | "outside" => NotOverlapping,
        // KLayout `interacting` — shared area *or* zero-area contact.
        "interacting" => Interacting,
        "not_interacting" => NotInteracting,
        "inside" => Inside,
        "not_inside" => NotInside,
        "covering" => Covering,
        "not_covering" => NotCovering,
        "not_circle_or_octagon" => NotCircleOrOctagon,
        "not_circle" => NotCircle,
        "holes" => Holes,
        "with_holes" => WithHoles,
        "with_text" => WithText,
        "with_bbox_min" => {
            let (lo, hi) = bounds()?;
            WithBBoxMin(lo, hi)
        }
        "with_bbox_max" => {
            let (lo, hi) = bounds()?;
            WithBBoxMax(lo, hi)
        }
        "close" => Close(radius_dbu()?),
        "open" => Open(radius_dbu()?),
        "grow" => Grow(radius_dbu()?),
        "shrink" => Shrink(radius_dbu()?),
        "grow_x" => GrowX(radius_dbu()?),
        "grow_y" => GrowY(radius_dbu()?),
        "shrink_x" => ShrinkX(radius_dbu()?),
        "shrink_y" => ShrinkY(radius_dbu()?),
        other => return Err(format!("unsupported op '{other}'")),
    })
}

/// Parse an edge layer's `op` string to a [`merge::EdgeOp`].  `min`/`max` carry the
/// op's bounds — µm for the length filters, degrees for the angle ones.
pub fn parse_edge_op(
    op: &str,
    min: Option<f64>,
    max: Option<f64>,
    dbu_to_um: f64,
) -> Result<merge::EdgeOp, String> {
    use merge::EdgeOp::*;
    let to_dbu = |v: Option<f64>| v.map(|x| (x / dbu_to_um).round() as i32);
    let bounds = || -> Result<(Option<i32>, Option<i32>), String> {
        if min.is_none() && max.is_none() {
            return Err(format!(
                "edge op '{op}' requires a `min` and/or `max` bound"
            ));
        }
        Ok((to_dbu(min), to_dbu(max)))
    };
    let angles = || -> Result<(i32, i32), String> {
        match (min, max) {
            (Some(a), Some(b)) => Ok((a.round() as i32, b.round() as i32)),
            _ => Err(format!(
                "edge op '{op}' requires both `min` and `max` in degrees"
            )),
        }
    };
    Ok(match op {
        "edges" => Edges,
        "and" => And,
        "not" => Not,
        "inside_part" => InsidePart,
        "outside_part" => OutsidePart,
        "with_length" => {
            let (lo, hi) = bounds()?;
            WithLength(lo, hi)
        }
        "without_length" => {
            let (lo, hi) = bounds()?;
            WithoutLength(lo, hi)
        }
        "with_angle" => {
            let (lo, hi) = angles()?;
            WithAngle(lo, hi)
        }
        "without_angle" => {
            let (lo, hi) = angles()?;
            WithoutAngle(lo, hi)
        }
        other => return Err(format!("unsupported edge op '{other}'")),
    })
}

/// One relaxation pass of the virtual-layer halo propagation: push each tiled virtual's
/// halo (plus whatever reach its own operator needs) down onto its sources.  Called to a
/// fixed point by `run_drc`, because virtuals chain and a single pass moves a halo only
/// one link along the chain.
fn propagate_virtual_halos(
    tiled_virtuals: &[(pdk::TiledVirtualSpec, merge::VirtualOp)],
    in_scope: Option<&std::collections::HashSet<(i16, i16)>>,
    halo_by_layer: &mut std::collections::HashMap<(i16, i16), i32>,
    dbu_to_um: f64,
) {
    for (spec, op) in tiled_virtuals {
        // A virtual the deck never reads costs nothing to build, but its halo would
        // still be charged to its sources - and those sources may well be layers this
        // deck does read.  GF180's slotting chain is eight sizing operators deep, so
        // leaving it in scope would put a 120 um halo on the drawn metal of every deck
        // in the PDK.  Only virtuals in the run's layer closure propagate.
        if in_scope.is_some_and(|n| !n.contains(&spec.key)) {
            continue;
        }
        let extra = match op {
            merge::VirtualOp::Close(r) | merge::VirtualOp::Open(r) => 2 * r,
            merge::VirtualOp::Grow(r) | merge::VirtualOp::GrowX(r) | merge::VirtualOp::GrowY(r) => {
                *r
            }
            // A directional erode reads geometry up to `r` away along its axis (the
            // complement is dilated by that much), so the source needs the same reach.
            // An isotropic erode reads the same distance in every direction.
            merge::VirtualOp::Shrink(r)
            | merge::VirtualOp::ShrinkX(r)
            | merge::VirtualOp::ShrinkY(r) => *r,
            // For `holes`/`with_holes`, radius declares the maximum expected ring
            // extent: a hole only materialises in a tile whose bucket assembles the
            // WHOLE ring, so the source needs the full ring within reach.
            merge::VirtualOp::Holes | merge::VirtualOp::WithHoles => spec
                .radius
                .map(|r| (r / dbu_to_um).ceil() as i32)
                .unwrap_or(0),
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
            let op = parse_virtual_op(&spec.op, spec.radius, spec.min, spec.max, dbu_to_um)
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
    let needed: Option<std::collections::HashSet<(i16, i16)>> = if rules
        .iter()
        .any(|r| ALL_LAYER_CHECKS.contains(&r.check.as_str()))
    {
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
        if connectivity
            && rules
                .iter()
                .any(|r| NET_AWARE_CHECKS.contains(&r.check.as_str()))
        {
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
            for el in &pdk.edge_layers {
                let Some(elayer) = pdk.layer(&el.name) else {
                    continue;
                };
                if !n.contains(&(elayer.gds_layer as i16, elayer.gds_datatype as i16)) {
                    continue;
                }
                for src in &el.layers {
                    if let Some(s) = pdk.layer(src) {
                        added |= n.insert((s.gds_layer as i16, s.gds_datatype as i16));
                    }
                }
            }
            for vl in &pdk.virtual_layers {
                let Some(vlayer) = pdk.layer(&vl.name) else {
                    continue;
                };
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
    const DIST_CHECKS: &[&str] = &[
        "min_width",
        "max_width",
        "exact_width",
        "min_space",
        "min_notch",
        "min_enclosure",
        "max_enclosure",
    ];
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
    let mut halo_by_layer: std::collections::HashMap<(i16, i16), i32> =
        std::collections::HashMap::new();
    for rule in rules
        .iter()
        .filter(|r| DIST_CHECKS.contains(&r.check.as_str()))
    {
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
    //
    // Virtuals chain, and a chain's reach is the sum of its links: GF180's slotting
    // derivation is four directional sizes in a row, so the drawn metal underneath needs
    // 4·15 µm of halo, not 15.  A single pass in declaration order only ever moves a halo
    // one link, and only when the deck happens to declare the consumer first, so this
    // runs to a fixed point instead.  Halos only ever grow, and each pass that changes
    // nothing ends it; the virtual graph is acyclic, so it terminates.
    loop {
        let before = halo_by_layer.clone();
        propagate_virtual_halos(
            &tiled_virtuals,
            needed.as_ref(),
            &mut halo_by_layer,
            dbu_to_um,
        );
        if halo_by_layer == before {
            break;
        }
    }

    if std::env::var("GDSCHECK_DUMP_HALO").is_ok() {
        let mut hv: Vec<_> = halo_by_layer.iter().collect();
        hv.sort_by_key(|(_, h)| std::cmp::Reverse(**h));
        eprintln!("--- per-layer halo (dbu), top 30 ---");
        for ((l, d), h) in hv.iter().take(30) {
            eprintln!(
                "  halo {:>9} dbu ({:.1} um)  layer {}/{}",
                h,
                **h as f64 * dbu_to_um,
                l,
                d
            );
        }
    }

    let mut cache = cache::Cache::new();
    let mut merged = merge::MergedCache::new(tile_dbu, halo_dbu, halo_by_layer);
    for (spec, op) in tiled_virtuals {
        merged.register_virtual(spec.key, op, spec.sources, spec.text);
    }
    for spec in pdk.tiled_edge_layers() {
        let op = parse_edge_op(&spec.op, spec.min, spec.max, dbu_to_um)
            .map_err(|e| format!("Edge layer '{}': {e}", spec.name))?;
        merged.register_edge(spec.key, op, spec.sources);
    }
    let mut violations = vec![];

    // A deck may touch dozens of layers; the merged geometry of all of them at
    // once does not fit in memory.  Record the last rule index that references
    // each layer, then free that layer's cached tiles/regions once the deck moves
    // past it.  Decks are grouped by layer, so only a few stay resident at a time.
    let mut last_use: std::collections::HashMap<(i16, i16), usize> =
        std::collections::HashMap::new();
    for (i, rule) in rules.iter().enumerate() {
        for l in &rule.layers {
            last_use.insert((l.gds_layer as i16, l.gds_datatype as i16), i);
        }
    }

    // Net extraction is lazy: build it once, only if the deck actually has a net-aware
    // check and connectivity is enabled.  A geometry-only deck never pays for it.
    let net = if connectivity
        && rules
            .iter()
            .any(|r| NET_AWARE_CHECKS.contains(&r.check.as_str()))
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

    /// A record whose length field is exactly 4 has an empty payload. gds21 panics on
    /// that for any string record, so it is rewritten to a two-byte payload — real
    /// designs do contain empty labels, and one of them should not take a DRC run down.
    #[test]
    fn empty_string_records_are_repaired() {
        // STRNAME "TOP\0" (len 8), an empty STRING (len 4), then ENDEL (len 4).
        let mut gds = vec![0x00, 0x08, 0x06, 0x06, b'T', b'O', b'P', 0x00];
        gds.extend_from_slice(&[0x00, 0x04, 0x19, 0x06]);
        gds.extend_from_slice(&[0x00, 0x04, 0x11, 0x00]);
        let (out, repaired) = repair_empty_strings(gds);
        assert_eq!(repaired, 1);
        assert_eq!(
            out,
            vec![
                0x00, 0x08, 0x06, 0x06, b'T', b'O', b'P', 0x00, // STRNAME, untouched
                0x00, 0x06, 0x19, 0x06, 0x00, 0x00, // STRING, now two bytes long
                0x00, 0x04, 0x11, 0x00, // ENDEL, untouched
            ]
        );
    }

    /// A non-empty string record, and a non-string record that happens to be 4 bytes
    /// long, must both come through untouched — ENDEL and friends are always len 4.
    #[test]
    fn repair_leaves_everything_else_alone() {
        let gds = vec![
            0x00, 0x08, 0x06, 0x06, b'T', b'O', b'P', 0x00, // STRNAME "TOP"
            0x00, 0x04, 0x11, 0x00, // ENDEL: len 4, but not a string record
            0x00, 0x04, 0x07, 0x00, // ENDSTR
        ];
        let (out, repaired) = repair_empty_strings(gds.clone());
        assert_eq!(repaired, 0);
        assert_eq!(out, gds);
    }

    /// Anything that is not a clean record stream is handed to gds21 untouched, so a
    /// malformed file still produces gds21's error rather than a mangled one from here.
    #[test]
    fn repair_passes_through_a_malformed_stream() {
        let gds = vec![0x00, 0x02, 0x06, 0x06, 0xff]; // length 2 is impossible
        let (out, repaired) = repair_empty_strings(gds.clone());
        assert_eq!(repaired, 0);
        assert_eq!(out, gds);
    }

    /// A typo'd op or a missing parameter must be a hard error, not a silently empty
    /// layer (which would turn every rule referencing it into a false-clean).
    #[test]
    fn parse_virtual_op_rejects_bad_config() {
        assert!(parse_virtual_op("interacting", None, None, None, 0.001).is_ok());
        assert!(parse_virtual_op("grow", Some(0.5), None, None, 0.001).is_ok());
        let e = parse_virtual_op("interactign", None, None, None, 0.001).unwrap_err();
        assert!(e.contains("unsupported op"), "{e}");
        let e = parse_virtual_op("close", None, None, None, 0.001).unwrap_err();
        assert!(e.contains("requires a radius"), "{e}");
        let e = parse_virtual_op("shrink_x", None, None, None, 0.001).unwrap_err();
        assert!(e.contains("requires a radius"), "{e}");
        // A bbox filter with neither bound would keep everything — almost certainly a
        // mistyped key rather than an intentional no-op filter.
        let e = parse_virtual_op("with_bbox_min", None, None, None, 0.001).unwrap_err();
        assert!(e.contains("`min` and/or `max`"), "{e}");
    }

    /// `overlapping` and `interacting` must resolve to *different* ops: they differ only
    /// on zero-area contact, and silently aliasing them would be a correctness bug.
    #[test]
    fn parse_virtual_op_separates_overlapping_from_interacting() {
        let over = parse_virtual_op("overlapping", None, None, None, 0.001).unwrap();
        let inter = parse_virtual_op("interacting", None, None, None, 0.001).unwrap();
        assert_ne!(over, inter);
        // KLayout's `not_outside` / `outside` are the same relation under other names.
        assert_eq!(
            over,
            parse_virtual_op("not_outside", None, None, None, 0.001).unwrap()
        );
        assert_eq!(
            parse_virtual_op("not_overlapping", None, None, None, 0.001).unwrap(),
            parse_virtual_op("outside", None, None, None, 0.001).unwrap()
        );
    }

    /// Bounds are converted from µm to DBU with the library's own scale.
    #[test]
    fn parse_virtual_op_converts_bbox_bounds_to_dbu() {
        let op = parse_virtual_op("with_bbox_min", None, Some(2.0), Some(10.0), 0.001).unwrap();
        assert_eq!(op, merge::VirtualOp::WithBBoxMin(Some(2000), Some(10_000)));
        let op = parse_virtual_op("with_bbox_max", None, None, Some(0.5), 0.001).unwrap();
        assert_eq!(op, merge::VirtualOp::WithBBoxMax(None, Some(500)));
    }
}
