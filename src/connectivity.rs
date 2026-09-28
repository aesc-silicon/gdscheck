// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Electrical connectivity (net extraction).
//!
//! Net-aware checks (e.g. the antenna ratio rules, §7.1) need to know which shapes are
//! electrically the same net.  This module extracts nets from geometry alone, lazily —
//! it is only built when a net-aware check asks for it, so geometry-only decks (a plain
//! `metal5` run) never pay for it.
//!
//! ## Model
//!
//! Connectivity is defined by a list of [`ConnectSpec`]s, each a *connector* layer (a via
//! or contact) and the conductor layers it bridges.  A connector sits inside every layer
//! it joins (DRC enclosure), so a single point of the connector lands inside one region of
//! each bridged layer; those regions are unioned into one net.  A layer's own connected
//! regions are already merged by [`stitch_labeled`], so lateral routing on one layer needs
//! no bridging — only the vertical via/contact stack does.
//!
//! Nets are a union-find over `(layer, region)` nodes of the *conductor* layers;
//! [`Connectivity::net_at`] maps a point on a layer to its net id.  A connector that is
//! never a conductor - every via and contact - is no node: it is the reason two
//! conductor regions are one net, and once they are joined it has nothing further to
//! say.  What it keeps is the node of one region it touched, so a rule that sums the
//! connector's own area per net (the antenna ratio reads via area) can still ask which
//! net that is.  A design has ten times as many vias as it has conductor regions, and
//! every prefix partition is one entry per node.

use crate::layout::FlatLayout;
use crate::merge::{
    LabeledRegions, MergedCache, TileMap, UnionFind, overlap_within, point_in_merged, poly_bbox,
    stitch_labeled, stitch_labeled_indexed, stitch_regions_small,
};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub type LayerKey = (i16, i16);

/// A connector no larger than this on either side (µm) is joined where its anchor lies;
/// a larger one to every region it overlaps.  A via or a contact lies in one region of
/// each layer it joins, a tap strip running under a rail does not.
const POINT_CONNECTOR_UM: f64 = 1.0;

/// A connector layer and the conductor layers it electrically joins where it overlaps
/// them (e.g. `Cont` joins `Activ`/`GatPoly` to `Metal1`; `Via1` joins `Metal1`/`Metal2`).
#[derive(Clone, Debug)]
pub struct ConnectSpec {
    pub connector: LayerKey,
    pub layers: Vec<LayerKey>,
}

struct LayerData {
    labeled: LabeledRegions,
    /// Global node id of this layer's region 0; region `r` is node `base + r`.
    base: usize,
    /// Each region's size, what is kept of `labeled.regions` once the nets are built.
    sizes: Vec<RegionSize>,
}

/// A layer that only ever bridges: its regions, and for each the node of a conductor
/// region it joined - `u32::MAX` where it touched none.
struct ConnectorData {
    regions: Vec<crate::merge::Region>,
    attach: Vec<u32>,
    sizes: Vec<RegionSize>,
}

/// What a rule reads of a region once the nets are built: its area and perimeter (DBU).
/// Where it lies was needed to join it and is not kept - a design with forty million
/// contacts held 1.9 GB of markers for no reader.
#[derive(Clone, Copy, Debug)]
pub struct RegionSize {
    pub area_dbu: f64,
    pub perimeter_dbu: f64,
}

impl From<&crate::merge::Region> for RegionSize {
    fn from(r: &crate::merge::Region) -> Self {
        RegionSize {
            area_dbu: r.area_dbu,
            perimeter_dbu: r.perimeter_dbu,
        }
    }
}

/// A net partition over a prefix of the connect steps: net id per global node.
///
/// A net id is a `u32`: a partition is one entry per node, there is one partition per
/// connect step, and a design of sixteen million nodes over twenty steps is a gigabyte
/// and a half of them either way - so the width of the entry is the size of the run.
/// Four billion nets is not a layout anyone checks.
pub struct Partition {
    node_net: Vec<u32>,
    net_count: usize,
}

impl Partition {
    /// Net id of the region of `layer` containing point `(x, y)` (DBU), if any.
    pub fn net_at(&self, conn: &Connectivity, layer: LayerKey, x: f64, y: f64) -> Option<usize> {
        let node = region_node_at(&conn.layers, layer, x, y, conn.tile_dbu)?;
        Some(self.node_net[node] as usize)
    }

    /// Net id of a global node — O(1).  Pair with [`Connectivity::node_base`] /
    /// [`Connectivity::node_at`] to avoid a point lookup per level.
    pub fn net_of(&self, node: usize) -> usize {
        self.node_net[node] as usize
    }

    pub fn net_count(&self) -> usize {
        self.net_count
    }
}

/// Extracted connectivity: labeled regions per layer plus the ordered connect steps, from
/// which a [`Partition`] can be taken over any prefix (lazily — regions are built once).
pub struct Connectivity {
    tile_dbu: i32,
    layers: HashMap<LayerKey, LayerData>,
    connectors: HashMap<LayerKey, ConnectorData>,
    specs: Vec<ConnectSpec>,
    n_nodes: usize,
    /// Every prefix's partition, keyed by prefix length — computed once in [`build`], so
    /// a prefix shared across rules (e.g. the Metal levels used by both Ant.b and Ant.e)
    /// costs nothing extra.  Plain (not interior-mutable) so `Connectivity` stays `Sync`
    /// and a gate can resolve nets from inside a rayon-parallel check.
    partitions: HashMap<usize, Arc<Partition>>,
    /// Partition over *all* steps (the full net), for net_at / net_count.
    full: Arc<Partition>,
    /// Per connect step, the pairs of nodes a large connector joins by overlap, beside
    /// what its anchor joins (see [`overlap_bridges`]).
    bridges: Vec<Vec<(usize, usize)>>,
}

impl Connectivity {
    /// Build connectivity for `specs`.  Ensures and reads each referenced layer's merged
    /// tiles from `cache`, so it shares the one tiled merge with the geometric checks.
    pub fn build(
        cache: &mut MergedCache,
        layout: &FlatLayout,
        specs: &[ConnectSpec],
        dbu_to_um: f64,
    ) -> Self {
        let tile_dbu = cache.tile_dbu();

        // Every layer that participates: each connector and each conductor it bridges.
        let mut keys: Vec<LayerKey> = Vec::new();
        for s in specs {
            for k in std::iter::once(s.connector).chain(s.layers.iter().copied()) {
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
        }

        // A conductor - a layer a connector is resolved *into* - is indexed for the point
        // lookups and carries the nodes.  A layer that only ever bridges (Cont, the vias)
        // is neither: nobody looks into it, and it is no node, only the reason two are
        // one.  A rule reading its area per net (Ant.c/d/f) goes by region index.
        let conductors: HashSet<LayerKey> = specs
            .iter()
            .flat_map(|s| s.layers.iter().copied())
            .collect();

        // Build labeled regions per conductor and assign a contiguous block of node ids.
        let mut layers: HashMap<LayerKey, LayerData> = HashMap::new();
        let mut connectors: HashMap<LayerKey, ConnectorData> = HashMap::new();
        let mut next_base = 0usize;
        let mut n_connector_regions = 0usize;
        // `GDSCHECK_CONN_TRACE=1` reports what extraction costs per layer.  The two
        // numbers that matter are `polys` against `regions` - how many polygon copies the
        // tiling holds for each region it yields, which is where a halo shows up - and
        // `n_nodes`, which sets what every prefix partition costs.
        let trace = std::env::var("GDSCHECK_CONN_TRACE").is_ok();
        // Merge every layer first, one after another, since the cache is written; then
        // stitch them all at once, since the tiles are only read.  A layer's stitch is a
        // walk of its tiles on one core, and fifteen layers walked one after another
        // were the phase's length.
        // A drawn layer is merged and stitched before the next is merged, the way one
        // layer at a time did it: a stitch holds its pieces beside the tiles until it
        // is done, and the contacts' stitch over the whole chip on top of every other
        // layer already merged raised the run's high-water mark by a gigabyte the
        // allocator never gave back.  The derived layers, small and cheap to merge, are
        // merged in turn and stitched all at once.
        let mut merge_secs: HashMap<LayerKey, f64> = HashMap::new();
        let stitch_one = |cache: &MergedCache, &key: &LayerKey| {
            let tiles_arc = cache.tiles(key.0, key.1);
            let tiles = &*tiles_arc;
            let t1 = std::time::Instant::now();
            let labeled = if conductors.contains(&key) {
                stitch_labeled(tiles, tile_dbu)
            } else {
                LabeledRegions {
                    regions: stitch_regions_small(tiles, tile_dbu),
                    by_tile: HashMap::new(),
                }
            };
            (key, labeled, t1.elapsed().as_secs_f64())
        };
        let (drawn, derived): (Vec<LayerKey>, Vec<LayerKey>) =
            keys.iter().partition(|k| cache.is_drawn(**k));
        let point_dbu = (POINT_CONNECTOR_UM / dbu_to_um).round() as i32;
        let connector_keys: HashSet<LayerKey> = specs.iter().map(|s| s.connector).collect();
        let mut large: HashMap<LayerKey, Vec<LargeCopy>> = HashMap::new();
        // Per layer, for the trace: its halo, tiles and polygon copies.
        let mut shape: HashMap<LayerKey, (i32, usize, usize)> = HashMap::new();
        // What a layer leaves behind once stitched: its size for the trace, and the
        // copies a large connector joins by overlap.
        let mut keep = |cache: &MergedCache, key: LayerKey| {
            let tiles_arc = cache.tiles(key.0, key.1);
            let tiles = &*tiles_arc;
            shape.insert(
                key,
                (
                    cache.halo_dbu(key.0, key.1),
                    tiles.len(),
                    tiles.values().map(|v| v.len()).sum::<usize>(),
                ),
            );
            if connector_keys.contains(&key) {
                large.insert(key, large_copies(tiles, tile_dbu, point_dbu));
            }
        };
        // The derived layers first, built from their sources and stitched all at once;
        // then everything the cache holds goes, and each drawn layer is merged, stitched
        // and freed in turn.  Held all together until the last was stitched, the tiles of
        // every layer of the graph were the peak of the run: 25 GB over the flattened
        // layout on FMD_QNC_greyhound_ihp, where the nets they leave are 7.
        for &key in &derived {
            let t0 = std::time::Instant::now();
            cache.ensure(layout, key.0, key.1);
            merge_secs.insert(key, t0.elapsed().as_secs_f64());
        }
        let cache_ref: &MergedCache = cache;
        let derived_stitched: Vec<(LayerKey, LabeledRegions, f64)> = derived
            .par_iter()
            .map(|k| stitch_one(cache_ref, k))
            .collect();
        for &key in &derived {
            keep(cache, key);
        }
        for (key, _) in cache.resident_layers() {
            cache.evict(key.0, key.1);
        }
        cache.settle_frees();
        crate::memory::trim();

        let mut stitched: Vec<(LayerKey, LabeledRegions, f64)> = Vec::new();
        for &key in &drawn {
            let t0 = std::time::Instant::now();
            cache.ensure(layout, key.0, key.1);
            merge_secs.insert(key, t0.elapsed().as_secs_f64());
            stitched.push(stitch_one(cache, &key));
            keep(cache, key);
            cache.evict(key.0, key.1);
            cache.settle_frees();
            crate::memory::trim();
        }
        stitched.extend(derived_stitched);
        for (key, labeled, t_stitch) in stitched {
            let indexed = conductors.contains(&key);
            if trace {
                let (halo, tiles, polys) = shape[&key];
                eprintln!(
                    "conn {}/{} halo={} indexed={} tiles={} polys={} regions={} \
                     merge={:.1}s stitch={:.1}s",
                    key.0,
                    key.1,
                    halo,
                    indexed,
                    tiles,
                    polys,
                    labeled.regions.len(),
                    merge_secs[&key],
                    t_stitch
                );
            }
            if indexed {
                let base = next_base;
                next_base += labeled.regions.len();
                layers.insert(
                    key,
                    LayerData {
                        labeled,
                        base,
                        sizes: Vec::new(),
                    },
                );
            } else {
                let n = labeled.regions.len();
                n_connector_regions += n;
                connectors.insert(
                    key,
                    ConnectorData {
                        regions: labeled.regions,
                        attach: vec![u32::MAX; n],
                        sizes: Vec::new(),
                    },
                );
            }
        }
        if trace {
            eprintln!(
                "conn n_nodes={} connector regions={} steps={} prefix partitions cost {:.1} GB",
                next_base,
                n_connector_regions,
                specs.len(),
                (next_base * (specs.len() + 1) * std::mem::size_of::<u32>()) as f64 / 1e9
            );
        }
        let bridges: Vec<Vec<(usize, usize)>> = specs
            .iter()
            .map(|s| {
                let copies = large.get(&s.connector).map(Vec::as_slice).unwrap_or(&[]);
                overlap_bridges(copies, s, &layers, tile_dbu)
            })
            .collect();
        let mut conn = Connectivity {
            tile_dbu,
            layers,
            connectors,
            specs: specs.to_vec(),
            n_nodes: next_base,
            partitions: HashMap::new(),
            full: Arc::new(Partition {
                node_net: Vec::new(),
                net_count: 0,
            }),
            bridges,
        };
        let t2 = std::time::Instant::now();
        conn.partitions = conn.compute_partitions();
        // The regions were looked up at their anchors to join them; from here on a rule
        // reads each one's size only.
        for d in conn.layers.values_mut() {
            d.sizes = d.labeled.regions.iter().map(RegionSize::from).collect();
            d.labeled.regions = Vec::new();
        }
        for c in conn.connectors.values_mut() {
            c.sizes = c.regions.iter().map(RegionSize::from).collect();
            c.regions = Vec::new();
        }
        if trace {
            eprintln!(
                "conn partitions built in {:.1}s",
                t2.elapsed().as_secs_f64()
            );
        }
        conn.full = Arc::clone(&conn.partitions[&specs.len()]);
        conn
    }

    /// Net partition using only the first `up_to` connect steps (`up_to == specs.len()` is
    /// the full net).  All prefixes are precomputed once in a single incremental union-find
    /// pass (the expensive connector point-lookups happen only once total), then memoised.
    pub fn partition(&self, up_to: usize) -> Arc<Partition> {
        let up_to = up_to.min(self.specs.len());
        Arc::clone(self.partitions.get(&up_to).expect("prefix precomputed"))
    }

    /// Compute the partition at every prefix `0..=specs.len()` incrementally: one
    /// union-find, applying one connect step at a time and snapshotting after each.
    /// A connector's regions learn here which node they attached to.
    fn compute_partitions(&mut self) -> HashMap<usize, Arc<Partition>> {
        let Connectivity {
            layers,
            connectors,
            specs,
            tile_dbu,
            n_nodes,
            bridges,
            ..
        } = self;
        let (layers, tile_dbu, n_nodes) = (&*layers, *tile_dbu, *n_nodes);
        let mut uf = UnionFind::new(n_nodes);
        let mut cache = HashMap::new();
        cache.insert(0, Arc::new(snapshot(&mut uf, n_nodes)));
        // Whether two nets became one: a step joining nothing new leaves the partition
        // as it was, and shares it instead of holding a copy - on FMD_QNC_greyhound_ihp
        // half of seventeen steps, 22 million nodes each.
        let join = |uf: &mut UnionFind, a: usize, b: usize, changed: &mut bool| {
            if uf.find(a) != uf.find(b) {
                uf.union(a, b);
                *changed = true;
            }
        };
        for (k, s) in specs.iter().enumerate() {
            let mut changed = false;
            // The lookups are the work - a point-in-polygon per via per bridged layer,
            // millions of them - and they only read; the unions are cheap and replayed
            // in order, so the result is the same as the serial loop's.
            let nodes_under = |anchor: (f64, f64)| -> Vec<usize> {
                s.layers
                    .iter()
                    .filter_map(|&lk| region_node_at(layers, lk, anchor.0, anchor.1, tile_dbu))
                    .collect()
            };
            if let Some(conn) = layers.get(&s.connector) {
                // A connector that is a conductor elsewhere is a node, and joins what it
                // touches to itself.
                let pairs: Vec<(usize, usize)> = conn
                    .labeled
                    .regions
                    .par_iter()
                    .enumerate()
                    .flat_map_iter(|(r, region)| {
                        let conn_node = conn.base + r;
                        nodes_under(region.anchor)
                            .into_iter()
                            .map(move |node| (conn_node, node))
                            .collect::<Vec<_>>()
                    })
                    .collect();
                for (a, b) in pairs {
                    join(&mut uf, a, b, &mut changed);
                }
            } else if let Some(cd) = connectors.get_mut(&s.connector) {
                // A connector-only layer joins the nodes it touches to each other.  The
                // first node it ever touches is the one it remembers, and every node it
                // touches after that - in this step or a later one, since a connect
                // graph names one conductor per step - is joined to it: the connector is
                // the hub of its stack without being a node of it.
                let found: Vec<Vec<usize>> = cd
                    .regions
                    .par_iter()
                    .map(|region| nodes_under(region.anchor))
                    .collect();
                for (r, nodes) in found.into_iter().enumerate() {
                    for n in nodes {
                        if cd.attach[r] == u32::MAX {
                            cd.attach[r] = n as u32;
                        } else {
                            join(&mut uf, cd.attach[r] as usize, n, &mut changed);
                        }
                    }
                }
            }
            for &(a, b) in &bridges[k] {
                join(&mut uf, a, b, &mut changed);
            }
            let part = if changed {
                Arc::new(snapshot(&mut uf, n_nodes))
            } else {
                Arc::clone(&cache[&k])
            };
            cache.insert(k + 1, part);
            if std::env::var("GDSCHECK_CONN_TRACE").is_ok() {
                let rss = std::fs::read_to_string("/proc/self/statm")
                    .ok()
                    .and_then(|s| s.split_whitespace().nth(1).map(|v| v.to_string()))
                    .unwrap_or_default();
                eprintln!(
                    "conn step {k} done, nets={} rss={:.1} GB",
                    cache[&(k + 1)].net_count,
                    rss.parse::<f64>().unwrap_or(0.0) * 4096.0 / 1e9
                );
            }
        }
        cache
    }
}

/// Compact the current union-find roots into a dense net id per node.
fn snapshot(uf: &mut UnionFind, n_nodes: usize) -> Partition {
    // A root is a node id, so a vector indexed by root numbers the nets without a hash
    // per node - millions of nodes, twenty-two times over.
    let mut root_net = vec![u32::MAX; n_nodes];
    let mut net_count = 0u32;
    let mut node_net = vec![0u32; n_nodes];
    for (node, slot) in node_net.iter_mut().enumerate() {
        let root = uf.find(node);
        if root_net[root] == u32::MAX {
            root_net[root] = net_count;
            net_count += 1;
        }
        *slot = root_net[root];
    }
    Partition {
        node_net,
        net_count: net_count as usize,
    }
}

impl Connectivity {
    /// The first connect step index at which `layer` becomes connected (its `*_ratio` net),
    /// i.e. the prefix length to pass to [`partition`].  `None` if it never connects.
    ///
    /// A layer can join the graph as either end of a step, and a via joins as the
    /// *connector*: `Via1` bridges Metal1 to Metal2 and appears in no step's `layers`.
    /// Matching only `layers` therefore left every via unresolvable, and an antenna rule
    /// measuring via area silently contributed nothing at all - the failure mode this
    /// engine works hardest to avoid.  A connector is connected from the step that
    /// introduces it, which is the step it bridges its first pair at.
    pub fn connect_prefix(&self, layer: LayerKey) -> Option<usize> {
        self.specs
            .iter()
            .position(|s| s.layers.contains(&layer) || s.connector == layer)
            .map(|i| i + 1)
    }

    /// Region sizes (area, perimeter) of `layer`, as built for connectivity - a conductor's or
    /// a connector's.
    pub fn regions_of(&self, layer: LayerKey) -> &[RegionSize] {
        if let Some(d) = self.layers.get(&layer) {
            return &d.sizes;
        }
        self.connectors
            .get(&layer)
            .map(|c| c.sizes.as_slice())
            .unwrap_or(&[])
    }

    /// Whether `layer` is in the connect graph at all, as a conductor or a connector.
    pub fn in_graph(&self, layer: LayerKey) -> bool {
        self.layers.contains_key(&layer) || self.connectors.contains_key(&layer)
    }

    /// The global node region `idx` of `layer` belongs with: its own node for a
    /// conductor, the node of a conductor region it joined for a connector - `None` for
    /// a connector that touched nothing.  Lets a check map its regions to nets in O(1).
    pub fn region_node(&self, layer: LayerKey, idx: usize) -> Option<usize> {
        if let Some(d) = self.layers.get(&layer) {
            return Some(d.base + idx);
        }
        let a = *self.connectors.get(&layer)?.attach.get(idx)?;
        (a != u32::MAX).then_some(a as usize)
    }

    /// Global node id of `layer`'s region 0, if the layer is a conductor of the connect
    /// graph; region `r` is then node `base + r`.
    pub fn node_base(&self, layer: LayerKey) -> Option<usize> {
        self.layers.get(&layer).map(|d| d.base)
    }

    /// Global node of the region of `layer` containing `(x, y)` (a point lookup).
    pub fn node_at(&self, layer: LayerKey, x: f64, y: f64) -> Option<usize> {
        region_node_at(&self.layers, layer, x, y, self.tile_dbu)
    }

    /// Net id of the region of `layer` containing point `(x, y)` in the full net.
    pub fn net_at(&self, layer: LayerKey, x: f64, y: f64) -> Option<usize> {
        self.full.net_at(self, layer, x, y)
    }

    /// Total number of distinct nets in the full net.
    pub fn net_count(&self) -> usize {
        self.full.net_count()
    }
}

/// What a connector larger than a via joins besides its anchor: the nodes of every
/// region of the layers it bridges - and of its own, when it is a conductor too - that
/// it overlaps, joined to each other.  A connector is looked up at one point, which is
/// right for a via inside one region of each layer and wrong for a tap strip: the fill
/// cells' N+ under a row's VDD rail on fortalesa_chip, one region 2.7 mm long across
/// the pieces of Activ the gates cut, had its anchor in a piece no contact reached, and
/// the row's well never met VDD (NW.b1 against the SRAM macro's well).  Each tile reads
/// the overlaps within its core, where both layers' copies are exact; a connector
/// crossing a tile line is one region across its tiles, so what it overlaps in each of
/// them is joined.
fn overlap_bridges(
    copies: &[LargeCopy],
    spec: &ConnectSpec,
    layers: &HashMap<LayerKey, LayerData>,
    tile_dbu: i32,
) -> Vec<(usize, usize)> {
    if copies.is_empty() {
        return Vec::new();
    }
    let t = tile_dbu as i64;
    let targets: Vec<&LayerData> = spec
        .layers
        .iter()
        .chain(std::iter::once(&spec.connector))
        .filter_map(|k| layers.get(k))
        .collect();
    // (group, node) for every overlap.
    let mut hits: Vec<(u8, usize, usize)> = copies
        .par_iter()
        .flat_map_iter(|c| {
            let (x0, y0) = (c.tile.0 as i64 * t, c.tile.1 as i64 * t);
            let (x1, y1) = (x0 + t, y0 + t);
            let (bx0, by0, bx1, by1) = poly_bbox(&c.poly);
            let mut out: Vec<(u8, usize, usize)> = Vec::new();
            for d in &targets {
                let Some(pieces) = d.labeled.by_tile.get(&c.tile) else {
                    continue;
                };
                for (q, r) in pieces {
                    let (qx0, qy0, qx1, qy1) = poly_bbox(q);
                    if qx1 <= bx0 || qx0 >= bx1 || qy1 <= by0 || qy0 >= by1 {
                        continue;
                    }
                    if overlap_within(&c.poly, q, x0, y0, x1, y1) {
                        out.push((c.group.0, c.group.1, d.base + r));
                    }
                }
            }
            out
        })
        .collect();
    // Filed in a fixed order, so the union-find - and the net numbers - do not depend
    // on the order the tiles came back in.
    hits.sort_unstable();
    hits.dedup();
    hits.windows(2)
        .filter(|w| (w[0].0, w[0].1) == (w[1].0, w[1].1))
        .map(|w| (w[0].2, w[1].2))
        .collect()
}

/// A connector's copy larger than a via with some of it in its tile's core, and the
/// region it belongs to: a stitched region where the copy reaches a tile line, the
/// copy itself where it lies within its core.  Kept past the connector's tiles, which
/// are freed as soon as it is stitched; [`overlap_bridges`] reads them once every
/// conductor is.
struct LargeCopy {
    tile: (i32, i32),
    poly: crate::merge::MergedPoly,
    group: (u8, usize),
}

fn large_copies(tiles: &TileMap, tile_dbu: i32, point_dbu: i32) -> Vec<LargeCopy> {
    let t = tile_dbu as i64;
    let core = |(tx, ty): (i32, i32)| {
        let (x0, y0) = (tx as i64 * t, ty as i64 * t);
        (x0, y0, x0 + t, y0 + t)
    };
    let mut large: Vec<((i32, i32), usize)> = tiles
        .par_iter()
        .flat_map_iter(|(&tile, polys)| {
            let (x0, y0, x1, y1) = core(tile);
            polys
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    let (bx0, by0, bx1, by1) = poly_bbox(p);
                    (bx1 - bx0 > point_dbu || by1 - by0 > point_dbu)
                        && (bx1 as i64) > x0
                        && (bx0 as i64) < x1
                        && (by1 as i64) > y0
                        && (by0 as i64) < y1
                })
                .map(|(i, _)| (tile, i))
                .collect::<Vec<_>>()
        })
        .collect();
    if large.is_empty() {
        return Vec::new();
    }
    large.sort_unstable();
    // Which region a copy reaching a tile line belongs to, stitched only when one does.
    let crosses = large.iter().any(|&(tile, i)| {
        let (x0, y0, x1, y1) = core(tile);
        let (bx0, by0, bx1, by1) = poly_bbox(&tiles[&tile][i]);
        (bx0 as i64) <= x0 || (by0 as i64) <= y0 || (bx1 as i64) >= x1 || (by1 as i64) >= y1
    });
    let region_of: HashMap<((i32, i32), usize), usize> = if crosses {
        stitch_labeled_indexed(tiles, tile_dbu)
            .by_tile
            .into_iter()
            .flat_map(|(tile, v)| v.into_iter().map(move |(i, r)| ((tile, i), r)))
            .collect()
    } else {
        HashMap::new()
    };
    large
        .into_iter()
        .enumerate()
        .map(|(n, (tile, i))| LargeCopy {
            tile,
            poly: tiles[&tile][i].clone(),
            group: match region_of.get(&(tile, i)) {
                Some(&r) => (0, r),
                None => (1, n),
            },
        })
        .collect()
}

/// Global node id of the region of `layer` containing `(x, y)`, via the tile index.
fn region_node_at(
    layers: &HashMap<LayerKey, LayerData>,
    layer: LayerKey,
    x: f64,
    y: f64,
    tile_dbu: i32,
) -> Option<usize> {
    let data = layers.get(&layer)?;
    let t = tile_dbu as f64;
    let tile = ((x / t).floor() as i32, (y / t).floor() as i32);
    let polys = data.labeled.by_tile.get(&tile)?;
    for (poly, region) in polys {
        if point_in_merged(x, y, poly) {
            return Some(data.base + region);
        }
    }
    None
}
