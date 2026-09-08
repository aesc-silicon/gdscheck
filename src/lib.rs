// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod cache;
pub mod checks;
pub mod connectivity;
pub mod flatten;
pub mod geom;
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

/// Process CPU time so far, user plus system, in seconds; 0 where `/proc` is not there.
pub fn cpu_seconds() -> f64 {
    std::fs::read_to_string("/proc/self/stat")
        .ok()
        .and_then(|s| {
            // Fields 14 and 15 (1-based) after the parenthesised command name.
            let rest = s.rsplit(')').next()?;
            let f: Vec<&str> = rest.split_whitespace().collect();
            let ticks: f64 = f.get(11)?.parse::<f64>().ok()? + f.get(12)?.parse::<f64>().ok()?;
            Some(ticks / 100.0)
        })
        .unwrap_or(0.0)
}

/// `GDSCHECK_RULE_TRACE=1`: one line per stage of a run with its wall time and the CPU
/// time it used, so the stages that run on one core show themselves.
struct PhaseTrace {
    on: bool,
    wall: std::time::Instant,
    cpu: f64,
}

impl PhaseTrace {
    fn new() -> Self {
        Self {
            on: std::env::var("GDSCHECK_RULE_TRACE").is_ok(),
            wall: std::time::Instant::now(),
            cpu: cpu_seconds(),
        }
    }

    /// Close the stage `name` and open the next.
    fn end(&mut self, name: &str) {
        if !self.on {
            return;
        }
        let (w, c) = (self.wall.elapsed().as_secs_f64(), cpu_seconds() - self.cpu);
        eprintln!(
            "phase {name} wall={w:.1}s cpu={c:.1}s cores={:.1}",
            if w > 0.0 { c / w } else { 0.0 }
        );
        self.wall = std::time::Instant::now();
        self.cpu = cpu_seconds();
    }
}

/// Checks that need electrical connectivity (net extraction).  When connectivity is
/// disabled (`connectivity == false`) these are skipped rather than run on no nets.
/// Populated as net-aware checks land (e.g. the antenna ratio rules).
pub const NET_AWARE_CHECKS: &[&str] = &[
    "antenna_ratio",
    "gate_connected_min_area",
    "max_nets_under",
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
    slack: Option<f64>,
    dbu_to_um: f64,
) -> Result<merge::VirtualOp, String> {
    use merge::VirtualOp::*;
    let radius_dbu = || {
        radius
            .map(|r| (r / dbu_to_um).round() as i32)
            .ok_or_else(|| format!("op '{op}' requires a radius"))
    };
    let to_dbu = |v: Option<f64>| v.map(|x| (x / dbu_to_um).round() as i32);
    // Neighbour counts for the region selectors: whole numbers, and a minimum below one is
    // a selector that keeps everything, which is a deck bug rather than a rule.
    let counts = || -> Result<(Option<u32>, Option<u32>), String> {
        for (v, which) in [(min, "min"), (max, "max")] {
            if let Some(v) = v
                && (v < 1.0 || v.fract() != 0.0)
            {
                return Err(format!(
                    "op '{op}' takes a whole `{which}` count of 1 or more"
                ));
            }
        }
        Ok((min.map(|v| v as u32), max.map(|v| v as u32)))
    };
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
        //
        // The selectors take `min`/`max` as KLayout's inclusive neighbour *counts*, not as
        // a measurement: `interacting` with `min: 2, max: 2` is `interacting(other, 2, 2)`.
        // Absent bounds are the uncounted form, "at least one".
        "overlapping" | "not_outside" => {
            let (a, b) = counts()?;
            Overlapping(a, b)
        }
        "not_overlapping" | "outside" => {
            let (a, b) = counts()?;
            NotOverlapping(a, b)
        }
        // KLayout `interacting` — shared area *or* zero-area contact.
        "interacting" => {
            let (a, b) = counts()?;
            Interacting(a, b)
        }
        "not_interacting" => {
            let (a, b) = counts()?;
            NotInteracting(a, b)
        }
        "inside" => Inside,
        "not_inside" => NotInside,
        "covering" => {
            let (a, b) = counts()?;
            Covering(a, b)
        }
        "not_covering" => {
            let (a, b) = counts()?;
            NotCovering(a, b)
        }
        "not_circle_or_octagon" => NotCircleOrOctagon,
        "not_circle" => NotCircle,
        "holes" => Holes,
        "with_holes" => WithHoles,
        "with_text" => WithText,
        "extents" => Extents,
        "with_area" => {
            if min.is_none() && max.is_none() {
                return Err(format!("op '{op}' requires a `min` and/or `max` bound"));
            }
            // Bounds are um^2 in the deck and DBU^2 here, so the conversion squares.
            let to_dbu2 = |v: Option<f64>| v.map(|x| (x / (dbu_to_um * dbu_to_um)).round() as i64);
            WithArea(to_dbu2(min), to_dbu2(max))
        }
        "with_bbox_min" => {
            let (lo, hi) = bounds()?;
            WithBBoxMin(lo, hi)
        }
        "with_bbox_max" => {
            let (lo, hi) = bounds()?;
            WithBBoxMax(lo, hi)
        }
        "enclosure_above" => {
            let v = min.ok_or_else(|| format!("virtual op '{op}' requires a `min`"))?;
            EnclosureAbove((v / dbu_to_um).round() as i32)
        }
        "enclosure_below" => {
            let v = max.ok_or_else(|| format!("virtual op '{op}' requires a `max`"))?;
            EnclosureBelow((v / dbu_to_um).round() as i32)
        }
        "separation_below" => {
            let v = max.ok_or_else(|| format!("virtual op '{op}' requires a `max`"))?;
            SeparationBelow((v / dbu_to_um).round() as i32)
        }
        "close" => Close(radius_dbu()?),
        "open" => Open(radius_dbu()?),
        // Slack is opt-in; see `VirtualLayerDef::slack` for the one case that wants it.
        "grow" => Grow(
            radius_dbu()?,
            slack.map_or(0, |m| (m / dbu_to_um).round() as i32),
        ),
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
    fraction: Option<f64>,
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
        "or" | "join" => Or,
        "inside_part" => InsidePart,
        "outside_part" => OutsidePart,
        // Whole-edge selection: keeps or drops each segment entire, where the `_part`
        // ops cut it at the boundary.
        "interacting" => Interacting,
        "not_interacting" => NotInteracting,
        "interacting_edges" => InteractingEdges,
        "not_interacting_edges" => NotInteractingEdges,
        // `centers(length, fraction)`: `min` is the absolute length in µm, `fraction` the
        // relative one, and KLayout keeps whichever is longer.  Neither given would keep
        // the edge entire, which is not what any deck means by asking for its centre.
        "width_below" => {
            let v = max.ok_or_else(|| format!("edge op '{op}' requires a `max`"))?;
            merge::EdgeOp::WidthBelow((v / dbu_to_um).round() as i32)
        }
        "centers" => {
            if min.is_none() && fraction.is_none() {
                return Err(format!("edge op '{op}' requires a `min` and/or `fraction`"));
            }
            if let Some(f) = fraction
                && !(0.0..=1.0).contains(&f)
            {
                return Err(format!(
                    "edge op '{op}' takes a `fraction` in 0..1, got {f}"
                ));
            }
            Centers(
                to_dbu(min).unwrap_or(0),
                fraction.map_or(0, |f| (f * 1000.0).round() as i32),
            )
        }
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

/// A reach along each axis, in DBU.  A halo is one number - the tiles are square - but
/// a derivation's reach is not: `shrink_y 15` reads fifteen micrometres up and down and
/// nothing sideways.  Summing the four directional sizes of GF180's slotting opening as
/// four isotropic reaches charged its drawn metal twice what the opening reads, and
/// every layer feeding it the same.  The axes are tracked apart and the halo is the
/// larger of the two at the end.
type Reach = (i32, i32);

/// One relaxation pass of the virtual-layer halo propagation: push each tiled virtual's
/// own halo (plus its op's intrinsic reach) onto its sources.  Called repeatedly to a
/// fixed point by [`halo_table`], because virtuals chain and a single pass moves a halo only
/// one link.  A virtual outside `in_scope` neither needs building nor propagates.
///
/// A lazy virtual is composed from its sources' tiles, so each source needs at least the
/// virtual's own halo (the result keeps only what the sources covered).  A `close`
/// dilates then erodes by its radius, so its source needs 2·r more to be exact in the
/// core; a `grow` only dilates, so 1·r.  The radius part is charged even when no distance
/// rule reads the virtual - a grow feeding a `nonempty` chain still needs its source
/// within reach to be right per tile.
fn propagate_virtual_halos(
    tiled_virtuals: &[(pdk::TiledVirtualSpec, merge::VirtualOp)],
    in_scope: Option<&std::collections::HashSet<(i16, i16)>>,
    clippable: &std::collections::HashSet<(i16, i16)>,
    halo_by_layer: &mut std::collections::HashMap<(i16, i16), Reach>,
    why: &mut std::collections::HashMap<(i16, i16), String>,
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
        let extra: Reach = match op {
            merge::VirtualOp::Close(r) | merge::VirtualOp::Open(r) => (2 * r, 2 * r),
            // Reads to the far side of the gap it measures.
            merge::VirtualOp::SeparationBelow(r)
            | merge::VirtualOp::EnclosureBelow(r)
            | merge::VirtualOp::EnclosureAbove(r) => (*r, *r),
            merge::VirtualOp::Grow(r, _) => (*r, *r),
            // A directional size reads geometry up to `r` away along its axis only -
            // an erode dilates the complement by that much, a dilate the shape.  An
            // isotropic erode reads the same distance in every direction.
            merge::VirtualOp::GrowX(r) | merge::VirtualOp::ShrinkX(r) => (*r, 0),
            merge::VirtualOp::GrowY(r) | merge::VirtualOp::ShrinkY(r) => (0, *r),
            merge::VirtualOp::Shrink(r) => (*r, *r),
            // `holes`/`with_holes` read the stitched region, so a ring of any size has
            // its hole without the source holding the whole ring in one tile.
            _ => (0, 0),
        };
        // A layer delivered as core-clipped pieces (see `clippable_layers`) only has to
        // be exact in its core, so its sources owe it its own reach and nothing of what
        // its consumers ask of it; that part is met by copying the pieces outward.
        let own = if clippable.contains(&spec.key) {
            (0, 0)
        } else {
            halo_by_layer.get(&spec.key).copied().unwrap_or((0, 0))
        };
        let need = (own.0 + extra.0, own.1 + extra.1);
        if need == (0, 0) {
            continue;
        }
        for src in &spec.sources {
            let e = halo_by_layer.entry(*src).or_insert((0, 0));
            if need.0 > e.0 || need.1 > e.1 {
                *e = (e.0.max(need.0), e.1.max(need.1));
                let via = why
                    .get(&spec.key)
                    .map(|w| format!(" <- {w}"))
                    .unwrap_or_default();
                why.insert(*src, format!("{}{via}", spec.name));
            }
        }
    }
}

/// The checks whose value is a distance the merge has to see across a tile edge.
///
/// Every check that measures *between* shapes belongs here, whatever else gates it: the
/// net-aware spacings, the parallel-run and bent variants, the array spacing, the
/// extension and overlap and endcap margins, the erosion `wide_uncovered` runs.  For a
/// long time only the plain eight were listed, and the rest worked because some other
/// rule on the same layers had raised the halo to cover them - LPW.2a's 1.4 µm
/// different-net spacing rode on LPW.3's 2.5 µm.  A halo built per rule has no such
/// luck to ride on, which is how the omission showed: three foundry violations at
/// 1.395 µm gone, under a 1 µm halo.
const DIST_CHECKS: &[&str] = &[
    "min_width",
    "max_width",
    "exact_width",
    "min_45_width",
    "max_distance",
    "max_space",
    "min_space",
    "min_space_different_net",
    "min_space_same_net",
    "min_space_prl",
    "min_space_bent",
    "min_array_space",
    "min_notch",
    "min_enclosure",
    "max_enclosure",
    "min_endcap_enclosure",
    "min_extension",
    "min_overlap",
    "min_via_array",
    "wide_uncovered",
    // A polygon's own extent: the copy in its tile has to be whole up to the value, or a
    // long shape cut at the tile line is read as several short ones.  MDN.13a's 50 µm
    // `max_length` never fired once its body was built from clipped pieces.
    "min_length",
    "max_length",
    "min_dim",
    "max_dim",
    "gate_length",
    "min_edge_length",
];

/// The checks that read a polygon's own shape - its holes, its corners, its area - and
/// so need the tile's copy whole a little past the core.
const SHAPE_CHECKS: &[&str] = &[
    "no_ring",
    "ring_covers_boundary",
    "min_enclosed_area",
    "no_corner",
    "min_area",
    "max_area",
];

/// A halo per layer (DBU), and for each the rule or derivation chain that set it.
type HaloTable = (
    std::collections::HashMap<(i16, i16), i32>,
    std::collections::HashMap<(i16, i16), String>,
);

/// The halo each layer needs so that `rules` are exact per tile, and why.
///
/// A rule seeds the layers it names with its own reach; a derivation charges its sources
/// what it reaches on top of what is asked of it, run to a fixed point along the chains;
/// an edge layer cut from a measurement reads as far as the measurement does.  Layers
/// nothing raises are absent, and a reader takes its own floor for them.
///
/// Called once over every rule for the configured maxima - what `GDSCHECK_DUMP_HALO`
/// prints, and what a layer a rule reaches only incidentally gets - and once per rule
/// over that rule alone, which is what its merges are built at.  The first is a maximum
/// over every consumer of a layer, and on a real design one long-reach chain on a dense
/// layer (a 200 um guard-ring `holes` on the actives, the slotting opening on the vias)
/// made every other rule on it pay for that reach: Contact at 60 um is 175 million
/// polygon copies from 3.6 million shapes, for a rule that measures 70 nm.
#[allow(clippy::too_many_arguments)]
fn halo_table(
    rules: &[&pdk::RuleDefinition],
    in_scope: Option<&std::collections::HashSet<(i16, i16)>>,
    clippable: &std::collections::HashSet<(i16, i16)>,
    tiled_virtuals: &[(pdk::TiledVirtualSpec, merge::VirtualOp)],
    edge_specs: &[pdk::TiledEdgeSpec],
    is_empty_base: &dyn Fn(&pdk::Layer) -> bool,
    halo_dbu: i32,
    dbu_to_um: f64,
) -> HaloTable {
    let mut halo: std::collections::HashMap<(i16, i16), Reach> = std::collections::HashMap::new();
    let mut why: std::collections::HashMap<(i16, i16), String> = std::collections::HashMap::new();
    // A check that reads a polygon's own shape - its holes, its corners, its area -
    // needs the tile's copy whole a little past the core, which a merge of whole copies
    // always gave and a layer built from clipped pieces gives only when something
    // asked; those seed the minimum.  A distance check seeds its value.  A check that
    // stitches regions or takes a boolean residual asks nothing of the tiles beyond the
    // core, and seeding it anyway made a layer copied out that never was, promoting a
    // neighbour's slack into a second Rsil.c marker.
    for rule in rules.iter() {
        let dist = DIST_CHECKS.contains(&rule.check.as_str());
        if !dist && !SHAPE_CHECKS.contains(&rule.check.as_str()) {
            continue;
        }
        // A spacing rule between two layers can only fire if *both* are present, so an
        // empty partner must not inflate the other's halo.  In a combined run (the full
        // suite) `min_space [LBE, Activ] = 30 µm` would otherwise give the dense Activ a
        // 30 µm halo on a chip that has no LBE at all - a ~100 GB tiled merge for a rule
        // that cannot produce a single violation.  (Only base/global layers are checked;
        // a lazy virtual isn't materialised yet, so it is conservatively treated as
        // non-empty - the inflating rules in practice reference base layers.)
        let inflating = matches!(rule.check.as_str(), "min_space" | "min_notch")
            && rule.layers.iter().any(is_empty_base);
        let h = if dist && !inflating {
            (merge::MIN_HALO_UM.max(rule.value) / dbu_to_um).ceil() as i32
        } else {
            halo_dbu
        };
        for l in &rule.layers {
            let key = (l.gds_layer as i16, l.gds_datatype as i16);
            let e = halo.entry(key).or_insert((0, 0));
            if h > e.0 || h > e.1 {
                *e = (e.0.max(h), e.1.max(h));
                why.insert(key, format!("rule {}", rule.id));
            }
        }
    }
    // Virtuals chain, and a chain's reach is the sum of its links.  A single pass in
    // declaration order only ever moves a halo one link, and only when the deck happens
    // to declare the consumer first, so this runs to a fixed point instead.  Halos only
    // ever grow, and each pass that changes nothing ends it; the virtual graph is
    // acyclic, so it terminates.
    //
    // Edge layers chain the same way and are walked in the same loop: an edge layer
    // owes its sources its own reach - a 50 µm `max_width` on gate-end edges checks
    // that the span lies inside the polygon they were cut from, fifty microns out -
    // plus what its op reads (a measurement, as far as it measures), and never less
    // than the minimum for a polygon source, whose boundary has to be the region's a
    // little past the core and not a tile's.
    loop {
        let before = halo.clone();
        propagate_virtual_halos(
            tiled_virtuals,
            in_scope,
            clippable,
            &mut halo,
            &mut why,
            dbu_to_um,
        );
        for spec in edge_specs {
            if in_scope.is_some_and(|n| !n.contains(&spec.key)) {
                continue;
            }
            let own = halo.get(&spec.key).copied().unwrap_or((0, 0));
            let extra = match (spec.op.as_str(), spec.max) {
                ("width_below", Some(v)) => (v / dbu_to_um).ceil() as i32,
                _ => 0,
            };
            let need = (own.0 + extra).max(halo_dbu);
            let need = (need, (own.1 + extra).max(halo_dbu));
            for src in &spec.sources {
                let e = halo.entry(*src).or_insert((0, 0));
                if need.0 > e.0 || need.1 > e.1 {
                    *e = (e.0.max(need.0), e.1.max(need.1));
                    let via = why
                        .get(&spec.key)
                        .map(|w| format!(" <- {w}"))
                        .unwrap_or_default();
                    why.insert(*src, format!("edge layer {}{via}", spec.name));
                }
            }
        }
        if halo == before {
            break;
        }
    }
    let halo = halo
        .into_iter()
        .map(|(k, (x, y))| (k, x.max(y).max(halo_dbu)))
        .collect();
    (halo, why)
}

type LayerSet = std::collections::HashSet<(i16, i16)>;

/// The derived layers that can be handed to their consumers as core-clipped pieces.
///
/// A tile's copy of a derived layer is exact in the core and trails off beyond it, so a
/// consumer that reads across a tile edge needs the copy to be exact that far, and the
/// layer's sources further still: a chain of eight 15 µm sizes charged its drawn metal
/// 60 µm, and the vias under it 30 µm, for a rule that reads nothing across a tile edge
/// at all.  Cutting each tile's result to its core and copying the pieces to the tiles
/// within the consumers' reach delivers exactly that reach from a layer that was only
/// ever exact in its core - so its sources owe it its own operator's reach and no more,
/// and the chain stops accumulating.
///
/// Pieces are a region only to a consumer that reads them as one.  A boolean unions
/// them; a size unions them first (see `shrink`); a selection stitches them along the
/// cuts, and a region filter sums their areas.  Anything that looks at a polygon on its
/// own - a rectangle or circle filter, a hole finder, an extents box, a `covering` whose
/// filter has to fit inside one candidate polygon, an edge cut, a rule measuring a wall -
/// would see the cuts, and a layer with such a consumer stays whole.  A selection or
/// region filter passes its candidate's pieces straight through to its own output, so it
/// qualifies only if it qualifies itself, which is why this runs to a fixed point.
fn clippable_layers(
    tiled_virtuals: &[(pdk::TiledVirtualSpec, merge::VirtualOp)],
    edge_specs: &[pdk::TiledEdgeSpec],
    rules: &[pdk::RuleDefinition],
) -> (LayerSet, LayerSet) {
    use merge::VirtualOp as O;
    let reads_as_region = |op: O| {
        matches!(
            op,
            O::Union
                | O::Intersection
                | O::Difference
                | O::Overlapping(_, _)
                | O::NotOverlapping(_, _)
                | O::Interacting(_, _)
                | O::NotInteracting(_, _)
                | O::Inside
                | O::NotInside
                | O::Grow(_, _)
                | O::GrowX(_)
                | O::GrowY(_)
                | O::Shrink(_)
                | O::ShrinkX(_)
                | O::ShrinkY(_)
                | O::Close(_)
                | O::Open(_)
                | O::WithArea(_, _)
                | O::WithBBoxMin(_, _)
                | O::WithBBoxMax(_, _)
                | O::Holes
                | O::WithHoles
                | O::Extents
        )
    };
    let passes_through = |op: O| {
        matches!(
            op,
            O::Overlapping(_, _)
                | O::NotOverlapping(_, _)
                | O::Interacting(_, _)
                | O::NotInteracting(_, _)
                | O::Inside
                | O::NotInside
                | O::WithArea(_, _)
                | O::WithBBoxMin(_, _)
                | O::WithBBoxMax(_, _)
        )
    };
    let mut read_whole: std::collections::HashSet<(i16, i16)> = rules
        .iter()
        .flat_map(|r| {
            r.layers
                .iter()
                .map(|l| (l.gds_layer as i16, l.gds_datatype as i16))
        })
        .collect();
    for spec in edge_specs {
        read_whole.extend(spec.sources.iter().copied());
    }
    // Pieces are for readers that work on regions.  Anything that reads a polygon copy
    // as a polygon is built the old way, whole copies down to the drawn layers - never
    // clipped, never cut at the zone when copied out, and a selection feeding another
    // layer left without copies - and so is everything upstream of it, since a layer
    // built from pieces is itself in pieces.  That is every check that measures or
    // reads a shape: a wall the tiles have cut into per-tile fragments is reported once
    // per fragment, where whole copies gave every tile the identical wall and the
    // report folded them to one (CUP.2 on a millimetre of 0.3 µm metal went from one
    // marker to fifty).  It is the enclosure engine, which tests every vertex of the
    // enclosed shape against the copy of the enclosing layer in the tile that owns the
    // shape, and `covering`, which asks whether its filter fits inside one candidate
    // copy - both right only when the copy happens to extend over the other shape,
    // which whole copies of drawn geometry do (MIMTM.3 encloses a sixty-micron fuse
    // window in a derived plate).  And it is an edge cut, whose segments feed the edge
    // checks.  What is left for pieces is a chain that ends in `nonempty`, a coverage
    // residual, an area or a density - and the slotting chains, the ones that cost
    // the most, are exactly that.
    let mut sources_of: std::collections::HashMap<(i16, i16), &[(i16, i16)]> =
        std::collections::HashMap::new();
    for (spec, _) in tiled_virtuals {
        sources_of.insert(spec.key, &spec.sources);
    }
    let reads_polygons = |check: &str| {
        DIST_CHECKS.contains(&check)
            || SHAPE_CHECKS.contains(&check)
            || matches!(check, "no_angle" | "offgrid")
    };
    let mut stack: Vec<(i16, i16)> = rules
        .iter()
        .filter(|r| reads_polygons(&r.check))
        .flat_map(|r| {
            r.layers
                .iter()
                .map(|l| (l.gds_layer as i16, l.gds_datatype as i16))
        })
        .collect();
    for spec in edge_specs {
        stack.extend(spec.sources.iter().copied());
    }
    for (spec, op) in tiled_virtuals {
        if matches!(op, O::Covering(_, _) | O::NotCovering(_, _)) {
            stack.extend(spec.sources.first().copied());
        }
    }
    let mut whole_chain: std::collections::HashSet<(i16, i16)> = std::collections::HashSet::new();
    while let Some(k) = stack.pop() {
        if !whole_chain.insert(k) {
            continue;
        }
        if let Some(srcs) = sources_of.get(&k) {
            stack.extend(srcs.iter().copied());
        }
    }
    read_whole.extend(whole_chain.iter().copied());
    // Every consumer of each layer: the consumer's op, and whether the layer is the
    // candidate it passes through.
    type Consumer = ((i16, i16), merge::VirtualOp, bool);
    let mut consumers: std::collections::HashMap<(i16, i16), Vec<Consumer>> =
        std::collections::HashMap::new();
    for (spec, op) in tiled_virtuals {
        for (i, src) in spec.sources.iter().enumerate() {
            consumers
                .entry(*src)
                .or_default()
                .push((spec.key, *op, i == 0 && passes_through(*op)));
        }
    }
    let mut ok: std::collections::HashSet<(i16, i16)> = tiled_virtuals
        .iter()
        .filter(|(spec, _)| !read_whole.contains(&spec.key))
        .filter(|(spec, _)| {
            consumers.get(&spec.key).is_some_and(|cs| {
                !cs.is_empty() && cs.iter().all(|(_, op, _)| reads_as_region(*op))
            })
        })
        .map(|(spec, _)| spec.key)
        .collect();
    loop {
        let before = ok.len();
        let snapshot = ok.clone();
        ok.retain(|key| {
            consumers[key]
                .iter()
                .all(|(consumer, _, through)| !through || snapshot.contains(consumer))
        });
        if ok.len() == before {
            break;
        }
    }
    (ok, whole_chain)
}

/// Every layer `seeds` reach through the derivation graph: the seeds, their sources,
/// and so on down to the drawn layers.
fn layer_closure(
    seeds: impl IntoIterator<Item = (i16, i16)>,
    sources_of: &std::collections::HashMap<(i16, i16), Vec<(i16, i16)>>,
) -> std::collections::HashSet<(i16, i16)> {
    let mut out = std::collections::HashSet::new();
    let mut stack: Vec<(i16, i16)> = seeds.into_iter().collect();
    while let Some(k) = stack.pop() {
        if !out.insert(k) {
            continue;
        }
        if let Some(srcs) = sources_of.get(&k) {
            stack.extend(srcs.iter().copied());
        }
    }
    out
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
    run_drc_impl(
        LibSource::Path(gds_path),
        process,
        decks,
        suite,
        topcell,
        connectivity,
    )
}

/// Where a run's library comes from: read here, or handed over already read.
enum LibSource<'a> {
    Path(&'a str),
    Loaded(&'a GdsLibrary),
}

/// [`run_drc`] over a library already in memory.  The command line reads the file once
/// to print its name and validate the top cell, and read it a second time in here: two
/// gunzips and two parses of a 14 MB file, nine seconds of a run's start on one core.
pub fn run_drc_with(
    lib: &GdsLibrary,
    process: &str,
    decks: &[&str],
    suite: Option<&str>,
    topcell: &str,
    connectivity: bool,
) -> Result<Vec<Violation>, String> {
    run_drc_impl(
        LibSource::Loaded(lib),
        process,
        decks,
        suite,
        topcell,
        connectivity,
    )
}

fn run_drc_impl(
    source: LibSource,
    process: &str,
    decks: &[&str],
    suite: Option<&str>,
    topcell: &str,
    connectivity: bool,
) -> Result<Vec<Violation>, String> {
    let mut phase = PhaseTrace::new();
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

    // The rules are validated above before the file is touched, so a bad deck is
    // reported without a layout - and a missing layout is reported after a good deck.
    let owned: GdsLibrary;
    let lib: &GdsLibrary = match source {
        LibSource::Loaded(l) => l,
        LibSource::Path(p) => {
            owned = load_gds(p).map_err(|e| e.to_string())?;
            &owned
        }
    };
    if !lib.structs.iter().any(|s| s.name == topcell) {
        return Err(format!("Topcell '{topcell}' not found in library"));
    }
    phase.end("pdk+rules");

    let dbu_to_um = lib.units.1 * 1e6;

    // Resolve each lazy virtual's op string once (radii converted to DBU); a typo'd
    // op or a missing radius is a config error, not a silently empty layer.
    let tiled_virtuals: Vec<(pdk::TiledVirtualSpec, merge::VirtualOp)> = tiled_virtuals
        .into_iter()
        .map(|spec| {
            let op = parse_virtual_op(
                &spec.op,
                spec.radius,
                spec.min,
                spec.max,
                spec.slack,
                dbu_to_um,
            )
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
            // A `layer_params` entry arrives as `<key>` and `<key>_dt`.
            for (k, &l) in &rule.params {
                if let Some(&dt) = rule.params.get(&format!("{k}_dt")) {
                    n.insert((l as i16, dt as i16));
                }
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

    let mut layout = flatten::flatten_to_elems(topcell, lib, needed.as_ref());
    phase.end("flatten");
    pdk.compute_virtual_layers(&mut layout, dbu_to_um);
    phase.end("global virtuals");

    // One tiled-merge cache shared by all geometric checks.
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
    let edge_specs = pdk.tiled_edge_layers();
    // What each derived layer is built from, for walking a rule's closure.
    let mut sources_of: std::collections::HashMap<(i16, i16), Vec<(i16, i16)>> =
        std::collections::HashMap::new();
    for (spec, _) in &tiled_virtuals {
        sources_of.insert(spec.key, spec.sources.clone());
    }
    for spec in &edge_specs {
        sources_of.insert(spec.key, spec.sources.clone());
    }
    let rule_keys = |rule: &pdk::RuleDefinition| -> Vec<(i16, i16)> {
        rule.layers
            .iter()
            .map(|l| (l.gds_layer as i16, l.gds_datatype as i16))
            .collect()
    };
    // The configured maximum per layer: over every rule, for the layers a rule reaches
    // without naming them and for the diagnostics.
    let (clippable, whole_chain) = clippable_layers(&tiled_virtuals, &edge_specs, &rules);
    let all_rules: Vec<&pdk::RuleDefinition> = rules.iter().collect();
    let (halo_by_layer, halo_why) = halo_table(
        &all_rules,
        needed.as_ref(),
        &clippable,
        &tiled_virtuals,
        &edge_specs,
        &is_empty_base,
        halo_dbu,
        dbu_to_um,
    );
    // And per rule, over its own closure, which is what its merges are built at.
    let rule_halos: Vec<merge::RuleHalos> = rules
        .iter()
        .map(|rule| {
            let closure = layer_closure(rule_keys(rule), &sources_of);
            let table = halo_table(
                &[rule],
                Some(&closure),
                &clippable,
                &tiled_virtuals,
                &edge_specs,
                &is_empty_base,
                halo_dbu,
                dbu_to_um,
            )
            .0;
            (table, closure)
        })
        .collect();

    phase.end("halo tables");
    if std::env::var("GDSCHECK_DUMP_HALO").is_ok() {
        let mut hv: Vec<_> = halo_by_layer.iter().collect();
        hv.sort_by_key(|(_, h)| std::cmp::Reverse(**h));
        eprintln!("--- per-layer halo (dbu), above the {halo_dbu} dbu minimum ---");
        for ((l, d), h) in hv.iter().filter(|(_, h)| **h > halo_dbu) {
            eprintln!(
                "  halo {:>9} dbu ({:>7.1} um)  layer {:>7}/{:<3}  {}",
                h,
                **h as f64 * dbu_to_um,
                l,
                d,
                halo_why
                    .get(&(*l, *d))
                    .map(String::as_str)
                    .unwrap_or("(unattributed)")
            );
        }
    }

    let mut cache = cache::Cache::new();
    let virtual_defs = tiled_virtuals.clone();
    let mut merged = merge::MergedCache::new(tile_dbu, halo_dbu, halo_by_layer);
    merged.set_clippable(clippable.clone());
    merged.set_whole_chain(whole_chain);
    merged.set_names(
        tiled_virtuals
            .iter()
            .map(|(spec, _)| (spec.key, spec.name.clone()))
            .chain(edge_specs.iter().map(|spec| (spec.key, spec.name.clone())))
            .collect(),
    );
    for (spec, op) in tiled_virtuals {
        merged.register_virtual(spec.key, op, spec.sources, spec.text);
    }
    for spec in pdk.tiled_edge_layers() {
        let op = parse_edge_op(&spec.op, spec.min, spec.max, spec.fraction, dbu_to_um)
            .map_err(|e| format!("Edge layer '{}': {e}", spec.name))?;
        merged.register_edge(spec.key, op, spec.sources);
    }
    let mut violations = vec![];

    // A deck may touch dozens of layers; the merged geometry of all of them at
    // once does not fit in memory.  Record the last rule whose closure reaches each
    // layer - named or built from, down to the drawn layers - then free that layer's
    // cache once the deck moves past it.  Decks are grouped by layer, so only a few
    // stay resident at a time.  Going by the layers a rule *names* left every
    // intermediate of a derivation chain resident for the run: 422 of the 827 virtual
    // layers GF180's main suite builds are named by no rule, and at 150 bytes a
    // polygon copy they were half of a peak that reached 100 GB.
    let mut last_use: std::collections::HashMap<(i16, i16), usize> =
        std::collections::HashMap::new();
    for (i, rule) in rules.iter().enumerate() {
        for key in layer_closure(rule_keys(rule), &sources_of) {
            last_use.insert(key, i);
        }
    }
    // And what every later rule will need of each layer, so a layer cached fatter than
    // that is dropped as well: rebuilt at the thinner halo on its next use, for the cost
    // of one merge.  Without this a halo only ever ratchets up - via3 merged at 31 µm
    // for the slotting opening, 109 million copies, stayed resident through the guard
    // ring deck that needed it at one, and the run died there.
    let n_rules = rules.len();
    let mut future_need: std::collections::HashMap<(i16, i16), Vec<i32>> =
        last_use.keys().map(|k| (*k, vec![-1; n_rules])).collect();
    let mut running: std::collections::HashMap<(i16, i16), i32> = std::collections::HashMap::new();
    for i in (0..n_rules).rev() {
        for (key, need) in &running {
            future_need.get_mut(key).expect("key from a closure")[i] = *need;
        }
        let (table, closure) = &rule_halos[i];
        for key in closure {
            let need = table.get(key).copied().unwrap_or(halo_dbu);
            let e = running.entry(*key).or_insert(need);
            *e = (*e).max(need);
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
        // Net extraction reads its layers through a cache of its own, tiled with **no
        // halo**.  A halo exists so a measurement can see across a tile edge; extraction
        // measures nothing.  It stitches regions, which is decided by core ownership and
        // by `link_adjacent_pieces` joining the pieces either side of a tile line, and it
        // resolves points, which are looked up in the tile that contains them - neither
        // reads a halo copy.  Every layer in the connect graph is a drawn layer or a
        // boolean of drawn layers, so no morphological op needs one either.
        //
        // Sharing the checks' cache made extraction pay the checks' halos, which are set
        // by the longest rule that touches a layer and by the slotting chain's eight
        // stacked sizes: 60 µm on Contact and 120 µm on the drawn metals.  Against a
        // 20 µm tile those multiply a layer's geometry by the square of the ratio -
        // Contact came to 175 million polygon copies from 3.6 million drawn shapes, and
        // Metal1 at 120 µm never finished.
        let mut conn_merged =
            merge::MergedCache::new(tile_dbu, 0, std::collections::HashMap::new());
        for (spec, op) in virtual_defs {
            conn_merged.register_virtual(spec.key, op, spec.sources, spec.text);
        }
        let c = connectivity::Connectivity::build(&mut conn_merged, &layout, &pdk.connectivity);
        drop(conn_merged);
        println!("done ({:.1}s)", t.elapsed().as_secs_f64());
        Some(c)
    } else {
        None
    };
    phase.end("net extraction");

    for (i, rule) in rules.iter().enumerate() {
        // What this rule needs of every layer in its closure, so a layer is merged at
        // that rather than at the maximum some other rule on it set.
        // Build a little ahead: a layer this rule wants at h that a later rule wants
        // at up to 1.5h is built for the later rule now, since the smaller copy could
        // not serve it and would be merged again.  comp at 200 um followed by a rule at
        // 215 um was two 1.6 s merges of 9 million copies for a 14% difference.
        let (mut table, closure) = rule_halos[i].clone();
        for (key, want) in table.iter_mut() {
            let fut = future_need[key][i];
            if fut > *want && fut <= *want + *want / 2 {
                *want = fut;
            }
        }
        merged.set_rule_halos(Some((table, closure)));
        if NET_AWARE_CHECKS.contains(&rule.check.as_str()) && net.is_none() {
            println!(
                "[{}] Skipping net-aware check '{}' (connectivity disabled)",
                rule.id, rule.check
            );
            continue;
        }
        let t_rule = std::time::Instant::now();
        let c_rule = cpu_seconds();
        violations.append(&mut checks::run_rule(
            rule,
            &layout,
            dbu_to_um,
            &mut cache,
            &mut merged,
            net.as_ref(),
        ));
        if std::env::var("GDSCHECK_RULE_TRACE").is_ok() {
            let rss = std::fs::read_to_string("/proc/self/statm")
                .ok()
                .and_then(|s| s.split_whitespace().nth(1).map(|v| v.to_string()))
                .unwrap_or_default();
            eprintln!(
                "rule {} {} {:.1}s cpu={:.1}s rss={:.1}GB cache={}",
                rule.id,
                rule.check,
                t_rule.elapsed().as_secs_f64(),
                cpu_seconds() - c_rule,
                rss.parse::<f64>().unwrap_or(0.0) * 4096.0 / 1e9,
                merged.resident_summary()
            );
        }

        for key in &rule_halos[i].1 {
            let future = future_need[key][i];
            // Dropped when nothing later needs it, or when it is far fatter than
            // anything later needs - the same ratio the rebuild uses, so a copy a
            // little fatter than the next rule's reach serves it rather than being
            // merged again at 14% fewer copies.
            let cached = merged.cached_halo(*key);
            if future < 0 || cached.is_some_and(|h| h > future * 4) {
                if cached.is_some() && std::env::var("GDSCHECK_RULE_TRACE").is_ok() {
                    eprintln!(
                        "evict {}/{} cached={:?} future={future}",
                        key.0, key.1, cached
                    );
                }
                merged.evict(key.0, key.1);
            }
        }
    }
    merged.set_rule_halos(None);

    phase.end("rules");
    // A run is over the same layout twice, so its report should be the same file twice.
    // The checks emit while walking tile maps, whose iteration order is not stable, so
    // the violations arrive shuffled; the *set* is deterministic and the order is not.
    // Sorting here makes two reports of one layout diffable, which is what anyone
    // comparing a fix against a baseline needs.
    violations.sort_by(|a, b| {
        let key = |v: &violation::Violation| {
            let (x, y, x2, y2) = match v.geometry {
                violation::ViolationGeometry::Point { x, y } => (x, y, x, y),
                violation::ViolationGeometry::Edge { x1, y1, x2, y2 } => (x1, y1, x2, y2),
                violation::ViolationGeometry::None => (0.0, 0.0, 0.0, 0.0),
            };
            (v.rule_id.clone(), x, y, x2, y2, v.message.clone())
        };
        key(a)
            .partial_cmp(&key(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // A violation sitting exactly on a tile line is claimed by both tiles that share it -
    // see `Core::owns`, which would rather report one twice than let the tile that cannot
    // see it silence the tile that can.  Sorted, those two are adjacent and identical.
    violations.dedup_by(|a, b| {
        a.rule_id == b.rule_id && a.message == b.message && a.geometry == b.geometry
    });
    phase.end("sort+dedup");
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
        assert!(parse_virtual_op("interacting", None, None, None, None, 0.001).is_ok());
        assert!(parse_virtual_op("grow", Some(0.5), None, None, None, 0.001).is_ok());
        let e = parse_virtual_op("interactign", None, None, None, None, 0.001).unwrap_err();
        assert!(e.contains("unsupported op"), "{e}");
        let e = parse_virtual_op("close", None, None, None, None, 0.001).unwrap_err();
        assert!(e.contains("requires a radius"), "{e}");
        let e = parse_virtual_op("shrink_x", None, None, None, None, 0.001).unwrap_err();
        assert!(e.contains("requires a radius"), "{e}");
        // A bbox filter with neither bound would keep everything — almost certainly a
        // mistyped key rather than an intentional no-op filter.
        let e = parse_virtual_op("with_bbox_min", None, None, None, None, 0.001).unwrap_err();
        assert!(e.contains("`min` and/or `max`"), "{e}");
        // A selector's bounds are neighbour counts, so a fraction is a deck that meant a
        // measurement, and a zero minimum is a selector that keeps everything.
        let e = parse_virtual_op("interacting", None, Some(1.5), None, None, 0.001).unwrap_err();
        assert!(e.contains("whole `min` count"), "{e}");
        let e = parse_virtual_op("covering", None, Some(0.0), None, None, 0.001).unwrap_err();
        assert!(e.contains("whole `min` count"), "{e}");
        assert!(parse_virtual_op("interacting", None, Some(2.0), Some(2.0), None, 0.001).is_ok());
    }

    /// `overlapping` and `interacting` must resolve to *different* ops: they differ only
    /// on zero-area contact, and silently aliasing them would be a correctness bug.
    #[test]
    fn parse_virtual_op_separates_overlapping_from_interacting() {
        let over = parse_virtual_op("overlapping", None, None, None, None, 0.001).unwrap();
        let inter = parse_virtual_op("interacting", None, None, None, None, 0.001).unwrap();
        assert_ne!(over, inter);
        // KLayout's `not_outside` / `outside` are the same relation under other names.
        assert_eq!(
            over,
            parse_virtual_op("not_outside", None, None, None, None, 0.001).unwrap()
        );
        assert_eq!(
            parse_virtual_op("not_overlapping", None, None, None, None, 0.001).unwrap(),
            parse_virtual_op("outside", None, None, None, None, 0.001).unwrap()
        );
    }

    /// Bounds are converted from µm to DBU with the library's own scale.
    #[test]
    fn parse_virtual_op_converts_bbox_bounds_to_dbu() {
        let op =
            parse_virtual_op("with_bbox_min", None, Some(2.0), Some(10.0), None, 0.001).unwrap();
        assert_eq!(op, merge::VirtualOp::WithBBoxMin(Some(2000), Some(10_000)));
        let op = parse_virtual_op("with_bbox_max", None, None, Some(0.5), None, 0.001).unwrap();
        assert_eq!(op, merge::VirtualOp::WithBBoxMax(None, Some(500)));
    }
}
