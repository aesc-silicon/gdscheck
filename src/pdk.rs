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
    EMBEDDED_PDKS
        .iter()
        .find(|(k, _)| *k == rel)
        .map(|(_, v)| *v)
}

/// Where a PDK's files come from, so deck files resolve the same way whether the
/// PDK was loaded by embedded process name or from a filesystem `pdk.yml`.
#[derive(Debug)]
enum PdkSource {
    /// Filesystem: `rel` is resolved against `dir` (the `pdk.yml`'s directory).  What
    /// the tree lacks is read from the embedded PDKs as if `dir` sat among them, so an
    /// out-of-tree PDK reaches a bundled base and its decks by the same `../<name>/…`
    /// paths a bundled derivative uses.
    Fs { dir: PathBuf, name: String },
    /// Embedded: files live at `pdks/<process>/<rel>`.
    Embedded(String),
}

impl PdkSource {
    fn fs(dir: PathBuf) -> Self {
        let name = dir
            .canonicalize()
            .ok()
            .and_then(|d| d.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_default();
        PdkSource::Fs { dir, name }
    }

    fn read(&self, rel: &str) -> Result<String, String> {
        match self {
            PdkSource::Fs { dir, name } => {
                let p = dir.join(rel);
                match std::fs::read_to_string(&p) {
                    Ok(s) => Ok(s),
                    Err(e) => embedded_file(&format!("{name}/{rel}"))
                        .map(str::to_owned)
                        .ok_or_else(|| format!("{}: {e}", p.display())),
                }
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

/// Where a process was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Built into the binary.
    Embedded,
    /// A `pdk.yml` on the filesystem: given as a path, or found under a search directory.
    External(PathBuf),
}

/// A process spec resolved to what [`PdkConfig::for_process`] loads: the embedded name,
/// or the path of the `pdk.yml` found for it.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub spec: String,
    pub origin: Origin,
}

/// The environment variable holding extra PDK directories, separated like `PATH`.
pub const PDK_PATH_VAR: &str = "GDSCHECK_PDK_PATH";

/// The directories a process name is looked up in, in order: `extra` (a command line's
/// `--pdk-path`), then [`PDK_PATH_VAR`].  Each holds one directory per process, with its
/// `pdk.yml` inside, the way `pdks/` does.
pub fn search_dirs(extra: &[PathBuf]) -> Vec<PathBuf> {
    let env = std::env::var_os(PDK_PATH_VAR).unwrap_or_default();
    dirs_from(extra, &env)
}

fn dirs_from(extra: &[PathBuf], env: &std::ffi::OsStr) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = extra.to_vec();
    out.extend(std::env::split_paths(env).filter(|p| !p.as_os_str().is_empty()));
    let mut seen = HashSet::new();
    out.retain(|d| seen.insert(d.clone()));
    out
}

/// Resolve a process spec: a path to a readable `pdk.yml`, a name found as
/// `<dir>/<name>/pdk.yml` under the search directories, or an embedded name, in that
/// order - so a PDK on the search path shadows a bundled one of the same name.
pub fn resolve_process(spec: &str, extra: &[PathBuf]) -> Result<Resolved, String> {
    if Path::new(spec).is_file() {
        return Ok(Resolved {
            spec: spec.to_string(),
            origin: Origin::External(PathBuf::from(spec)),
        });
    }
    let dirs = search_dirs(extra);
    if !spec.contains('/') && !spec.contains(std::path::MAIN_SEPARATOR) {
        for dir in &dirs {
            let p = dir.join(spec).join("pdk.yml");
            if p.is_file() {
                return Ok(Resolved {
                    spec: p.to_string_lossy().into_owned(),
                    origin: Origin::External(p),
                });
            }
        }
        if embedded_file(&format!("{spec}/pdk.yml")).is_some() {
            return Ok(Resolved {
                spec: spec.to_string(),
                origin: Origin::Embedded,
            });
        }
    }
    let searched = if dirs.is_empty() {
        String::new()
    } else {
        format!(
            ", not found under {}",
            dirs.iter()
                .map(|d| d.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    Err(format!(
        "'{spec}' is not an embedded process ({}){searched}, and not a readable pdk.yml",
        PdkConfig::embedded_processes().join(", ")
    ))
}

/// Every process there is: the ones under the search directories, in the order they are
/// searched, then the embedded ones.  A name listed twice is shadowed by its first entry.
pub fn list_processes(extra: &[PathBuf]) -> Vec<(String, Origin)> {
    let mut out: Vec<(String, Origin)> = Vec::new();
    for dir in search_dirs(extra) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut found: Vec<(String, PathBuf)> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.join("pdk.yml").is_file())
            .filter_map(|p| {
                p.file_name()
                    .map(|n| (n.to_string_lossy().into_owned(), p.join("pdk.yml")))
            })
            .collect();
        found.sort();
        out.extend(found.into_iter().map(|(n, p)| (n, Origin::External(p))));
    }
    out.extend(
        PdkConfig::embedded_processes()
            .into_iter()
            .map(|n| (n.to_string(), Origin::Embedded)),
    );
    out
}

#[derive(Debug, Deserialize, Clone)]
pub struct Layer {
    pub name: String,
    pub gds_layer: u16,
    pub gds_datatype: u16,
}

/// A derived region layer: one operation of a sentence under `virtual_layers:` (see
/// [`crate::expr`]), lowered to the op and the source names the merge cache builds it
/// from.
#[derive(Debug, Clone)]
pub struct VirtualLayerDef {
    pub name: String,
    pub op: String,
    pub layers: Vec<String>,
    /// Distance (µm) for parameterised ops such as `close` (the half-merge radius).
    pub radius: Option<f64>,
    /// Text pattern for the `with_text` op (exact match, or prefix if it ends in `*`).
    pub text: Option<String>,
    /// Inclusive lower bound (µm) for the `with_bbox_min`/`with_bbox_max` filters;
    /// absent means unbounded below.
    pub min: Option<f64>,
    /// Exclusive upper bound (µm) for the `with_bbox_min`/`with_bbox_max` filters;
    /// absent means unbounded above.
    pub max: Option<f64>,
    /// Extra reach (µm) for `grow`, beyond its radius.  Absent means none.
    ///
    /// Wanted only when the grown layer is a *selection radius* — "everything within X of
    /// this" — because a shape at exactly X then merely touches the grown region, and a
    /// whole-region test reads a zero-area touch as no overlap.  A hair of slack turns
    /// that into a hairline overlap and the shape is selected.
    ///
    /// Wanted nowhere else, and it used to be the default: a band that decides which of
    /// two limits applies, or a region a rule must not reach into, is judged wrong by any
    /// slack at all.  That cost five rules across four decks before the default was
    /// flipped, each one over-reporting plausibly rather than failing.  A sentence says
    /// it with the word `within` in place of `grow`.
    pub slack: Option<f64>,
}

/// A derived *edge* layer: boundary segments rather than regions.  One operation of a
/// sentence whose left side is an edge layer, or `edges`/`width_below` on a region.
///
/// `min`/`max` carry the op's bounds — µm for the length filters, degrees for the angle
/// ones — and are ignored by the ops that take none.
#[derive(Debug, Clone)]
pub struct EdgeLayerDef {
    pub name: String,
    pub op: String,
    pub layers: Vec<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// `centers` only: the middle part to keep, as a fraction of each edge's length.
    pub fraction: Option<f64>,
}

/// An edge layer resolved to GDS numbers, ready for the merge cache.
#[derive(Debug)]
pub struct TiledEdgeSpec {
    pub name: String,
    pub key: (i16, i16),
    pub op: String,
    pub sources: Vec<(i16, i16)>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub fraction: Option<f64>,
}

/// A lazy virtual layer resolved to GDS numbers, ready for the merge cache.
#[derive(Debug, Clone)]
pub struct TiledVirtualSpec {
    /// The virtual layer's name (for diagnostics).
    pub name: String,
    /// Synthetic (layer, datatype) key the virtual is registered under.
    pub key: (i16, i16),
    /// Op name as written in the PDK; parsed to a `merge::VirtualOp` by `run_drc`.
    pub op: String,
    /// Resolved source layer keys.
    pub sources: Vec<(i16, i16)>,
    /// Radius (µm) for the parameterised ops (`close`/`open`/`grow`).
    pub radius: Option<f64>,
    /// Text pattern for the `with_text` op.
    pub text: Option<String>,
    /// Extra reach (µm) for `grow`; see [`VirtualLayerDef::slack`].
    pub slack: Option<f64>,
    /// Bounding-box side bounds (µm) for the `with_bbox_min`/`with_bbox_max` filters.
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RuleRaw {
    pub id: String,
    pub check: String,
    pub layers: Vec<String>,
    pub value: f64,
    /// A number or a word, as the deck writes it: `rows: 3`, `sides: adjacent`.
    #[serde(default)]
    pub params: HashMap<String, Param>,
    /// Layer names whose shapes this rule should skip (e.g. a `forbidden` past the
    /// edge seal not checking its passivation ring).
    #[serde(default)]
    pub ignore: Vec<String>,
    /// Optional text/label pattern a check may need (e.g. the exemption label for
    /// `forbidden_unless_labeled`).
    #[serde(default)]
    pub text: Option<String>,
    /// Params whose value is a *layer*, given by name.  A check that takes a layer as a
    /// parameter (rather than as one of `layers`) would otherwise need its GDS number
    /// written into the deck - impossible for a derived layer, whose number is assigned
    /// by position in `virtual_layers`.  A layer name and a mode word look alike in YAML,
    /// so these are their own block; each entry is resolved at load time into `params`
    /// as `<name>` and `<name>_dt`.
    #[serde(default)]
    pub layer_params: HashMap<String, String>,
}

/// A rule parameter: the number or the word the deck wrote.  YAML decides which -
/// `0.5` is a number, `bent` a word - and a check asks for the kind it wants through
/// [`RuleDefinition::num`] or [`RuleDefinition::word`], which say so when the deck gave
/// the other.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Param {
    Num(f64),
    Word(String),
}

impl std::fmt::Display for Param {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Param::Num(v) => write!(f, "{v}"),
            Param::Word(w) => f.write_str(w),
        }
    }
}

#[derive(Debug)]
pub struct RuleDefinition {
    pub id: String,
    pub check: String,
    pub layers: Vec<Layer>,
    pub value: f64,
    /// See [`Param`].  Read through [`Self::num`] and [`Self::word`].
    pub params: HashMap<String, Param>,
    pub ignore: Vec<Layer>,
    pub text: Option<String>,
}

impl RuleDefinition {
    /// The numeric param `key`, or `None` - saying so if the deck wrote a word there.
    pub fn num(&self, key: &str) -> Option<f64> {
        match self.params.get(key)? {
            Param::Num(v) => Some(*v),
            Param::Word(w) => {
                eprintln!(
                    "[{}] {}: param `{key}` must be a number, not `{w}`",
                    self.id, self.check
                );
                None
            }
        }
    }

    /// The word param `key`, or `None` - saying so if the deck wrote a number there.
    pub fn word(&self, key: &str) -> Option<&str> {
        match self.params.get(key)? {
            Param::Word(w) => Some(w),
            Param::Num(v) => {
                eprintln!(
                    "[{}] {}: param `{key}` must be a word, not `{v}`",
                    self.id, self.check
                );
                None
            }
        }
    }
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
    /// Blacklist of rule ids to drop after the whitelist is applied; omit to keep
    /// everything.  Useful for "whole deck minus a handful" imports (e.g. a `core`
    /// suite that takes every rule except the density checks).
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct PdkRaw {
    pub name: String,
    pub version: String,
    /// Optional base `pdk.yml` (path relative to this file, or a bare process name for
    /// the base beside it) whose `layers` and `virtual_layers` are inherited — this
    /// PDK's own entries are appended after them, and one under a base name replaces
    /// the base one.  Everything else (`decks`, `suites`, `connectivity`) always comes from
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
    /// Derived layers, region and edge alike: a name and the sentence that makes it,
    /// in the order the file gives them.  See [`crate::expr`].
    #[serde(default)]
    pub virtual_layers: serde_norway::Mapping,
    /// Electrical connect graph for net extraction (used by net-aware checks).
    #[serde(default)]
    pub connectivity: Vec<ConnectivityRaw>,
    /// Cells checked as delivered; a base PDK's waivers are inherited.
    #[serde(default)]
    pub waivers: Vec<Waiver>,
}

/// A waiver: violations of the named rules whose marker lies inside a placed instance
/// of a matching cell are reported, but as waived.  A PDK states these for the cells
/// it ships checked as delivered - a foundry's pad and IO library - and nothing else
/// does: a waiver is a statement about geometry, not a run option, so it is not on
/// the command line where it would be reached for instead of a fix.
#[derive(Debug, Deserialize, Clone)]
pub struct Waiver {
    /// Cell name patterns; `*` matches any run of characters.
    pub cells: Vec<String>,
    /// Rule ids the waiver covers; absent means every rule.
    #[serde(default)]
    pub rules: Option<Vec<String>>,
    #[serde(default)]
    pub reason: String,
}

impl Waiver {
    pub fn matches_cell(&self, cell: &str) -> bool {
        self.cells.iter().any(|p| glob_matches(p, cell))
    }

    pub fn covers_rule(&self, rule_id: &str) -> bool {
        self.rules
            .as_ref()
            .is_none_or(|ids| ids.iter().any(|id| id == rule_id))
    }
}

/// `pattern` against `s`, where `*` stands for any run of characters, including none.
pub fn glob_matches(pattern: &str, s: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == s;
    }
    let mut rest = s;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            let Some(r) = rest.strip_prefix(part) else {
                return false;
            };
            rest = r;
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else if part.is_empty() {
            continue;
        } else {
            let Some(at) = rest.find(part) else {
                return false;
            };
            rest = &rest[at + part.len()..];
        }
    }
    true
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
    /// Derived edge layers, referenced by rules like any other layer.
    pub edge_layers: Vec<EdgeLayerDef>,
    /// Resolved connect graph for net extraction; empty if the PDK declares none.
    pub connectivity: Vec<crate::connectivity::ConnectSpec>,
    /// Cells whose violations are reported waived (see [`Waiver`]).
    pub waivers: Vec<Waiver>,
    layer_map: HashMap<String, Layer>,
    source: PdkSource,
}

impl PdkConfig {
    /// Every embedded process name, i.e. every directory under `pdks/` holding a
    /// `pdk.yml`.  Sorted, so callers that iterate are deterministic.
    pub fn embedded_processes() -> Vec<&'static str> {
        let mut v: Vec<&'static str> = EMBEDDED_PDKS
            .iter()
            .filter_map(|(path, _)| path.strip_suffix("/pdk.yml"))
            .filter(|p| !p.contains('/'))
            .collect();
        v.sort_unstable();
        v
    }

    /// Load a PDK by process spec, as [`resolve_process`] reads it with no extra search
    /// directories: a path to a `pdk.yml`, a name under [`PDK_PATH_VAR`], or an embedded
    /// name such as `"ihp-sg13g2"`.
    pub fn for_process(spec: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let resolved = resolve_process(spec, &[])?;
        match resolved.origin {
            Origin::Embedded => {
                let content = embedded_file(&format!("{spec}/pdk.yml")).expect("resolved");
                Self::from_yaml(content, PdkSource::Embedded(spec.to_string()))
            }
            Origin::External(_) => Self::load(&resolved.spec),
        }
    }

    /// Load a PDK from a filesystem `pdk.yml` path.
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let dir = Path::new(path)
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        Self::from_yaml(&content, PdkSource::fs(dir))
    }

    fn from_yaml(content: &str, source: PdkSource) -> Result<Self, Box<dyn std::error::Error>> {
        let mut raw: PdkRaw = serde_norway::from_str(content)?;

        if let Some(base_rel) = raw.extends.take() {
            // A bare name is the base beside this PDK, bundled or not:
            // `extends: ihp-sg13g2` reads as `../ihp-sg13g2/pdk.yml`.
            let base_rel = if base_rel.contains('/') || base_rel.ends_with(".yml") {
                base_rel
            } else {
                format!("../{base_rel}/pdk.yml")
            };
            let base_content = source.read(&base_rel)?;
            let base: PdkRaw = serde_norway::from_str(&base_content)?;
            if base.extends.is_some() {
                return Err(format!(
                    "extends chain not supported: base '{base_rel}' itself extends another PDK"
                )
                .into());
            }
            let mut layers = base.layers;
            layers.extend(raw.layers);
            raw.layers = layers;
            let mut waivers = base.waivers;
            waivers.extend(raw.waivers);
            raw.waivers = waivers;
            // Base derivations first, the child's appended; a child entry under a base
            // name *replaces* the base one, in the child's position.
            let mut merged = base.virtual_layers;
            for (k, v) in raw.virtual_layers {
                merged.shift_remove(&k);
                merged.insert(k, v);
            }
            raw.virtual_layers = merged;
        }

        // The derived layers, as sentences.  A name may be declared once: a virtual
        // layer's synthetic number comes from its position, so a second declaration
        // under the same name would leave the first one built but unreachable and every
        // rule naming it silently measuring the other.  (YAML itself refuses a key given
        // twice in one mapping; this catches a sentence named like a drawn layer.)
        let mut sentences: Vec<(String, String)> = Vec::with_capacity(raw.virtual_layers.len());
        for (k, v) in &raw.virtual_layers {
            let (Some(name), Some(sentence)) = (k.as_str(), v.as_str()) else {
                return Err(format!(
                    "virtual layer {k:?}: a name and a sentence, as `name: a and b`"
                )
                .into());
            };
            if crate::expr::is_op_word(name) {
                return Err(
                    format!("virtual layer '{name}': `{name}` is an operation word").into(),
                );
            }
            sentences.push((name.to_string(), sentence.to_string()));
        }
        let mut declared: HashSet<&str> = HashSet::new();
        for name in raw
            .layers
            .iter()
            .map(|l| l.name.as_str())
            .chain(sentences.iter().map(|(n, _)| n.as_str()))
        {
            if !declared.insert(name) {
                return Err(format!("layer '{name}' is declared more than once").into());
            }
        }

        let mut layer_map: HashMap<String, Layer> = raw
            .layers
            .into_iter()
            .map(|l| (l.name.clone(), l))
            .collect();

        // What kind of layer each name is, so a sentence can be typed against the names
        // it uses - including ones declared after it.  Every drawn layer is a region; a
        // sentence's kind follows from its words, read with the table so far, and the
        // passes repeat until a pass changes nothing, so a chain of forward references
        // settles however long it is.  A sentence that does not type yet is left as a
        // region for now; the lowering below reports what is really wrong with it.
        let mut kinds: HashMap<String, crate::expr::Kind> = layer_map
            .keys()
            .map(|n| (n.clone(), crate::expr::Kind::Poly))
            .collect();
        for (name, _) in &sentences {
            kinds.insert(name.clone(), crate::expr::Kind::Poly);
        }
        for _ in 0..=sentences.len() {
            let mut changed = false;
            for (name, sentence) in &sentences {
                let lookup = |n: &str| kinds.get(n).copied();
                let kind = crate::expr::kind_of_sentence(sentence, &lookup)
                    .unwrap_or(crate::expr::Kind::Poly);
                if kinds.insert(name.clone(), kind) != Some(kind) {
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // Lower every sentence to the derived layers it needs, and register each under a
        // synthetic GDS number in the order lowered, so a rule names one exactly as it
        // names a drawn layer and the cache decides which kind it is.
        let mut virtual_layers = Vec::new();
        let mut edge_layers = Vec::new();
        let mut next = VIRTUAL_LAYER_BASE;
        for (name, sentence) in &sentences {
            let lookup = |n: &str| kinds.get(n).copied();
            let defs = crate::expr::lower(name, sentence, &lookup)
                .map_err(|e| format!("virtual layer '{name}': {e}"))?;
            for def in defs {
                layer_map.insert(
                    def.name().to_string(),
                    Layer {
                        name: def.name().to_string(),
                        gds_layer: next,
                        gds_datatype: 0,
                    },
                );
                next += 1;
                match def {
                    crate::expr::Def::Poly(v) => virtual_layers.push(v),
                    crate::expr::Def::Edge(e) => edge_layers.push(e),
                }
            }
        }

        // Deck paths stay relative to the PDK; the source resolves them at load.
        let decks = raw
            .decks
            .into_iter()
            .map(|d| DeckRef {
                name: d.name,
                path: d.path,
                description: d.description,
            })
            .collect();

        let suites = raw
            .suites
            .into_iter()
            .map(|s| DeckRef {
                name: s.name,
                path: s.path,
                description: s.description,
            })
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
                (!layers.is_empty())
                    .then_some(crate::connectivity::ConnectSpec { connector, layers })
            })
            .collect();

        Ok(PdkConfig {
            name: raw.name,
            version: raw.version,
            decks,
            suites,
            virtual_layers,
            edge_layers,
            connectivity,
            waivers: raw.waivers,
            layer_map,
            source,
        })
    }

    pub fn layer(&self, name: &str) -> Option<&Layer> {
        self.layer_map.get(name)
    }

    /// Every layer the PDK names, drawn and derived, by name.
    pub fn layers(&self) -> impl Iterator<Item = (&str, &Layer)> {
        self.layer_map.iter().map(|(n, l)| (n.as_str(), l))
    }

    /// The virtual layers `rules` need materialised in the layout rather than built per
    /// tile: the ones a whole-layout check reads (see
    /// [`crate::checks::reads_layout`]) and the ones whose op has no tiled
    /// form.  A layer that must be eager but cannot be - its op only exists tiled, or it
    /// is built from another virtual layer - is an error naming the rule that asked.
    pub fn eager_layers(&self, rules: &[RuleDefinition]) -> Result<HashSet<String>, String> {
        let by_name: HashMap<&str, &VirtualLayerDef> = self
            .virtual_layers
            .iter()
            .map(|v| (v.name.as_str(), v))
            .collect();
        let mut eager: HashMap<String, String> = HashMap::new();
        for v in &self.virtual_layers {
            if v.op == "inside_ring" {
                eager.insert(v.name.clone(), "its op, inside_ring".into());
            }
        }
        for rule in rules {
            if !crate::checks::reads_layout(rule) {
                continue;
            }
            for l in rule.layers.iter().chain(rule.ignore.iter()) {
                if by_name.contains_key(l.name.as_str()) {
                    eager
                        .entry(l.name.clone())
                        .or_insert_with(|| format!("rule {} ({})", rule.id, rule.check));
                }
                if self.edge_layers.iter().any(|e| e.name == l.name) {
                    return Err(format!(
                        "Rule '{}' ({}) reads the whole layout and names the edge layer \
                         '{}', which only exists in the tiled cache",
                        rule.id, rule.check, l.name
                    ));
                }
            }
        }
        for (name, why) in &eager {
            let v = by_name[name.as_str()];
            if !matches!(
                v.op.as_str(),
                "union" | "intersection" | "difference" | "inside_ring" | "close"
            ) {
                return Err(format!(
                    "virtual layer '{name}' must be materialised for {why}, but `{}` only \
                     exists in the tiled cache",
                    v.op
                ));
            }
            for src in &v.layers {
                if by_name.contains_key(src.as_str())
                    || self.edge_layers.iter().any(|e| &e.name == src)
                {
                    return Err(format!(
                        "virtual layer '{name}' must be materialised for {why}, but it is \
                         built from the virtual layer '{src}', which is not"
                    ));
                }
            }
        }
        Ok(eager.into_keys().collect())
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
        let mut xy: Vec<gds21::GdsPoint> = m
            .outer
            .iter()
            .map(|p| gds21::GdsPoint::new(p.x, p.y))
            .collect();
        if let Some(first) = xy.first().cloned() {
            xy.push(first); // close the ring (GDS convention)
        }
        GdsBoundary {
            layer,
            datatype: dt,
            xy,
            ..Default::default()
        }
    }

    /// Materialise the virtual layers named in `eager` into the layout, from the
    /// boundaries already there.  Every other virtual layer is built per tile in the
    /// merge cache instead; see [`Self::eager_layers`] for which ones are named here.
    /// Uses only the boundaries already in the layout, so an eager layer cannot be
    /// built from another virtual layer.
    pub fn compute_virtual_layers(
        &self,
        layout: &mut FlatLayout,
        dbu_to_um: f64,
        eager: &HashSet<String>,
    ) {
        let mut to_insert: Vec<(i16, i16, GdsBoundary)> = vec![];

        for vl_def in &self.virtual_layers {
            if !eager.contains(&vl_def.name) {
                continue;
            }
            let Some(vl_layer) = self.layer_map.get(&vl_def.name) else {
                continue;
            };
            let vl_gds = vl_layer.gds_layer as i16;
            let vl_dt = vl_layer.gds_datatype as i16;

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
                        let src_dt = src.gds_datatype as i16;
                        if !seen_layers.insert((src_gds, src_dt)) {
                            continue;
                        }

                        for b in layout.get(src_gds, src_dt) {
                            let key: Vec<(i32, i32)> = b.xy.iter().map(|p| (p.x, p.y)).collect();
                            if seen_shapes.insert(key) {
                                to_insert.push((
                                    vl_gds,
                                    vl_dt,
                                    GdsBoundary {
                                        layer: vl_gds,
                                        datatype: vl_dt,
                                        xy: b.xy.clone(),
                                        ..Default::default()
                                    },
                                ));
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
                            to_insert.push((
                                vl_gds,
                                vl_dt,
                                Self::merged_outer_boundary(&m, vl_gds, vl_dt, Some(&vl_def.name)),
                            ));
                        }
                    }
                }
                "difference" | "not" => {
                    // Geometric NOT: the first layer minus all the rest (e.g.
                    // `ContNoSealring = Cont NOT EdgeSeal` removes the seal ring).
                    let Some((base_name, clip_names)) = vl_def.layers.split_first() else {
                        eprintln!(
                            "Virtual layer '{}': difference needs at least one layer",
                            vl_def.name
                        );
                        continue;
                    };
                    let Some(base) = self.layer_map.get(base_name) else {
                        eprintln!(
                            "Virtual layer '{}': source layer '{}' not found",
                            vl_def.name, base_name
                        );
                        continue;
                    };
                    let base_b = layout.get(base.gds_layer as i16, base.gds_datatype as i16);

                    let mut clips: Vec<&[GdsBoundary]> = Vec::with_capacity(clip_names.len());
                    let mut ok = true;
                    for name in clip_names {
                        let Some(c) = self.layer_map.get(name) else {
                            eprintln!(
                                "Virtual layer '{}': source layer '{}' not found",
                                vl_def.name, name
                            );
                            ok = false;
                            break;
                        };
                        clips.push(layout.get(c.gds_layer as i16, c.gds_datatype as i16));
                    }
                    if ok {
                        for m in crate::merge::difference_layers(base_b, &clips) {
                            to_insert.push((
                                vl_gds,
                                vl_dt,
                                Self::merged_outer_boundary(&m, vl_gds, vl_dt, Some(&vl_def.name)),
                            ));
                        }
                    }
                }
                "inside_ring" => {
                    // `inside_ring(target, ring)` = the part of `target` (layers[0]) that
                    // lies within the area enclosed by `ring` (layers[1]).  The ring's
                    // holes are filled, so a seal *frame* becomes "seal + interior" —
                    // there is no drawn layer for that region, so we derive it here.
                    // Both sources are real layers, so no virtual layer references
                    // another (which `compute_virtual_layers` does not support).
                    if vl_def.layers.len() < 2 {
                        eprintln!(
                            "Virtual layer '{}': inside_ring needs 2 layers (target, ring)",
                            vl_def.name
                        );
                        continue;
                    }
                    let (Some(target), Some(ring)) = (
                        self.layer_map.get(&vl_def.layers[0]),
                        self.layer_map.get(&vl_def.layers[1]),
                    ) else {
                        eprintln!(
                            "Virtual layer '{}': a source layer was not found",
                            vl_def.name
                        );
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
                        to_insert.push((
                            vl_gds,
                            vl_dt,
                            Self::merged_outer_boundary(&m, vl_gds, vl_dt, Some(&vl_def.name)),
                        ));
                    }
                }
                "close" => {
                    // Morphological closing of one layer: merge regions whose gap is below
                    // `radius * 2` µm (the "same-net merge" of NW.b / NBL.b).  Convex regions
                    // that stay separate keep their outer edges, so a downstream `min_space`
                    // sees the true gap between distinct (merged) regions.
                    let (Some(src_name), Some(radius_um)) = (vl_def.layers.first(), vl_def.radius)
                    else {
                        eprintln!(
                            "Virtual layer '{}': close needs one layer and a radius",
                            vl_def.name
                        );
                        continue;
                    };
                    let Some(src) = self.layer_map.get(src_name) else {
                        eprintln!(
                            "Virtual layer '{}': source layer '{}' not found",
                            vl_def.name, src_name
                        );
                        continue;
                    };
                    let merged = crate::merge::merge_boundaries(
                        layout.get(src.gds_layer as i16, src.gds_datatype as i16),
                    );
                    let radius_dbu = radius_um / dbu_to_um;
                    for m in crate::merge::closing(&merged, radius_dbu) {
                        to_insert.push((
                            vl_gds,
                            vl_dt,
                            Self::merged_outer_boundary(&m, vl_gds, vl_dt, Some(&vl_def.name)),
                        ));
                    }
                }
                other => {
                    eprintln!(
                        "Virtual layer '{}': unsupported op '{}' (supported: union, intersection, difference, inside_ring, close)",
                        vl_def.name, other
                    );
                }
            }
        }

        for (layer, dt, b) in to_insert {
            layout.insert(layer, dt, b);
        }
    }

    /// Edge layers resolved to GDS numbers, ready for the merge cache.  A source that
    /// does not resolve is a hard skip, same as for virtual layers.
    pub fn tiled_edge_layers(&self) -> Vec<TiledEdgeSpec> {
        let key = |name: &str| {
            self.layer_map
                .get(name)
                .map(|l| (l.gds_layer as i16, l.gds_datatype as i16))
        };
        let mut out = Vec::new();
        for el in &self.edge_layers {
            let Some(ekey) = key(&el.name) else { continue };
            let mut sources = Vec::with_capacity(el.layers.len());
            let mut ok = true;
            for s in &el.layers {
                match key(s) {
                    Some(k) => sources.push(k),
                    None => {
                        eprintln!("Edge layer '{}': source layer '{}' not found", el.name, s);
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                out.push(TiledEdgeSpec {
                    name: el.name.clone(),
                    key: ekey,
                    op: el.op.clone(),
                    sources,
                    min: el.min,
                    max: el.max,
                    fraction: el.fraction,
                });
            }
        }
        out
    }

    /// The virtual layers built per tile in the merge cache - every one not named in
    /// `eager` - resolved to GDS numbers: `(synthetic key, op, source keys)`.
    /// Sources/keys that don't resolve are skipped.
    pub fn tiled_virtual_layers(&self, eager: &HashSet<String>) -> Vec<TiledVirtualSpec> {
        let key = |name: &str| {
            self.layer_map
                .get(name)
                .map(|l| (l.gds_layer as i16, l.gds_datatype as i16))
        };
        let mut out = Vec::new();
        for vl in &self.virtual_layers {
            if eager.contains(&vl.name) {
                continue;
            }
            let Some(vkey) = key(&vl.name) else { continue };
            let mut sources = Vec::with_capacity(vl.layers.len());
            let mut ok = true;
            for s in &vl.layers {
                match key(s) {
                    Some(k) => sources.push(k),
                    None => {
                        eprintln!(
                            "Lazy virtual layer '{}': source layer '{}' not found",
                            vl.name, s
                        );
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
                    slack: vl.slack,
                    text: vl.text.clone(),
                    min: vl.min,
                    max: vl.max,
                });
            }
        }
        out
    }

    /// Expand a suite into the concatenated rules of the decks it imports, keeping
    /// only the whitelisted rule ids where an include specifies them.
    pub fn load_suite(
        &self,
        suite_name: &str,
    ) -> Result<Vec<RuleDefinition>, Box<dyn std::error::Error>> {
        let suite_ref = self
            .suites
            .iter()
            .find(|s| s.name == suite_name)
            .ok_or_else(|| format!("Suite '{suite_name}' not found in PDK '{}'", self.name))?;

        let content = self.source.read(&suite_ref.path)?;
        let raw: SuiteRaw = serde_norway::from_str(&content)?;

        let mut rules = Vec::new();
        for inc in &raw.include {
            rules.extend(self.load_deck_filtered(
                &inc.deck,
                inc.rules.as_deref(),
                inc.exclude.as_deref(),
            )?);
        }
        Ok(rules)
    }

    pub fn load_deck(
        &self,
        deck_name: &str,
    ) -> Result<Vec<RuleDefinition>, Box<dyn std::error::Error>> {
        self.load_deck_filtered(deck_name, None, None)
    }

    /// Load a deck's rules, optionally restricted to a whitelist of rule ids (`only`)
    /// and/or with a blacklist of ids removed (`exclude`).  The whitelist is applied
    /// first, then the blacklist.  Every id in either list must match at least one
    /// rule in the deck — an unknown id errors loudly so a suite typo can't silently
    /// drop or fail to drop a check.  An id may match several rules (e.g. `M1.b` is
    /// both `min_space` and `min_notch`); all matching entries are kept or dropped
    /// together.
    fn load_deck_filtered(
        &self,
        deck_name: &str,
        only: Option<&[String]>,
        exclude: Option<&[String]>,
    ) -> Result<Vec<RuleDefinition>, Box<dyn std::error::Error>> {
        let deck_ref = self
            .decks
            .iter()
            .find(|d| d.name == deck_name)
            .ok_or_else(|| format!("Deck '{deck_name}' not found in PDK '{}'", self.name))?;

        let content = self.source.read(&deck_ref.path)?;
        let mut raw: DeckRaw = serde_norway::from_str(&content)?;

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

        if let Some(ids) = exclude {
            let present: HashSet<&str> = raw.rules.iter().map(|r| r.id.as_str()).collect();
            if let Some(missing) = ids.iter().find(|w| !present.contains(w.as_str())) {
                return Err(format!(
                    "Suite excludes rule '{missing}' not found in deck '{deck_name}'"
                )
                .into());
            }
            raw.rules.retain(|r| !ids.iter().any(|w| w == &r.id));
        }

        let rules = raw
            .rules
            .into_iter()
            .map(|r| {
                if r.layers.is_empty() {
                    return Err(format!("Rule '{}' must define at least one layer", r.id));
                }
                let layers = r
                    .layers
                    .iter()
                    .map(|name| {
                        self.layer_map
                            .get(name)
                            .ok_or_else(|| {
                                format!("Rule '{}' references unknown layer '{}'", r.id, name)
                            })
                            .cloned()
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                // `ignore` is best-effort: each entry is a layer name or a raw
                // `layer/datatype` pair (for GDS layers the PDK doesn't name).
                let ignore = r
                    .ignore
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

                let mut params = r.params;
                for (key, name) in &r.layer_params {
                    let l = self.layer_map.get(name).ok_or_else(|| {
                        format!(
                            "Rule '{}' layer_param '{key}' references unknown layer '{name}'",
                            r.id
                        )
                    })?;
                    params.insert(key.clone(), Param::Num(l.gds_layer as f64));
                    params.insert(format!("{key}_dt"), Param::Num(l.gds_datatype as f64));
                }

                Ok(RuleDefinition {
                    id: r.id,
                    check: r.check,
                    layers,
                    value: r.value,
                    params,
                    ignore,
                    text: r.text,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(rules)
    }
}

#[cfg(test)]
mod path_tests {
    use super::dirs_from;
    use std::path::PathBuf;

    /// The command line's directories come first, then the variable's, each once.
    #[test]
    fn search_dirs_read_the_option_then_the_variable() {
        let extra = [PathBuf::from("/a"), PathBuf::from("/b")];
        let env = std::env::join_paths(["/b", "", "/c"]).unwrap();
        let dirs = dirs_from(&extra, &env);
        assert_eq!(
            dirs,
            [
                PathBuf::from("/a"),
                PathBuf::from("/b"),
                PathBuf::from("/c")
            ]
        );
        assert!(dirs_from(&[], std::ffi::OsStr::new("")).is_empty());
    }
}

#[cfg(test)]
mod waiver_tests {
    use super::glob_matches;

    #[test]
    fn glob_star_matches_any_run() {
        assert!(glob_matches("gf180mcu_fd_io__*", "gf180mcu_fd_io__in_c"));
        assert!(glob_matches("Bondpad_*", "Bondpad_5LM"));
        assert!(glob_matches("*_fill_*", "COMP_fill_cell"));
        assert!(glob_matches("exact", "exact"));
        assert!(!glob_matches("exact", "exactly"));
        assert!(!glob_matches("Bondpad_*", "xBondpad_5LM"));
        assert!(glob_matches("*", ""));
    }
}
