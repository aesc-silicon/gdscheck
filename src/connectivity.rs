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
    point_in_merged, stitch_labeled, stitch_regions, LabeledRegions, MergedCache, UnionFind,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

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
    /// Memoised partitions keyed by prefix length, so a prefix shared across rules (e.g.
    /// the Metal levels used by both Ant.b and Ant.e) is computed only once.
    partitions: RefCell<HashMap<usize, Rc<Partition>>>,
    /// Partition over *all* steps (the full net), for net_at / net_count.
    full: Rc<Partition>,
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
        let needs_index: HashSet<LayerKey> =
            specs.iter().flat_map(|s| s.layers.iter().copied()).collect();

        // Build labeled regions per layer and assign a contiguous block of node ids.
        let mut layers: HashMap<LayerKey, LayerData> = HashMap::new();
        let mut next_base = 0usize;
        for &key in &keys {
            cache.ensure(layout, key.0, key.1);
            let tiles = cache.tiles(key.0, key.1);
            let labeled = if needs_index.contains(&key) {
                stitch_labeled(tiles, tile_dbu)
            } else {
                LabeledRegions { regions: stitch_regions(tiles, tile_dbu), by_tile: HashMap::new() }
            };
            let base = next_base;
            next_base += labeled.regions.len();
            layers.insert(key, LayerData { labeled, base });
        }

        let mut conn = Connectivity {
            tile_dbu,
            layers,
            specs: specs.to_vec(),
            n_nodes: next_base,
            partitions: RefCell::new(HashMap::new()),
            full: Rc::new(Partition { node_net: Vec::new(), net_count: 0 }),
        };
        conn.full = conn.partition(specs.len());
        conn
    }

    /// Net partition using only the first `up_to` connect steps (`up_to == specs.len()` is
    /// the full net).  All prefixes are precomputed once in a single incremental union-find
    /// pass (the expensive connector point-lookups happen only once total), then memoised.
    pub fn partition(&self, up_to: usize) -> Rc<Partition> {
        let up_to = up_to.min(self.specs.len());
        self.ensure_partitions();
        Rc::clone(self.partitions.borrow().get(&up_to).expect("prefix precomputed"))
    }

    /// Precompute the partition at every prefix `0..=specs.len()` incrementally: one
    /// union-find, applying one connect step at a time and snapshotting after each.
    fn ensure_partitions(&self) {
        if !self.partitions.borrow().is_empty() {
            return;
        }
        let mut uf = UnionFind::new(self.n_nodes);
        let mut cache = self.partitions.borrow_mut();
        cache.insert(0, Rc::new(self.snapshot(&mut uf)));
        for (k, s) in self.specs.iter().enumerate() {
            if let Some(conn) = self.layers.get(&s.connector) {
                for (r, region) in conn.labeled.regions.iter().enumerate() {
                    let conn_node = conn.base + r;
                    let (mx, my) = region.marker;
                    for &lk in &s.layers {
                        if let Some(node) = region_node_at(&self.layers, lk, mx, my, self.tile_dbu) {
                            uf.union(conn_node, node);
                        }
                    }
                }
            }
            cache.insert(k + 1, Rc::new(self.snapshot(&mut uf)));
        }
    }

    /// Compact the current union-find roots into a dense net id per node.
    fn snapshot(&self, uf: &mut UnionFind) -> Partition {
        let mut root_net: HashMap<usize, usize> = HashMap::new();
        let mut node_net = vec![0usize; self.n_nodes];
        for (node, slot) in node_net.iter_mut().enumerate() {
            let root = uf.find(node);
            let net = match root_net.get(&root) {
                Some(&n) => n,
                None => {
                    let n = root_net.len();
                    root_net.insert(root, n);
                    n
                }
            };
            *slot = net;
        }
        Partition { node_net, net_count: root_net.len() }
    }

    /// The first connect step index at which `layer` becomes connected (its `*_ratio` net),
    /// i.e. the prefix length to pass to [`partition`].  `None` if it never connects.
    pub fn connect_prefix(&self, layer: LayerKey) -> Option<usize> {
        self.specs.iter().position(|s| s.layers.contains(&layer)).map(|i| i + 1)
    }

    /// Regions (area + marker) of `layer`, as built for connectivity.
    pub fn regions_of(&self, layer: LayerKey) -> &[crate::merge::Region] {
        self.layers.get(&layer).map(|d| d.labeled.regions.as_slice()).unwrap_or(&[])
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
