// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::violation::{Violation, ViolationGeometry};
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::collections::BTreeMap;
use std::io::BufWriter;

fn write_text<W: std::io::Write>(
    writer: &mut Writer<W>,
    tag: &str,
    text: &str,
) -> quick_xml::Result<()> {
    writer.write_event(Event::Start(BytesStart::new(tag)))?;
    writer.write_event(Event::Text(BytesText::new(text)))?;
    writer.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

/// A node in the category tree.  KLayout resolves an item's `<category>` element as
/// a **dot-separated path**, so a rule id like `Cnt.c.Digi` must exist as the
/// three-level chain `Cnt → c → Digi` — a two-level tree with a literal `c.Digi`
/// child is "not a valid category path" to the reader.
#[derive(Default)]
struct CatNode<'a> {
    /// Description of the category itself (set when a violation's id ends here).
    description: Option<&'a str>,
    children: BTreeMap<&'a str, CatNode<'a>>,
}

fn write_categories<W: std::io::Write>(
    writer: &mut Writer<W>,
    nodes: &BTreeMap<&str, CatNode>,
) -> quick_xml::Result<()> {
    writer.write_event(Event::Start(BytesStart::new("categories")))?;
    for (name, node) in nodes {
        writer.write_event(Event::Start(BytesStart::new("category")))?;
        write_text(writer, "name", name)?;
        if let Some(desc) = node.description {
            write_text(writer, "description", desc)?;
        }
        if !node.children.is_empty() {
            write_categories(writer, &node.children)?;
        }
        writer.write_event(Event::End(BytesEnd::new("category")))?;
    }
    writer.write_event(Event::End(BytesEnd::new("categories")))?;
    Ok(())
}

pub fn write_lyrdb(
    path: &str,
    topcell: &str,
    violations: &[Violation],
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let mut writer = Writer::new_with_indent(BufWriter::new(file), b' ', 2);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
    writer.write_event(Event::Start(BytesStart::new("report-database")))?;

    write_text(&mut writer, "description", "gdscheck DRC results")?;
    write_text(&mut writer, "generator", "gdscheck")?;

    // Build the category tree: every dot in a rule id is a hierarchy level.
    let mut tree: BTreeMap<&str, CatNode> = BTreeMap::new();
    for v in violations {
        let mut parts = v.rule_id.split('.');
        let first = parts.next().unwrap_or(&v.rule_id);
        let mut node = tree.entry(first).or_default();
        for part in parts {
            node = node.children.entry(part).or_default();
        }
        node.description.get_or_insert(v.description.as_str());
    }

    write_categories(&mut writer, &tree)?;

    // Cells
    writer.write_event(Event::Start(BytesStart::new("cells")))?;
    writer.write_event(Event::Start(BytesStart::new("cell")))?;
    write_text(&mut writer, "name", topcell)?;
    writer.write_event(Event::End(BytesEnd::new("cell")))?;
    writer.write_event(Event::End(BytesEnd::new("cells")))?;

    // Items
    writer.write_event(Event::Start(BytesStart::new("items")))?;
    for v in violations {
        writer.write_event(Event::Start(BytesStart::new("item")))?;
        write_text(&mut writer, "category", &v.rule_id)?;
        write_text(&mut writer, "cell", topcell)?;
        write_text(&mut writer, "visited", "false")?;
        write_text(&mut writer, "multiplicity", "1")?;

        writer.write_event(Event::Start(BytesStart::new("values")))?;
        // KLayout's RDB has no `point` value type; a degenerate (zero-length)
        // edge is the accepted way to mark a single location.
        let geometry = match &v.geometry {
            ViolationGeometry::Point { x, y } => {
                Some(format!("edge:({:.4},{:.4};{:.4},{:.4})", x, y, x, y))
            }
            ViolationGeometry::Edge { x1, y1, x2, y2 } => {
                Some(format!("edge:({:.4},{:.4};{:.4},{:.4})", x1, y1, x2, y2))
            }
            ViolationGeometry::None => None,
        };
        if let Some(g) = geometry {
            write_text(&mut writer, "value", &g)?;
        }
        // The full message as a per-item text value, so KLayout's marker browser
        // shows the layers and measured-vs-limit details, not just the category.
        write_text(&mut writer, "value", &format!("text: '{}'", v.message))?;
        writer.write_event(Event::End(BytesEnd::new("values")))?;

        writer.write_event(Event::End(BytesEnd::new("item")))?;
    }
    writer.write_event(Event::End(BytesEnd::new("items")))?;

    writer.write_event(Event::End(BytesEnd::new("report-database")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rule ids with several dots (e.g. `Cnt.c.Digi`) must produce a category chain
    /// at full depth — KLayout resolves the item's category as a dot-separated path
    /// and rejects the report otherwise.
    #[test]
    fn multi_dot_rule_ids_nest_fully() {
        let v = vec![
            Violation::point("Cnt.c.Digi", "Minimum enclosure violation", "m".into(), 1.0, 2.0),
            Violation::point("Cnt.c", "Minimum enclosure violation", "m".into(), 1.0, 2.0),
            Violation::global("forbidden", "Forbidden layer", "m".into()),
        ];
        let path = std::env::temp_dir().join("gdscheck_report_test.lyrdb");
        write_lyrdb(path.to_str().unwrap(), "TOP", &v).unwrap();
        let s = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).ok();
        // Cnt → c → Digi as nested categories, not a literal "c.Digi" leaf.
        let digi = s.find("<name>Digi</name>").expect("Digi category present");
        let c = s.find("<name>c</name>").expect("c category present");
        assert!(c < digi, "c must open before Digi");
        assert!(!s.contains("<name>c.Digi</name>"), "no literal dotted leaf");
    }
}
