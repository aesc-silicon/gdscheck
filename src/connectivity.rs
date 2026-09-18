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
    LabeledRegions, MergedCache, UnionFind, point_in_merged, stitch_labeled, stitch_regions,
};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub type LayerKey = (i16, i16);

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
}

/// A layer that only ever bridges: its regions, and for each the node of a conductor
/// region it joined - `u32::MAX` where it touched none.
struct ConnectorData {
    regions: Vec<crate::merge::Region>,
    attach: Vec<u32>,
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
}

impl Connectivity {
    /// Build connectivity for `specs`.  Ensures and reads each referenced layer's merged
    /// tiles from `cache`, so it shares the one tiled merge with the geometric checks.
    pub fn build(cache: &mut MergedCache, layout: &FlatLayout, specs: &[ConnectSpec]) -> Self {
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
            let tiles = cache.tiles(key.0, key.1);
            let t1 = std::time::Instant::now();
            let labeled = if conductors.contains(&key) {
                stitch_labeled(tiles, tile_dbu)
            } else {
                LabeledRegions {
                    regions: stitch_regions(tiles, tile_dbu),
                    by_tile: HashMap::new(),
                }
            };
            (key, labeled, t1.elapsed().as_secs_f64())
        };
        let (drawn, derived): (Vec<LayerKey>, Vec<LayerKey>) =
            keys.iter().partition(|k| cache.is_drawn(**k));
        let mut stitched: Vec<(LayerKey, LabeledRegions, f64)> = Vec::new();
        for &key in &drawn {
            let t0 = std::time::Instant::now();
            cache.ensure(layout, key.0, key.1);
            merge_secs.insert(key, t0.elapsed().as_secs_f64());
            stitched.push(stitch_one(cache, &key));
        }
        for &key in &derived {
            let t0 = std::time::Instant::now();
            cache.ensure(layout, key.0, key.1);
            merge_secs.insert(key, t0.elapsed().as_secs_f64());
        }
        let cache_ref: &MergedCache = cache;
        stitched.extend(
            derived
                .par_iter()
                .map(|k| stitch_one(cache_ref, k))
                .collect::<Vec<_>>(),
        );
        for (key, labeled, t_stitch) in stitched {
            let indexed = conductors.contains(&key);
            if trace {
                let tiles = cache.tiles(key.0, key.1);
                eprintln!(
                    "conn {}/{} halo={} indexed={} tiles={} polys={} regions={} \
                     merge={:.1}s stitch={:.1}s",
                    key.0,
                    key.1,
                    cache.halo_dbu(key.0, key.1),
                    indexed,
                    tiles.len(),
                    tiles.values().map(|v| v.len()).sum::<usize>(),
                    labeled.regions.len(),
                    merge_secs[&key],
                    t_stitch
                );
            }
            if indexed {
                let base = next_base;
                next_base += labeled.regions.len();
                layers.insert(key, LayerData { labeled, base });
            } else {
                let n = labeled.regions.len();
                n_connector_regions += n;
                connectors.insert(
                    key,
                    ConnectorData {
                        regions: labeled.regions,
                        attach: vec![u32::MAX; n],
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
        };
        let t2 = std::time::Instant::now();
        conn.partitions = conn.compute_partitions();
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
            ..
        } = self;
        let (layers, tile_dbu, n_nodes) = (&*layers, *tile_dbu, *n_nodes);
        let mut uf = UnionFind::new(n_nodes);
        let mut cache = HashMap::new();
        cache.insert(0, Arc::new(snapshot(&mut uf, n_nodes)));
        for (k, s) in specs.iter().enumerate() {
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
                    uf.union(a, b);
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
                            uf.union(cd.attach[r] as usize, n);
                        }
                    }
                }
            }
            cache.insert(k + 1, Arc::new(snapshot(&mut uf, n_nodes)));
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

    /// Regions (area + marker) of `layer`, as built for connectivity - a conductor's or
    /// a connector's.
    pub fn regions_of(&self, layer: LayerKey) -> &[crate::merge::Region] {
        if let Some(d) = self.layers.get(&layer) {
            return &d.labeled.regions;
        }
        self.connectors
            .get(&layer)
            .map(|c| c.regions.as_slice())
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
