// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Derived layers as sentences.
//!
//! A `pdk.yml` names a derived layer and says how it is made:
//!
//! ```yaml
//! virtual_layers:
//!   poly_otp:     poly2_drawn and otp_mk
//!   opl3a_poly:   (tgate or poly_field_otp_all) and otp_mk
//!   nat4_gate:    poly_nat_lv grow 0.5 inside ngate
//!   pl7_nom:      (nom_gate edges inside_part comp) with_angle min 25 max 65
//!   mdp3c_edges:  mdp3c_ncomp edges centers 99%
//!   vdd_pins:     Metal1.pin with_text "VDD*" TEXT
//! ```
//!
//! A sentence is read from left to right: an operand, then operations, each applied to
//! everything before it.  Parentheses group and nothing else does - there is no table of
//! precedences to remember.  An operation is a word, then its values (a number, a
//! percentage or a quoted string, and `min`/`max` bounds), then, for the operations that
//! take one, a second operand.  A bare number where a bound is expected means exactly
//! that: `interacting 2` is exactly two neighbours, `interacting min 2` at least two.
//!
//! The words are the ops of [`crate::merge::VirtualOp`] and [`crate::merge::EdgeOp`],
//! under their KLayout names.  Which of the two a word means follows from what stands
//! left of it: `and` on two regions is an area intersection, on two edge layers the
//! collinear overlap; `edges` turns a region into its boundary, and from there on the
//! sentence is about edges.  A word applied to the wrong kind is an error at load time,
//! with the column it happened at, rather than an empty layer at run time.
//!
//! A name alone is an alias - `top_metal: metal5` - and reads like one.
//!
//! Every operation in a sentence becomes one derived layer of the merge cache, exactly
//! as it did when each was its own `- name:` entry: the parser lowers a sentence to the
//! same [`VirtualLayerDef`]s and [`EdgeLayerDef`]s, naming the inner ones after their
//! owner (`opl3a_poly#1`), and a chain of one associative word (`a and b and c`) lowers
//! to the one n-ary op it always was.

use crate::pdk::{EdgeLayerDef, VirtualLayerDef};
use std::fmt;

/// Which kind of geometry a sub-sentence produces.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Poly,
    Edge,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Poly => "a region",
            Kind::Edge => "an edge layer",
        })
    }
}

/// A derived layer as the loader consumes it.
#[derive(Debug, Clone)]
pub enum Def {
    Poly(VirtualLayerDef),
    Edge(EdgeLayerDef),
}

impl Def {
    pub fn name(&self) -> &str {
        match self {
            Def::Poly(v) => &v.name,
            Def::Edge(e) => &e.name,
        }
    }
    pub fn kind(&self) -> Kind {
        match self {
            Def::Poly(_) => Kind::Poly,
            Def::Edge(_) => Kind::Edge,
        }
    }
    pub fn sources(&self) -> &[String] {
        match self {
            Def::Poly(v) => &v.layers,
            Def::Edge(e) => &e.layers,
        }
    }
}

/// The reach a `within` adds past its radius: one drawing grid step, enough to turn an
/// exact touch into a hairline overlap and nothing more.  See [`VirtualLayerDef::slack`].
pub const WITHIN_SLACK_UM: f64 = 0.005;

// ---------------------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------------------

#[derive(Clone, PartialEq, Debug)]
enum Tok {
    Word(String),
    Num(f64),
    Percent(f64),
    Str(String),
    Open,
    Close,
    End,
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn next(&mut self) -> Result<(usize, Tok), String> {
        let bytes = self.src.as_bytes();
        while self.pos < bytes.len() && bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        let at = self.pos;
        if self.pos >= bytes.len() {
            return Ok((at, Tok::End));
        }
        let c = bytes[self.pos];
        match c {
            b'(' => {
                self.pos += 1;
                Ok((at, Tok::Open))
            }
            b')' => {
                self.pos += 1;
                Ok((at, Tok::Close))
            }
            b'"' => {
                let start = self.pos + 1;
                let end = self.src[start..]
                    .find('"')
                    .map(|i| start + i)
                    .ok_or_else(|| format!("column {}: unterminated string", at + 1))?;
                self.pos = end + 1;
                Ok((at, Tok::Str(self.src[start..end].to_string())))
            }
            b'0'..=b'9' | b'.'
                if c != b'.' || bytes.get(self.pos + 1).is_some_and(u8::is_ascii_digit) =>
            {
                let start = self.pos;
                while self.pos < bytes.len()
                    && (bytes[self.pos].is_ascii_digit() || bytes[self.pos] == b'.')
                {
                    self.pos += 1;
                }
                let text = &self.src[start..self.pos];
                let v: f64 = text
                    .parse()
                    .map_err(|_| format!("column {}: `{text}` is not a number", at + 1))?;
                if self.pos < bytes.len() && bytes[self.pos] == b'%' {
                    self.pos += 1;
                    return Ok((at, Tok::Percent(v)));
                }
                Ok((at, Tok::Num(v)))
            }
            _ if c.is_ascii_alphabetic() || c == b'_' => {
                let start = self.pos;
                while self.pos < bytes.len()
                    && (bytes[self.pos].is_ascii_alphanumeric()
                        || matches!(bytes[self.pos], b'_' | b'.'))
                {
                    self.pos += 1;
                }
                Ok((at, Tok::Word(self.src[start..self.pos].to_string())))
            }
            _ => Err(format!(
                "column {}: unexpected `{}`",
                at + 1,
                &self.src[at..self.src.len().min(at + 1)]
            )),
        }
    }
}

// ---------------------------------------------------------------------------------------
// Syntax tree
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Num(f64),
    Percent(f64),
    Str(String),
}

#[derive(Debug, Clone)]
enum Node {
    Name(String),
    Op {
        word: String,
        col: usize,
        values: Vec<Value>,
        min: Option<f64>,
        max: Option<f64>,
        left: Box<Node>,
        right: Option<Box<Node>>,
    },
}

// ---------------------------------------------------------------------------------------
// The op table
// ---------------------------------------------------------------------------------------

/// What a word may take on its right.
#[derive(Clone, Copy, PartialEq)]
enum Right {
    /// No second operand.
    Nothing,
    Poly,
    Edge,
    /// Either: a region selected by what it touches may be asked about edges as well.
    Any,
}

/// What a word means for a given kind of left operand.
struct OpSpec {
    /// The op name the merge cache knows.
    canonical: &'static str,
    /// Whether the word takes a second operand, and of which kind.
    right: Right,
    /// What the word produces.
    out: Kind,
    /// How the word reads its values.
    values: Values,
}

#[derive(Clone, Copy, PartialEq)]
enum Values {
    /// No values at all.
    None,
    /// One number: the radius.
    Radius,
    /// One number: the radius, plus a grid step of slack.
    Within,
    /// One number, stored as `min`.
    OneMin,
    /// One number, stored as `max`.
    OneMax,
    /// One quoted string.
    Text,
    /// Bounds: `min`/`max`, or one bare number meaning both; optional.
    CountsOptional,
    /// Bounds: `min`/`max`, or one bare number meaning both; at least one required.
    Bounds,
    /// A number (absolute length, `min`) and/or a percentage (`fraction`).
    Centers,
}

fn spec(word: &str, left: Kind) -> Option<OpSpec> {
    use Kind::*;
    use Values::*;
    let s = |canonical, right, out, values| {
        Some(OpSpec {
            canonical,
            right,
            out,
            values,
        })
    };
    let (none, poly, edge, any) = (Right::Nothing, Right::Poly, Right::Edge, Right::Any);
    match (left, word) {
        (Poly, "and" | "intersection") => s("intersection", poly, Poly, None),
        (Poly, "or" | "union") => s("union", poly, Poly, None),
        (Poly, "not" | "difference") => s("difference", poly, Poly, None),
        (Poly, "overlapping" | "not_outside") => s("overlapping", poly, Poly, CountsOptional),
        (Poly, "not_overlapping" | "outside") => s("not_overlapping", poly, Poly, CountsOptional),
        (Poly, "interacting") => s("interacting", any, Poly, CountsOptional),
        (Poly, "not_interacting") => s("not_interacting", any, Poly, CountsOptional),
        (Poly, "inside") => s("inside", poly, Poly, None),
        (Poly, "not_inside") => s("not_inside", poly, Poly, None),
        (Poly, "covering") => s("covering", poly, Poly, CountsOptional),
        (Poly, "not_covering") => s("not_covering", poly, Poly, CountsOptional),
        (Poly, "enclosure_above") => s("enclosure_above", poly, Poly, OneMin),
        (Poly, "enclosure_below") => s("enclosure_below", poly, Poly, OneMax),
        (Poly, "separation_below") => s("separation_below", poly, Poly, OneMax),
        (Poly, "inside_ring") => s("inside_ring", poly, Poly, None),
        (Poly, "square") => s("square", none, Poly, None),
        (Poly, "not_square") => s("not_square", none, Poly, None),
        (Poly, "rectangle") => s("rectangle", none, Poly, None),
        (Poly, "not_rectangle") => s("not_rectangle", none, Poly, None),
        (Poly, "not_circle") => s("not_circle", none, Poly, None),
        (Poly, "not_circle_or_octagon") => s("not_circle_or_octagon", none, Poly, None),
        (Poly, "holes") => s("holes", none, Poly, None),
        (Poly, "with_holes") => s("with_holes", none, Poly, None),
        (Poly, "extents") => s("extents", none, Poly, None),
        (Poly, "with_text") => s("with_text", poly, Poly, Text),
        (Poly, "with_area") => s("with_area", none, Poly, Bounds),
        (Poly, "with_bbox_min") => s("with_bbox_min", none, Poly, Bounds),
        (Poly, "with_bbox_max") => s("with_bbox_max", none, Poly, Bounds),
        (Poly, "close") => s("close", none, Poly, Radius),
        (Poly, "open") => s("open", none, Poly, Radius),
        (Poly, "grow") => s("grow", none, Poly, Radius),
        (Poly, "within") => s("grow", none, Poly, Within),
        (Poly, "shrink") => s("shrink", none, Poly, Radius),
        (Poly, "grow_x") => s("grow_x", none, Poly, Radius),
        (Poly, "grow_y") => s("grow_y", none, Poly, Radius),
        (Poly, "shrink_x") => s("shrink_x", none, Poly, Radius),
        (Poly, "shrink_y") => s("shrink_y", none, Poly, Radius),
        (Poly, "edges") => s("edges", none, Edge, None),
        (Poly, "width_below") => s("width_below", none, Edge, OneMax),
        (Edge, "and") => s("and", edge, Edge, None),
        (Edge, "not") => s("not", edge, Edge, None),
        (Edge, "or" | "join") => s("or", edge, Edge, None),
        (Edge, "interacting_edges") => s("interacting_edges", edge, Edge, None),
        (Edge, "not_interacting_edges") => s("not_interacting_edges", edge, Edge, None),
        (Edge, "inside_part") => s("inside_part", poly, Edge, None),
        (Edge, "outside_part") => s("outside_part", poly, Edge, None),
        (Edge, "interacting") => s("interacting", poly, Edge, None),
        (Edge, "not_interacting") => s("not_interacting", poly, Edge, None),
        (Edge, "centers") => s("centers", none, Edge, Centers),
        (Edge, "with_length") => s("with_length", none, Edge, Bounds),
        (Edge, "without_length") => s("without_length", none, Edge, Bounds),
        (Edge, "with_angle") => s("with_angle", none, Edge, Bounds),
        (Edge, "without_angle") => s("without_angle", none, Edge, Bounds),
        _ => Option::None,
    }
}

/// Whether `word` is an operation for either kind - the words a layer may not be named.
pub fn is_op_word(word: &str) -> bool {
    spec(word, Kind::Poly).is_some()
        || spec(word, Kind::Edge).is_some()
        || matches!(word, "min" | "max")
}

/// The n-ary ops: a chain of one of these lowers to a single op over every operand.
fn is_chainable(canonical: &str, kind: Kind) -> bool {
    matches!(
        (kind, canonical),
        (Kind::Poly, "intersection" | "union" | "difference") | (Kind::Edge, "or")
    )
}

// ---------------------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------------------

struct Parser<'a> {
    lex: Lexer<'a>,
    look: (usize, Tok),
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Result<Self, String> {
        let mut lex = Lexer { src, pos: 0 };
        let look = lex.next()?;
        Ok(Parser { lex, look })
    }

    fn advance(&mut self) -> Result<(usize, Tok), String> {
        let cur = std::mem::replace(&mut self.look, (0, Tok::End));
        self.look = self.lex.next()?;
        Ok(cur)
    }

    /// `sentence := operand { op }`
    fn sentence(&mut self) -> Result<Node, String> {
        let mut node = self.operand()?;
        loop {
            match &self.look.1 {
                Tok::Word(w) if !matches!(w.as_str(), "min" | "max") => {
                    let (col, tok) = self.advance()?;
                    let Tok::Word(word) = tok else { unreachable!() };
                    let mut values = Vec::new();
                    let (mut min, mut max) = (None, None);
                    loop {
                        match &self.look.1 {
                            Tok::Num(_) | Tok::Percent(_) | Tok::Str(_) => {
                                let (_, t) = self.advance()?;
                                values.push(match t {
                                    Tok::Num(v) => Value::Num(v),
                                    Tok::Percent(v) => Value::Percent(v),
                                    Tok::Str(s) => Value::Str(s),
                                    _ => unreachable!(),
                                });
                            }
                            Tok::Word(w) if w == "min" || w == "max" => {
                                let (c, t) = self.advance()?;
                                let Tok::Word(w) = t else { unreachable!() };
                                let (vc, v) = self.advance()?;
                                let Tok::Num(v) = v else {
                                    return Err(format!("column {}: `{w}` needs a number", vc + 1));
                                };
                                let slot = if w == "min" { &mut min } else { &mut max };
                                if slot.is_some() {
                                    return Err(format!("column {}: `{w}` given twice", c + 1));
                                }
                                *slot = Some(v);
                            }
                            _ => break,
                        }
                    }
                    // A second operand, if one follows: a name or a parenthesis.  The
                    // op table decides whether the word wanted one.
                    let right = match &self.look.1 {
                        Tok::Word(w) if !matches!(w.as_str(), "min" | "max") && !is_op_word(w) => {
                            Some(Box::new(self.operand()?))
                        }
                        Tok::Open => Some(Box::new(self.operand()?)),
                        _ => None,
                    };
                    node = Node::Op {
                        word,
                        col,
                        values,
                        min,
                        max,
                        left: Box::new(node),
                        right,
                    };
                }
                Tok::End | Tok::Close => return Ok(node),
                Tok::Word(w) => {
                    return Err(format!(
                        "column {}: `{w}` without an operation",
                        self.look.0 + 1
                    ));
                }
                other => {
                    return Err(format!(
                        "column {}: unexpected {:?}",
                        self.look.0 + 1,
                        other
                    ));
                }
            }
        }
    }

    /// `operand := name | "(" sentence ")"`
    fn operand(&mut self) -> Result<Node, String> {
        match self.advance()? {
            (_, Tok::Word(w)) if !is_op_word(&w) => Ok(Node::Name(w)),
            (c, Tok::Word(w)) => Err(format!(
                "column {}: `{w}` is an operation, not a layer",
                c + 1
            )),
            (_, Tok::Open) => {
                let inner = self.sentence()?;
                match self.advance()? {
                    (_, Tok::Close) => Ok(inner),
                    (c, _) => Err(format!("column {}: expected `)`", c + 1)),
                }
            }
            (c, Tok::End) => Err(format!("column {}: expected a layer", c + 1)),
            (c, t) => Err(format!("column {}: expected a layer, found {t:?}", c + 1)),
        }
    }
}

// ---------------------------------------------------------------------------------------
// Lowering
// ---------------------------------------------------------------------------------------

struct Lowerer<'a> {
    owner: &'a str,
    kind_of: &'a dyn Fn(&str) -> Option<Kind>,
    out: Vec<Def>,
    anon: usize,
}

impl Lowerer<'_> {
    /// Lower `node`; returns the name that holds its result and its kind.  `root` names
    /// the owner; inner ops get `owner#n`.
    fn lower(&mut self, node: Node, root: Option<&str>) -> Result<(String, Kind), String> {
        match node {
            Node::Name(n) => {
                let kind = (self.kind_of)(&n).ok_or_else(|| format!("unknown layer `{n}`"))?;
                let Some(alias) = root else {
                    return Ok((n, kind));
                };
                // A name alone is an alias - `top_metal: metal5` - which is the union of
                // one layer, so a rule can name the alias like any derived layer.
                let def = match kind {
                    Kind::Poly => Def::Poly(VirtualLayerDef {
                        name: alias.to_string(),
                        op: "union".to_string(),
                        layers: vec![n],
                        radius: None,
                        text: None,
                        min: None,
                        max: None,
                        slack: None,
                    }),
                    Kind::Edge => Def::Edge(EdgeLayerDef {
                        name: alias.to_string(),
                        op: "or".to_string(),
                        layers: vec![n],
                        min: None,
                        max: None,
                        fraction: None,
                    }),
                };
                self.out.push(def);
                Ok((alias.to_string(), kind))
            }
            Node::Op {
                word,
                col,
                values,
                min,
                max,
                left,
                right,
            } => {
                let (left_name, left_kind) = self.lower(*left, None)?;
                let spec = spec(&word, left_kind).ok_or_else(|| {
                    format!(
                        "column {}: `{word}` is not an operation on {left_kind}",
                        col + 1
                    )
                })?;
                let mut layers = vec![left_name];
                match (spec.right, right) {
                    (Right::Nothing, Some(_)) => {
                        return Err(format!(
                            "column {}: `{word}` takes no second operand",
                            col + 1
                        ));
                    }
                    (Right::Nothing, None) => {}
                    (_, None) => {
                        return Err(format!(
                            "column {}: `{word}` needs a second operand",
                            col + 1
                        ));
                    }
                    (want, Some(r)) => {
                        let (rn, rk) = self.lower(*r, None)?;
                        let ok = match want {
                            Right::Poly => rk == Kind::Poly,
                            Right::Edge => rk == Kind::Edge,
                            _ => true,
                        };
                        if !ok {
                            let want = if want == Right::Poly {
                                Kind::Poly
                            } else {
                                Kind::Edge
                            };
                            return Err(format!(
                                "column {}: `{word}` wants {want} on its right, `{rn}` is {rk}",
                                col + 1
                            ));
                        }
                        layers.push(rn);
                    }
                }
                // A chain of one associative word folds into the op it is.
                if is_chainable(spec.canonical, spec.out)
                    && layers.len() == 2
                    && let Some(prev) = self.out.last()
                    && prev.name() == layers[0]
                    && prev.name().starts_with(self.owner)
                    && prev.name().contains('#')
                {
                    let same = match prev {
                        Def::Poly(v) => v.op == spec.canonical,
                        Def::Edge(e) => e.op == spec.canonical,
                    };
                    if same {
                        let Some(prev) = self.out.pop() else {
                            unreachable!()
                        };
                        self.anon -= 1;
                        let mut srcs = prev.sources().to_vec();
                        srcs.push(layers.pop().unwrap());
                        layers = srcs;
                    }
                }
                let name = match root {
                    Some(r) => r.to_string(),
                    None => {
                        self.anon += 1;
                        format!("{}#{}", self.owner, self.anon)
                    }
                };
                let def = self.build(&name, &spec, &word, col, layers, values, min, max)?;
                self.out.push(def);
                Ok((name, spec.out))
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        &self,
        name: &str,
        spec: &OpSpec,
        word: &str,
        col: usize,
        layers: Vec<String>,
        values: Vec<Value>,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Result<Def, String> {
        let at = || format!("column {}: `{word}`", col + 1);
        let one_num = || -> Result<f64, String> {
            match values.as_slice() {
                [Value::Num(v)] => Ok(*v),
                _ => Err(format!("{} takes one number", at())),
            }
        };
        let no_bounds = || -> Result<(), String> {
            if min.is_some() || max.is_some() {
                Err(format!("{} takes no min/max", at()))
            } else {
                Ok(())
            }
        };
        let (mut radius, mut slack, mut text, mut lo, mut hi, mut fraction) =
            (None, None, None, None, None, None);
        match spec.values {
            Values::None => {
                no_bounds()?;
                if !values.is_empty() {
                    return Err(format!("{} takes no values", at()));
                }
            }
            Values::Radius => {
                no_bounds()?;
                radius = Some(one_num()?);
            }
            Values::Within => {
                no_bounds()?;
                radius = Some(one_num()?);
                slack = Some(WITHIN_SLACK_UM);
            }
            Values::OneMin => {
                no_bounds()?;
                lo = Some(one_num()?);
            }
            Values::OneMax => {
                no_bounds()?;
                hi = Some(one_num()?);
            }
            Values::Text => {
                no_bounds()?;
                match values.as_slice() {
                    [Value::Str(s)] => text = Some(s.clone()),
                    _ => return Err(format!("{} takes one quoted string", at())),
                }
            }
            Values::CountsOptional | Values::Bounds => {
                match values.as_slice() {
                    [] => {
                        lo = min;
                        hi = max;
                    }
                    [Value::Num(v)] => {
                        if min.is_some() || max.is_some() {
                            return Err(format!(
                                "{} takes a bare number or min/max, not both",
                                at()
                            ));
                        }
                        lo = Some(*v);
                        hi = Some(*v);
                    }
                    _ => return Err(format!("{} takes one number, or min/max", at())),
                }
                if spec.values == Values::Bounds && lo.is_none() && hi.is_none() {
                    return Err(format!("{} needs a number, or min/max", at()));
                }
            }
            Values::Centers => {
                no_bounds()?;
                for v in &values {
                    match v {
                        Value::Num(n) if lo.is_none() => lo = Some(*n),
                        Value::Percent(p) if fraction.is_none() => fraction = Some(p / 100.0),
                        _ => return Err(format!("{} takes a length and/or a percentage", at())),
                    }
                }
                if lo.is_none() && fraction.is_none() {
                    return Err(format!("{} needs a length or a percentage", at()));
                }
            }
        }
        Ok(match spec.out {
            Kind::Poly => Def::Poly(VirtualLayerDef {
                name: name.to_string(),
                op: spec.canonical.to_string(),
                layers,
                radius,
                text,
                min: lo,
                max: hi,
                slack,
            }),
            Kind::Edge => Def::Edge(EdgeLayerDef {
                name: name.to_string(),
                op: spec.canonical.to_string(),
                layers,
                min: lo,
                max: hi,
                fraction,
            }),
        })
    }
}

/// Parse `sentence` as the derivation of the layer `name` and lower it to the derived
/// layers it needs, the layer itself last.  `kind_of` answers what kind of layer a name
/// is, `None` for a name nobody declared.
pub fn lower(
    name: &str,
    sentence: &str,
    kind_of: &dyn Fn(&str) -> Option<Kind>,
) -> Result<Vec<Def>, String> {
    let mut p = Parser::new(sentence)?;
    let node = p.sentence()?;
    if p.look.1 != Tok::End {
        return Err(format!("column {}: unexpected `)`", p.look.0 + 1));
    }
    let mut l = Lowerer {
        owner: name,
        kind_of,
        out: Vec::new(),
        anon: 0,
    };
    l.lower(node, Some(name))?;
    Ok(l.out)
}

/// The kind a sentence produces, without lowering it - so a table of names and kinds can
/// be built before any sentence is lowered against it.  Names it does not know are taken
/// to be regions, which is right for every drawn layer and wrong only for a sentence
/// that names an edge layer declared *after* it, which [`lower`] then reports.
pub fn kind_of_sentence(
    sentence: &str,
    kind_of: &dyn Fn(&str) -> Option<Kind>,
) -> Result<Kind, String> {
    let mut p = Parser::new(sentence)?;
    let node = p.sentence()?;
    fn kind(node: &Node, kind_of: &dyn Fn(&str) -> Option<Kind>) -> Result<Kind, String> {
        match node {
            Node::Name(n) => Ok(kind_of(n).unwrap_or(Kind::Poly)),
            Node::Op {
                word, col, left, ..
            } => {
                let lk = kind(left, kind_of)?;
                spec(word, lk).map(|s| s.out).ok_or_else(|| {
                    format!("column {}: `{word}` is not an operation on {lk}", col + 1)
                })
            }
        }
    }
    kind(&node, kind_of)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(n: &str) -> Option<Kind> {
        match n {
            "e" | "f" => Some(Kind::Edge),
            "a" | "b" | "c" | "d" | "Metal1.pin" => Some(Kind::Poly),
            _ => None,
        }
    }

    fn poly(d: &Def) -> &VirtualLayerDef {
        match d {
            Def::Poly(v) => v,
            Def::Edge(_) => panic!("expected a region"),
        }
    }

    fn edge(d: &Def) -> &EdgeLayerDef {
        match d {
            Def::Edge(e) => e,
            Def::Poly(_) => panic!("expected an edge layer"),
        }
    }

    #[test]
    fn a_chain_of_one_word_is_one_op() {
        let out = lower("x", "a and b and c", &kinds).unwrap();
        assert_eq!(out.len(), 1);
        let v = poly(&out[0]);
        assert_eq!((v.name.as_str(), v.op.as_str()), ("x", "intersection"));
        assert_eq!(v.layers, vec!["a", "b", "c"]);
        let out = lower("x", "a not b not c", &kinds).unwrap();
        assert_eq!(poly(&out[0]).layers, vec!["a", "b", "c"]);
    }

    #[test]
    fn left_to_right_and_parentheses() {
        // (a or b) and c: the union is an inner layer, the owner is the intersection.
        let out = lower("x", "(a or b) and c", &kinds).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(
            (poly(&out[0]).name.as_str(), poly(&out[0]).op.as_str()),
            ("x#1", "union")
        );
        assert_eq!(poly(&out[1]).layers, vec!["x#1", "c"]);
        // a grow 0.5 inside b: the grow is inner, the selection is the owner.
        let out = lower("x", "a grow 0.5 inside b", &kinds).unwrap();
        assert_eq!(poly(&out[0]).radius, Some(0.5));
        assert_eq!(
            (poly(&out[1]).op.as_str(), poly(&out[1]).layers.as_slice()),
            ("inside", &["x#1".to_string(), "b".to_string()][..])
        );
        // Two different chains do not fold into each other.
        let out = lower("x", "a and b or c", &kinds).unwrap();
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn values_and_bounds() {
        let v = poly(&lower("x", "a interacting 2 b", &kinds).unwrap()[0]).clone();
        assert_eq!((v.min, v.max), (Some(2.0), Some(2.0)));
        let v = poly(&lower("x", "a interacting min 2 b", &kinds).unwrap()[0]).clone();
        assert_eq!((v.min, v.max), (Some(2.0), None));
        let v = poly(&lower("x", "a with_area min 0.1444", &kinds).unwrap()[0]).clone();
        assert_eq!((v.op.as_str(), v.min), ("with_area", Some(0.1444)));
        let v = poly(&lower("x", "a within 1.06", &kinds).unwrap()[0]).clone();
        assert_eq!(
            (v.op.as_str(), v.radius, v.slack),
            ("grow", Some(1.06), Some(WITHIN_SLACK_UM))
        );
        let v = poly(&lower("x", "a enclosure_above 0.6 b", &kinds).unwrap()[0]).clone();
        assert_eq!((v.min, v.layers.len()), (Some(0.6), 2));
        let v = poly(&lower("x", "Metal1.pin with_text \"VDD*\" b", &kinds).unwrap()[0]).clone();
        assert_eq!((v.text.as_deref(), v.layers.len()), (Some("VDD*"), 2));
        let e = edge(&lower("x", "e centers 99%", &kinds).unwrap()[0]).clone();
        assert_eq!((e.op.as_str(), e.fraction), ("centers", Some(0.99)));
        let e = edge(&lower("x", "e with_angle min 25 max 65", &kinds).unwrap()[0]).clone();
        assert_eq!((e.min, e.max), (Some(25.0), Some(65.0)));
    }

    #[test]
    fn kinds_follow_the_words() {
        let out = lower("x", "a edges and (b edges) inside_part c", &kinds).unwrap();
        assert_eq!(out.len(), 4);
        assert_eq!(edge(&out[3]).op, "inside_part");
        assert_eq!(edge(&out[2]).op, "and");
        let out = lower("x", "a width_below 4.0", &kinds).unwrap();
        assert_eq!(
            (edge(&out[0]).op.as_str(), edge(&out[0]).max),
            ("width_below", Some(4.0))
        );
        assert_eq!(kind_of_sentence("a edges", &kinds).unwrap(), Kind::Edge);
        assert_eq!(kind_of_sentence("a grow 1", &kinds).unwrap(), Kind::Poly);
    }

    #[test]
    fn mistakes_are_named_with_a_column() {
        let err = |s: &str| lower("x", s, &kinds).unwrap_err();
        assert!(
            err("a edges and b").contains("wants an edge layer"),
            "{}",
            err("a edges and b")
        );
        assert!(err("a grow b").contains("takes no second operand"));
        assert!(err("a grow").contains("takes one number"));
        assert!(err("a inside").contains("needs a second operand"));
        assert!(err("a holes b").contains("takes no second operand"));
        assert!(err("a with_text b c").contains("quoted string"));
        assert!(err("a interacting 2 min 3 b").contains("not both"));
        assert!(err("a and (b").contains("expected `)`"));
        assert!(err("q and b").contains("unknown layer `q`"));
        assert_eq!(poly(&lower("x", "a", &kinds).unwrap()[0]).op, "union");
        assert!(err("a inside_part b").starts_with("column 3"));
        assert!(err("e centers").contains("needs a length or a percentage"));
    }
}
