// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::layout::FlatLayout;
use gds21::GdsBoundary;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// `EMBEDDED_PDKS: &[(&str, &str)]` — every file under `pdks/`, baked in at build
// time so `--process <name>` needs no external files (see build.rs).
include!(concat!(env!("OUT_DIR"), "/embedded_pdks.rs"));

/// Lexically normalize a `/`-separated path: resolve `.` and `..` components.
/// Lets one embedded PDK reference another's files (e.g. a derived process reusing
/// `../ihp-sg13g2/decks/activ.yml`), mirroring what the filesystem does natively
/// for `PdkSource::Fs`.
fn normalize_path(p: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for c in p.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Look up an embedded PDK file by its path relative to `pdks/`.
fn embedded_file(rel: &str) -> Option<&'static str> {
    let rel = normalize_path(rel);
    EMBEDDED_PDKS.iter().find(|(k, _)| *k == rel).map(|(_, v)| *v)
}

/// Where a PDK's files come from, so deck files resolve the same way whether the
/// PDK was loaded by embedded process name or from a filesystem `pdk.yml`.
#[derive(Debug)]
enum PdkSource {
    /// Filesystem: `rel` is resolved against this directory (the `pdk.yml`'s dir).
    Fs(PathBuf),
    /// Embedded: files live at `pdks/<process>/<rel>`.
    Embedded(String),
}

impl PdkSource {
    fn read(&self, rel: &str) -> Result<String, String> {
        match self {
            PdkSource::Fs(dir) => {
                let p = dir.join(rel);
                std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))
            }
            PdkSource::Embedded(process) => {
                let key = format!("{process}/{rel}");
                embedded_file(&key)
                    .map(str::to_owned)
                    .ok_or_else(|| format!("embedded PDK file not found: {key}"))
            }
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Layer {
    pub name: String,
    pub gds_layer: u16,
    pub gds_datatype: u16,
}

/// How a virtual layer is evaluated.
#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VirtualMode {
    /// Materialised globally into the layout up front (default).  Required for
    /// layers consumed by whole-layout checks such as `inside_boundary`.
    #[default]
    Global,
    /// Built lazily per tile inside the merge cache from its source layers' tiles.
    /// Avoids a global boolean over a dense layer; use for layers consumed only by
    /// tiled geometric checks (e.g. `ContNoSealring`).
    Lazy,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VirtualLayerDef {
    pub name: String,
    pub op: String,
    pub layers: Vec<String>,
    #[serde(default)]
    pub mode: VirtualMode,
    /// Distance (µm) for parameterised ops such as `close` (the half-merge radius).
    #[serde(default)]
    pub radius: Option<f64>,
    /// Text pattern for the `with_text` op (exact match, or prefix if it ends in `*`).
    #[serde(default)]
    pub text: Option<String>,
}

/// A lazy virtual layer resolved to GDS numbers, ready for the merge cache.
#[derive(Debug)]
pub struct TiledVirtualSpec {
    /// The virtual layer's name (for diagnostics).
    pub name: String,
    /// Synthetic (layer, datatype) key the virtual is registered under.
    pub key: (i16, i16),
    /// Op name as written in the PDK; parsed to a `merge::VirtualOp` by `run_drc`.
    pub op: String,
    /// Resolved source layer keys.
    pub sources: Vec<(i16, i16)>,
    /// Radius (µm) for the parameterised ops (`close`/`open`/`grow`; for
    /// `holes`/`with_holes` it declares the max expected ring extent for halos).
    pub radius: Option<f64>,
    /// Text pattern for the `with_text` op.
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RuleRaw {
    pub id: String,
    pub check: String,
    pub layers: Vec<String>,
    pub value: f64,
    #[serde(default)]
    pub params: HashMap<String, f64>,
    /// Layer names whose shapes this rule should skip (e.g. inside_boundary not
    /// checking the edge-seal passivation ring).
    #[serde(default)]
    pub ignore: Vec<String>,
    /// Optional text/label pattern a check may need (e.g. the exemption label for
    /// `forbidden_unless_labeled`).  `params` only carries numbers.
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug)]
pub struct RuleDefinition {
    pub id: String,
    pub check: String,
    pub layers: Vec<Layer>,
    pub value: f64,
    pub params: HashMap<String, f64>,
    pub ignore: Vec<Layer>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeckRefRaw {
    pub name: String,
    pub path: String,
    /// Optional one-line human description (e.g. "Latch-up"), shown by `list-*`.
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct DeckRef {
    pub name: String,
    pub path: String, // relative to the PDK; resolved by `PdkSource` at load time
    pub description: Option<String>,
}

/// A suite file: a curated selection of rules imported from one or more decks.
#[derive(Debug, Deserialize)]
struct SuiteRaw {
    pub include: Vec<SuiteIncludeRaw>,
}

#[derive(Debug, Deserialize)]
struct SuiteIncludeRaw {
    /// Name of a deck to import rules from.
    pub deck: String,
    /// Whitelist of rule ids to keep; omit to import the whole deck.
    #[serde(default)]
    pub rules: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct PdkRaw {
    pub name: String,
    pub version: String,
    /// Optional base `pdk.yml` (path relative to this file) whose `layers` and
    /// `virtual_layers` are inherited — this PDK's own entries are appended after
    /// them.  Everything else (`decks`, `suites`, `connectivity`) always comes from
    /// this file, so a derived process (e.g. SG13CMOS5L extending SG13G2) states its
    /// own deck list and connect graph explicitly while reusing the big layer and
    /// recognition tables.  One level only: the base may not itself extend.
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default)]
    pub layers: Vec<Layer>,
    pub decks: Vec<DeckRefRaw>,
    /// Suites import a selection of rules from decks (e.g. `precheck`, `main`).
    #[serde(default)]
    pub suites: Vec<DeckRefRaw>,
    #[serde(default)]
    pub virtual_layers: Vec<VirtualLayerDef>,
    /// Electrical connect graph for net extraction (used by net-aware checks).
    #[serde(default)]
    pub connectivity: Vec<ConnectivityRaw>,
}

#[derive(Debug, Deserialize)]
struct ConnectivityRaw {
    /// Connector layer (a via or contact) that joins the conductors it overlaps.
    pub connector: String,
    /// Conductor layers the connector bridges (e.g. the two metals around a via).
    pub layers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DeckRaw {
    pub rules: Vec<RuleRaw>,
}

/// Synthetic GDS layer numbers for virtual layers start here.
/// Must not overlap with any real PDK layer number.
const VIRTUAL_LAYER_BASE: u16 = 30000;

/// Parse a raw `"layer/datatype"` string (e.g. `"134/30"`) into a `Layer`.
fn parse_layer_datatype(s: &str) -> Option<Layer> {
    let (l, d) = s.split_once('/')?;
    Some(Layer {
        name: s.to_string(),
        gds_layer: l.trim().parse().ok()?,
        gds_datatype: d.trim().parse().ok()?,
    })
}

#[derive(Debug)]
pub struct PdkConfig {
    pub name: String,
    pub version: String,
    pub decks: Vec<DeckRef>,
    /// Suites (curated rule selections) resolved by `load_suite`, alongside decks.
    pub suites: Vec<DeckRef>,
    pub virtual_layers: Vec<VirtualLayerDef>,
    /// Resolved connect graph for net extraction; empty if the PDK declares none.
    pub connectivity: Vec<crate::connectivity::ConnectSpec>,
    layer_map: HashMap<String, Layer>,
    source: PdkSource,
}

impl PdkConfig {
    /// Load a PDK by process name (embedded, e.g. `"ihp-sg13g2"`) or by path to a
    /// `pdk.yml` (for custom/out-of-tree PDKs).
    pub fn for_process(spec: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(content) = embedded_file(&format!("{spec}/pdk.yml")) {
            return Self::from_yaml(content, PdkSource::Embedded(spec.to_string()));
        }
        let content = std::fs::read_to_string(spec).map_err(|e| {
            format!("'{spec}' is not a known process and not a readable pdk.yml: {e}")
        })?;
        let dir = Path::new(spec).parent().unwrap_or(Path::new(".")).to_path_buf();
        Self::from_yaml(&content, PdkSource::Fs(dir))
    }

    /// Load a PDK from a filesystem `pdk.yml` path.
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let dir = Path::new(path).parent().unwrap_or(Path::new(".")).to_path_buf();
        Self::from_yaml(&content, PdkSource::Fs(dir))
    }

    fn from_yaml(content: &str, source: PdkSource) -> Result<Self, Box<dyn std::error::Error>> {
        let mut raw: PdkRaw = serde_yml::from_str(content)?;

        if let Some(base_rel) = raw.extends.take() {
            let base_content = source.read(&base_rel)?;
            let base: PdkRaw = serde_yml::from_str(&base_content)?;
            if base.extends.is_some() {
                return Err(format!(
                    "extends chain not supported: base '{base_rel}' itself extends another PDK"
                )
                .into());
            }
            let mut layers = base.layers;
            layers.extend(raw.layers);
            raw.layers = layers;
            // Base virtuals first, the child's appended; a child entry with the same
            // name *replaces* the base one (keep the last of each name).
            let mut virtuals = base.virtual_layers;
            virtuals.extend(raw.virtual_layers);
            virtuals.reverse();
            let mut seen: HashSet<String> = HashSet::new();
            virtuals.retain(|v| seen.insert(v.name.clone()));
            virtuals.reverse();
            raw.virtual_layers = virtuals;
        }

        let mut layer_map: HashMap<String, Layer> = raw
            .layers
            .into_iter()
            .map(|l| (l.name.clone(), l))
            .collect();

        // Register virtual layers in the layer map with synthetic GDS numbers
        // so that rules can reference them by name just like real layers.
        for (i, vl) in raw.virtual_layers.iter().enumerate() {
            layer_map.insert(vl.name.clone(), Layer {
                name: vl.name.clone(),
                gds_layer: VIRTUAL_LAYER_BASE + i as u16,
                gds_datatype: 0,
            });
        }

        // Deck paths stay relative to the PDK; the source resolves them at load.
        let decks = raw
            .decks
            .into_iter()
            .map(|d| DeckRef { name: d.name, path: d.path, description: d.description })
            .collect();

        let suites = raw
            .suites
            .into_iter()
            .map(|s| DeckRef { name: s.name, path: s.path, description: s.description })
            .collect();

        // Resolve the connect graph's layer names to (layer, datatype) keys.  A spec
        // referencing an unknown layer, or with fewer than two resolved conductors, is
        // skipped — net extraction simply won't bridge through it.
        let resolve = |name: &str| {
            layer_map
                .get(name)
                .map(|l| (l.gds_layer as i16, l.gds_datatype as i16))
        };
        let connectivity = raw
            .connectivity
            .into_iter()
            .filter_map(|c| {
                let connector = resolve(&c.connector)?;
                let layers: Vec<_> = c.layers.iter().filter_map(|n| resolve(n)).collect();
                // A step joins the connector to the layers it overlaps; need at least one.
                (!layers.is_empty()).then_some(crate::connectivity::ConnectSpec { connector, layers })
            })
            .collect();

        Ok(PdkConfig {
            name: raw.name,
            version: raw.version,
            decks,
            suites,
            virtual_layers: raw.virtual_layers,
            connectivity,
            layer_map,
            source,
        })
    }

    pub fn layer(&self, name: &str) -> Option<&Layer> {
        self.layer_map.get(name)
    }

    /// Convert a merged region's **outer ring** to a closed `GdsBoundary` on
    /// `(layer, dt)`.  A single GDS boundary cannot represent holes; `context` (the
    /// virtual layer's name) enables a warning when any are dropped — pass `None`
    /// where dropping them is intentional (e.g. filling a ring's interior).
    fn merged_outer_boundary(
        m: &crate::merge::MergedPoly,
        layer: i16,
        dt: i16,
        context: Option<&str>,
    ) -> GdsBoundary {
        if let Some(name) = context.filter(|_| !m.holes.is_empty()) {
            eprintln!(
                "Virtual layer '{name}': result polygon has {} hole(s); holes are not represented",
                m.holes.len()
            );
        }
        let mut xy: Vec<gds21::GdsPoint> =
            m.outer.iter().map(|p| gds21::GdsPoint::new(p.x, p.y)).collect();
        if let Some(first) = xy.first().cloned() {
            xy.push(first); // close the ring (GDS convention)
        }
        GdsBoundary { layer, datatype: dt, xy, ..Default::default() }
    }

    /// Compute virtual layers from the layout and insert them into the layout.
    /// Uses only the boundaries already in the layout so virtual layers cannot
    /// reference each other.
    pub fn compute_virtual_layers(&self, layout: &mut FlatLayout, dbu_to_um: f64) {
        let mut to_insert: Vec<(i16, i16, GdsBoundary)> = vec![];

        for vl_def in &self.virtual_layers {
            // Lazy layers are built per tile in the merge cache, not materialised here.
            if vl_def.mode == VirtualMode::Lazy {
                continue;
            }
            let Some(vl_layer) = self.layer_map.get(&vl_def.name) else {
                continue;
            };
            let vl_gds = vl_layer.gds_layer as i16;
            let vl_dt  = vl_layer.gds_datatype as i16;

            match vl_def.op.as_str() {
                "union" => {
                    // seen_layers: skip if two names resolve to the same GDS layer/datatype.
                    // seen_shapes: skip identical polygons contributed by multiple source layers
                    //   (e.g. Passiv.sbump and dfpad may carry the same pad shapes).
                    let mut seen_layers: HashSet<(i16, i16)> = HashSet::new();
                    let mut seen_shapes: HashSet<Vec<(i32, i32)>> = HashSet::new();
                    for src_name in &vl_def.layers {
                        let Some(src) = self.layer_map.get(src_name) else {
                            eprintln!(
                                "Virtual layer '{}': source layer '{}' not found",
                                vl_def.name, src_name
                            );
                            continue;
                        };
                        let src_gds = src.gds_layer as i16;
                        let src_dt  = src.gds_datatype as i16;
                        if !seen_layers.insert((src_gds, src_dt)) {
                            continue;
                        }

                        for b in layout.get(src_gds, src_dt) {
                            let key: Vec<(i32, i32)> =
                                b.xy.iter().map(|p| (p.x, p.y)).collect();
                            if seen_shapes.insert(key) {
                                to_insert.push((vl_gds, vl_dt, GdsBoundary {
                                    layer: vl_gds,
                                    datatype: vl_dt,
                                    xy: b.xy.clone(),
                                    ..Default::default()
                                }));
                            }
                        }
                    }
                }
                "intersection" | "and" => {
                    // Geometric AND of the source layers (device recognition,
                    // e.g. CuPillarPad = Passiv.pillar AND dfpad).  If any source
                    // layer is missing or empty, the intersection is empty.
                    let mut srcs: Vec<&[GdsBoundary]> = Vec::with_capacity(vl_def.layers.len());
                    let mut ok = true;
                    for src_name in &vl_def.layers {
                        let Some(src) = self.layer_map.get(src_name) else {
                            eprintln!(
                                "Virtual layer '{}': source layer '{}' not found",
                                vl_def.name, src_name
                            );
                            ok = false;
                            break;
                        };
                        srcs.push(layout.get(src.gds_layer as i16, src.gds_datatype as i16));
                    }
                    if ok {
                        for m in crate::merge::intersect_layers(&srcs) {
                            to_insert.push((vl_gds, vl_dt,
                                Self::merged_outer_boundary(&m, vl_gds, vl_dt, Some(&vl_def.name))));
                        }
                    }
                }
                "difference" | "not" => {
                    // Geometric NOT: the first layer minus all the rest (e.g.
                    // `ContNoSealring = Cont NOT EdgeSeal` removes the seal ring).
                    let Some((base_name, clip_names)) = vl_def.layers.split_first() else {
                        eprintln!("Virtual layer '{}': difference needs at least one layer", vl_def.name);
                        continue;
                    };
                    let Some(base) = self.layer_map.get(base_name) else {
                        eprintln!("Virtual layer '{}': source layer '{}' not found", vl_def.name, base_name);
                        continue;
                    };
                    let base_b = layout.get(base.gds_layer as i16, base.gds_datatype as i16);

                    let mut clips: Vec<&[GdsBoundary]> = Vec::with_capacity(clip_names.len());
                    let mut ok = true;
                    for name in clip_names {
                        let Some(c) = self.layer_map.get(name) else {
                            eprintln!("Virtual layer '{}': source layer '{}' not found", vl_def.name, name);
                            ok = false;
                            break;
                        };
                        clips.push(layout.get(c.gds_layer as i16, c.gds_datatype as i16));
                    }
                    if ok {
                        for m in crate::merge::difference_layers(base_b, &clips) {
                            to_insert.push((vl_gds, vl_dt,
                                Self::merged_outer_boundary(&m, vl_gds, vl_dt, Some(&vl_def.name))));
                        }
                    }
                }
                "inside" => {
                    // `inside(target, ring)` = the part of `target` (layers[0]) that
                    // lies within the area enclosed by `ring` (layers[1]).  The ring's
                    // holes are filled, so a seal *frame* becomes "seal + interior" —
                    // there is no drawn layer for that region, so we derive it here.
                    // Both sources are real layers, so no virtual layer references
                    // another (which `compute_virtual_layers` does not support).
                    if vl_def.layers.len() < 2 {
                        eprintln!("Virtual layer '{}': inside needs 2 layers (target, ring)", vl_def.name);
                        continue;
                    }
                    let (Some(target), Some(ring)) = (
                        self.layer_map.get(&vl_def.layers[0]),
                        self.layer_map.get(&vl_def.layers[1]),
                    ) else {
                        eprintln!("Virtual layer '{}': a source layer was not found", vl_def.name);
                        continue;
                    };
                    // Fill the ring: each merged region's outer contour as a solid
                    // polygon (its holes dropped).
                    let ring_solid: Vec<GdsBoundary> = crate::merge::merge_boundaries(
                        layout.get(ring.gds_layer as i16, ring.gds_datatype as i16),
                    )
                    .iter()
                    .map(|m| Self::merged_outer_boundary(m, 0, 0, None))
                    .collect();

                    let target_b = layout.get(target.gds_layer as i16, target.gds_datatype as i16);
                    for m in crate::merge::intersect_layers(&[target_b, &ring_solid]) {
                        to_insert.push((vl_gds, vl_dt,
                            Self::merged_outer_boundary(&m, vl_gds, vl_dt, Some(&vl_def.name))));
                    }
                }
                "close" => {
                    // Morphological closing of one layer: merge regions whose gap is below
                    // `radius * 2` µm (the "same-net merge" of NW.b / NBL.b).  Convex regions
                    // that stay separate keep their outer edges, so a downstream `min_space`
                    // sees the true gap between distinct (merged) regions.
                    let (Some(src_name), Some(radius_um)) = (vl_def.layers.first(), vl_def.radius)
                    else {
                        eprintln!("Virtual layer '{}': close needs one layer and a radius", vl_def.name);
                        continue;
                    };
                    let Some(src) = self.layer_map.get(src_name) else {
                        eprintln!("Virtual layer '{}': source layer '{}' not found", vl_def.name, src_name);
                        continue;
                    };
                    let merged = crate::merge::merge_boundaries(
                        layout.get(src.gds_layer as i16, src.gds_datatype as i16),
                    );
                    let radius_dbu = radius_um / dbu_to_um;
                    for m in crate::merge::closing(&merged, radius_dbu) {
                        to_insert.push((vl_gds, vl_dt,
                            Self::merged_outer_boundary(&m, vl_gds, vl_dt, Some(&vl_def.name))));
                    }
                }
                other => {
                    eprintln!(
                        "Virtual layer '{}': unsupported op '{}' (supported: union, intersection, difference, inside, close)",
                        vl_def.name, other
                    );
                }
            }
        }

        for (layer, dt, b) in to_insert {
            layout.insert(layer, dt, b);
        }
    }

    /// Lazy (tiled) virtual layers, resolved to GDS numbers: `(synthetic key, op,
    /// source keys)`.  `run_drc` registers these with the merge cache so they are
    /// built per tile on demand.  Sources/keys that don't resolve are skipped.
    pub fn tiled_virtual_layers(&self) -> Vec<TiledVirtualSpec> {
        let key = |name: &str| self.layer_map.get(name).map(|l| (l.gds_layer as i16, l.gds_datatype as i16));
        let mut out = Vec::new();
        for vl in &self.virtual_layers {
            if vl.mode != VirtualMode::Lazy {
                continue;
            }
            let Some(vkey) = key(&vl.name) else { continue };
            let mut sources = Vec::with_capacity(vl.layers.len());
            let mut ok = true;
            for s in &vl.layers {
                match key(s) {
                    Some(k) => sources.push(k),
                    None => {
                        eprintln!("Lazy virtual layer '{}': source layer '{}' not found", vl.name, s);
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                out.push(TiledVirtualSpec {
                    name: vl.name.clone(),
                    key: vkey,
                    op: vl.op.clone(),
                    sources,
                    radius: vl.radius,
                    text: vl.text.clone(),
                });
            }
        }
        out
    }

    /// Expand a suite into the concatenated rules of the decks it imports, keeping
    /// only the whitelisted rule ids where an include specifies them.
    pub fn load_suite(&self, suite_name: &str) -> Result<Vec<RuleDefinition>, Box<dyn std::error::Error>> {
        let suite_ref = self
            .suites
            .iter()
            .find(|s| s.name == suite_name)
            .ok_or_else(|| format!("Suite '{suite_name}' not found in PDK '{}'", self.name))?;

        let content = self.source.read(&suite_ref.path)?;
        let raw: SuiteRaw = serde_yml::from_str(&content)?;

        let mut rules = Vec::new();
        for inc in &raw.include {
            rules.extend(self.load_deck_filtered(&inc.deck, inc.rules.as_deref())?);
        }
        Ok(rules)
    }

    pub fn load_deck(&self, deck_name: &str) -> Result<Vec<RuleDefinition>, Box<dyn std::error::Error>> {
        self.load_deck_filtered(deck_name, None)
    }

    /// Load a deck's rules, optionally restricted to a whitelist of rule ids (a suite
    /// import).  Every id in `only` must match at least one rule in the deck — an
    /// unknown id errors loudly so a suite typo can't silently drop a check.  An id
    /// may match several rules (e.g. `M1.b` is both `min_space` and `min_notch`);
    /// all matching entries are kept.
    fn load_deck_filtered(
        &self,
        deck_name: &str,
        only: Option<&[String]>,
    ) -> Result<Vec<RuleDefinition>, Box<dyn std::error::Error>> {
        let deck_ref = self
            .decks
            .iter()
            .find(|d| d.name == deck_name)
            .ok_or_else(|| format!("Deck '{deck_name}' not found in PDK '{}'", self.name))?;

        let content = self.source.read(&deck_ref.path)?;
        let mut raw: DeckRaw = serde_yml::from_str(&content)?;

        if let Some(ids) = only {
            let present: HashSet<&str> = raw.rules.iter().map(|r| r.id.as_str()).collect();
            if let Some(missing) = ids.iter().find(|w| !present.contains(w.as_str())) {
                return Err(format!(
                    "Suite references rule '{missing}' not found in deck '{deck_name}'"
                )
                .into());
            }
            raw.rules.retain(|r| ids.iter().any(|w| w == &r.id));
        }

        let rules = raw
            .rules
            .into_iter()
            .map(|r| {
                if r.layers.is_empty() {
                    return Err(format!("Rule '{}' must define at least one layer", r.id));
                }
                let layers = r.layers
                    .iter()
                    .map(|name| {
                        self.layer_map
                            .get(name)
                            .ok_or_else(|| format!("Rule '{}' references unknown layer '{}'", r.id, name))
                            .cloned()
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                // `ignore` is best-effort: each entry is a layer name or a raw
                // `layer/datatype` pair (for GDS layers the PDK doesn't name).
                let ignore = r.ignore
                    .iter()
                    .filter_map(|name| {
                        if let Some(l) = self.layer_map.get(name) {
                            return Some(l.clone());
                        }
                        if let Some(l) = parse_layer_datatype(name) {
                            return Some(l);
                        }
                        eprintln!("Rule '{}' ignore references unknown layer '{}'", r.id, name);
                        None
                    })
                    .collect();

                Ok(RuleDefinition {
                    id: r.id,
                    check: r.check,
                    layers,
                    value: r.value,
                    params: r.params,
                    ignore,
                    text: r.text,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(rules)
    }
}
