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
//! Nets are a union-find over `(layer, region)` nodes; [`Connectivity::net_at`] maps a
//! point on a layer to its net id.

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

/// A net partition over a prefix of the connect steps: net id per global node.
pub struct Partition {
    node_net: Vec<usize>,
    net_count: usize,
}

impl Partition {
    /// Net id of the region of `layer` containing point `(x, y)` (DBU), if any.
    pub fn net_at(&self, conn: &Connectivity, layer: LayerKey, x: f64, y: f64) -> Option<usize> {
        let node = region_node_at(&conn.layers, layer, x, y, conn.tile_dbu)?;
        Some(self.node_net[node])
    }

    /// Net id of a global node — O(1).  Pair with [`Connectivity::node_base`] /
    /// [`Connectivity::node_at`] to avoid a point lookup per level.
    pub fn net_of(&self, node: usize) -> usize {
        self.node_net[node]
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

        // A layer needs the point-lookup index only if it is *bridged into* (a connector
        // resolves a point into it) or queried by `node_at`.  Connector-only layers (Cont,
        // the vias) are never looked into — Ant.c/d/f read their area by region index — so
        // we skip cloning their (often very dense) polygons into a per-tile index.
        let needs_index: HashSet<LayerKey> = specs
            .iter()
            .flat_map(|s| s.layers.iter().copied())
            .collect();

        // Build labeled regions per layer and assign a contiguous block of node ids.
        let mut layers: HashMap<LayerKey, LayerData> = HashMap::new();
        let mut next_base = 0usize;
        // `GDSCHECK_CONN_TRACE=1` reports what extraction costs per layer.  The two
        // numbers that matter are `polys` against `regions` - how many polygon copies the
        // tiling holds for each region it yields, which is where a halo shows up - and
        // `n_nodes`, which sets what every prefix partition costs.
        let trace = std::env::var("GDSCHECK_CONN_TRACE").is_ok();
        for &key in &keys {
            let t0 = std::time::Instant::now();
            cache.ensure(layout, key.0, key.1);
            let t_merge = t0.elapsed().as_secs_f64();
            let tiles = cache.tiles(key.0, key.1);
            let (n_tiles, n_polys) = (tiles.len(), tiles.values().map(|v| v.len()).sum::<usize>());
            let t1 = std::time::Instant::now();
            let indexed = needs_index.contains(&key);
            let labeled = if indexed {
                stitch_labeled(tiles, tile_dbu)
            } else {
                LabeledRegions {
                    regions: stitch_regions(tiles, tile_dbu),
                    by_tile: HashMap::new(),
                }
            };
            if trace {
                eprintln!(
                    "conn {}/{} halo={} indexed={} tiles={} polys={} regions={} \
                     merge={:.1}s stitch={:.1}s",
                    key.0,
                    key.1,
                    cache.halo_dbu(key.0, key.1),
                    indexed,
                    n_tiles,
                    n_polys,
                    labeled.regions.len(),
                    t_merge,
                    t1.elapsed().as_secs_f64()
                );
            }
            let base = next_base;
            next_base += labeled.regions.len();
            layers.insert(key, LayerData { labeled, base });
        }
        if trace {
            eprintln!(
                "conn n_nodes={} steps={} prefix partitions cost {:.1} GB",
                next_base,
                specs.len(),
                (next_base * (specs.len() + 1) * 8) as f64 / 1e9
            );
        }

        let mut conn = Connectivity {
            tile_dbu,
            layers,
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
    fn compute_partitions(&self) -> HashMap<usize, Arc<Partition>> {
        let mut uf = UnionFind::new(self.n_nodes);
        let mut cache = HashMap::new();
        cache.insert(0, Arc::new(self.snapshot(&mut uf)));
        for (k, s) in self.specs.iter().enumerate() {
            if let Some(conn) = self.layers.get(&s.connector) {
                // The lookups are the work - a point-in-polygon per via per bridged
                // layer, millions of them - and they only read; the unions are cheap and
                // replayed in order, so the result is the same as the serial loop's.
                let pairs: Vec<(usize, usize)> = conn
                    .labeled
                    .regions
                    .par_iter()
                    .enumerate()
                    .flat_map_iter(|(r, region)| {
                        let conn_node = conn.base + r;
                        let (mx, my) = region.anchor;
                        s.layers
                            .iter()
                            .filter_map(move |&lk| {
                                region_node_at(&self.layers, lk, mx, my, self.tile_dbu)
                                    .map(|node| (conn_node, node))
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect();
                for (a, b) in pairs {
                    uf.union(a, b);
                }
            }
            cache.insert(k + 1, Arc::new(self.snapshot(&mut uf)));
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

    /// Compact the current union-find roots into a dense net id per node.
    fn snapshot(&self, uf: &mut UnionFind) -> Partition {
        // A root is a node id, so a vector indexed by root numbers the nets without a
        // hash per node - twenty million nodes, twenty-two times over.
        let mut root_net = vec![usize::MAX; self.n_nodes];
        let mut net_count = 0usize;
        let mut node_net = vec![0usize; self.n_nodes];
        for (node, slot) in node_net.iter_mut().enumerate() {
            let root = uf.find(node);
            if root_net[root] == usize::MAX {
                root_net[root] = net_count;
                net_count += 1;
            }
            *slot = root_net[root];
        }
        Partition {
            node_net,
            net_count,
        }
    }

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

    /// Regions (area + marker) of `layer`, as built for connectivity.
    pub fn regions_of(&self, layer: LayerKey) -> &[crate::merge::Region] {
        self.layers
            .get(&layer)
            .map(|d| d.labeled.regions.as_slice())
            .unwrap_or(&[])
    }

    /// Global node id of `layer`'s region 0, if the layer is in the connect graph; region
    /// `r` is then node `base + r`.  Lets a check map its regions to nets in O(1).
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
