//! Office formats: OOXML and OpenDocument, read as zip + XML.
use crate::{
    db::{err, Res},
    ingest::Section,
};
use quick_xml::events::{BytesRef, Event};
use std::collections::HashMap;
use std::io::Read;

/// Guard against zip bombs.
const MAX_MEMBER: u64 = 128 * 1024 * 1024;

fn le16(b: &[u8], o: usize) -> Res<usize> {
    Ok(u16::from_le_bytes([*b.get(o).ok_or(TRUNC)?, *b.get(o + 1).ok_or(TRUNC)?]) as usize)
}
fn le32(b: &[u8], o: usize) -> Res<usize> {
    let mut v = [0u8; 4];
    v.copy_from_slice(b.get(o..o + 4).ok_or(TRUNC)?);
    Ok(u32::from_le_bytes(v) as usize)
}
const TRUNC: &str = "Truncated archive: the file is incomplete or corrupt.";

/// Central directory of a zip archive.
struct Zip<'a> {
    data: &'a [u8],
    entries: Vec<(String, usize, usize, usize)>,
}
impl<'a> Zip<'a> {
    fn open(data: &'a [u8]) -> Res<Self> {
        // End of central directory, searched backwards.
        let window = data.len().min(22 + 0xFFFF);
        let start = data.len() - window;
        let eocd = (start..=data.len().saturating_sub(22))
            .rev()
            .find(|&i| data[i..i + 4] == [0x50, 0x4b, 0x05, 0x06])
            .ok_or("Ce fichier n'est pas une archive zip valide.")?;
        let count = le16(data, eocd + 10)?;
        let mut off = le32(data, eocd + 16)?;
        if off == 0xFFFF_FFFF || count == 0xFFFF {
            return Err("Archive au format zip64, non prise en charge.".into());
        }
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            if data.get(off..off + 4).ok_or(TRUNC)? != [0x50, 0x4b, 0x01, 0x02] {
                break;
            }
            let method = le16(data, off + 10)?;
            let compressed = le32(data, off + 20)?;
            let name_len = le16(data, off + 28)?;
            let extra_len = le16(data, off + 30)?;
            let comment_len = le16(data, off + 32)?;
            let local = le32(data, off + 42)?;
            let name =
                String::from_utf8_lossy(data.get(off + 46..off + 46 + name_len).ok_or(TRUNC)?)
                    .into_owned();
            entries.push((name, method, local, compressed));
            off += 46 + name_len + extra_len + comment_len;
        }
        Ok(Zip { data, entries })
    }
    fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.0.as_str())
    }
    fn read(&self, name: &str) -> Res<Vec<u8>> {
        let (_, method, local, compressed) = self
            .entries
            .iter()
            .find(|e| e.0 == name)
            .ok_or_else(|| format!("Membre « {name} » absent de l'archive."))?;
        let d = self.data;
        if d.get(*local..*local + 4).ok_or(TRUNC)? != [0x50, 0x4b, 0x03, 0x04] {
            return Err(TRUNC.into());
        }
        // The local header repeats name and extra lengths, which may differ from the central directory.
        let begin = *local + 30 + le16(d, *local + 26)? + le16(d, *local + 28)?;
        let raw = d.get(begin..begin + *compressed).ok_or(TRUNC)?;
        let mut out = Vec::new();
        match method {
            0 => out.extend_from_slice(raw),
            8 => {
                flate2::read::DeflateDecoder::new(raw)
                    .take(MAX_MEMBER)
                    .read_to_end(&mut out)
                    .map_err(err)?;
            }
            m => return Err(format!("Compression zip {m} non prise en charge.")),
        }
        Ok(out)
    }
}

/// Text of an XML entity.
fn entity(r: &BytesRef) -> String {
    if let Ok(Some(c)) = r.resolve_char_ref() {
        return c.to_string();
    }
    quick_xml::escape::resolve_predefined_entity(r.as_ref())
        .unwrap_or_default()
        .to_string()
}

/// Shared XML walk.
fn walk(xml: &[u8], text_in: &[&str], breaks: &[&str], tabs: &[&str]) -> Res<String> {
    let mut r = quick_xml::Reader::from_reader(xml);
    r.config_mut().trim_text(false);
    let mut out = String::new();
    let mut depth = 0usize;
    let mut buf = Vec::new();
    let local = |n: &str| n.rsplit(':').next().unwrap_or_default().to_string();
    loop {
        match r.read_event_into(&mut buf).map_err(err)? {
            Event::Eof => break,
            Event::Start(e) => {
                let n = local(e.local_name().as_ref());
                if breaks.contains(&n.as_str()) && !out.is_empty() && !out.ends_with(['\n', '\t']) {
                    out.push('\n');
                }
                if tabs.contains(&n.as_str()) && !out.is_empty() && !out.ends_with(['\n', '\t']) {
                    out.push('\t');
                }
                if text_in.contains(&n.as_str()) {
                    depth += 1;
                }
            }
            Event::End(e) => {
                if text_in.contains(&local(e.local_name().as_ref()).as_str()) {
                    depth = depth.saturating_sub(1);
                }
            }
            Event::Empty(e) => {
                let n = local(e.local_name().as_ref());
                if breaks.contains(&n.as_str()) && !out.ends_with('\n') && !out.is_empty() {
                    out.push('\n');
                }
                if tabs.contains(&n.as_str()) {
                    out.push('\t');
                }
            }
            Event::Text(t) => {
                if text_in.is_empty() || depth > 0 {
                    out.push_str(&t.xml10_content());
                }
            }
            Event::GeneralRef(e) if (text_in.is_empty() || depth > 0) => {
                out.push_str(&entity(&e));
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

/// Members of a kind, sorted numerically: slide2 before slide10.
fn numbered(z: &Zip, prefix: &str, suffix: &str) -> Vec<String> {
    let mut v: Vec<String> = z
        .names()
        .filter(|n| n.starts_with(prefix) && n.ends_with(suffix))
        .map(String::from)
        .collect();
    let num = |s: &str| {
        s.chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(0)
    };
    v.sort_by_key(|n| num(n));
    v
}

fn docx(z: &Zip) -> Res<Vec<Section>> {
    // <w:tab/> and <w:br/> carry paragraph layout.
    let text = walk(
        &z.read("word/document.xml")?,
        &["t"],
        &["p", "br", "tr"],
        &["tab", "tc"],
    )?;
    Ok(crate::ingest::text_sections(&text))
}

fn pptx(z: &Zip) -> Res<Vec<Section>> {
    let mut out = vec![];
    for (i, name) in numbered(z, "ppt/slides/slide", ".xml")
        .into_iter()
        .enumerate()
    {
        let text = walk(&z.read(&name)?, &["t"], &["p", "br"], &[])?;
        if !text.trim().is_empty() {
            out.push(Section {
                text,
                locator: format!("Diapositive {}", i + 1),
            });
        }
    }
    Ok(out)
}

/// A workbook keeps strings in a shared table; cells only carry the index.
fn xlsx(z: &Zip) -> Res<Vec<Section>> {
    let shared: Vec<String> = match z.read("xl/sharedStrings.xml") {
        Ok(x) => {
            let mut r = quick_xml::Reader::from_reader(x.as_slice());
            let mut v = vec![];
            let (mut buf, mut cur, mut inside) = (Vec::new(), String::new(), false);
            loop {
                match r.read_event_into(&mut buf).map_err(err)? {
                    Event::Eof => break,
                    Event::Start(e) => match e.local_name().as_ref() {
                        "si" => cur.clear(),
                        "t" => inside = true,
                        _ => {}
                    },
                    Event::End(e) => match e.local_name().as_ref() {
                        "si" => v.push(std::mem::take(&mut cur)),
                        "t" => inside = false,
                        _ => {}
                    },
                    Event::Text(t) if inside => cur.push_str(&t.xml10_content()),
                    Event::GeneralRef(e) if inside => cur.push_str(&entity(&e)),
                    _ => {}
                }
                buf.clear();
            }
            v
        }
        Err(_) => vec![],
    };
    let names = sheet_names(z).unwrap_or_default();
    let mut out = vec![];
    for (i, member) in numbered(z, "xl/worksheets/sheet", ".xml")
        .into_iter()
        .enumerate()
    {
        let text = sheet(&z.read(&member)?, &shared)?;
        if !text.trim().is_empty() {
            let label = names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("Feuille {}", i + 1));
            out.push(Section {
                text,
                locator: label,
            });
        }
    }
    Ok(out)
}
/// Sheet names, in workbook order.
fn sheet_names(z: &Zip) -> Res<Vec<String>> {
    let x = z.read("xl/workbook.xml")?;
    let mut r = quick_xml::Reader::from_reader(x.as_slice());
    let mut v = vec![];
    let mut buf = Vec::new();
    loop {
        let e = match r.read_event_into(&mut buf).map_err(err)? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) => e,
            _ => {
                buf.clear();
                continue;
            }
        };
        if e.local_name().as_ref() == "sheet" {
            if let Some(a) = e
                .attributes()
                .flatten()
                .find(|a| a.key.local_name().as_ref() == "name")
            {
                v.push(
                    a.normalized_value(quick_xml::XmlVersion::Explicit1_0)
                        .map_err(err)?
                        .into_owned(),
                );
            }
        }
        buf.clear();
    }
    Ok(v)
}
fn sheet(xml: &[u8], shared: &[String]) -> Res<String> {
    let mut r = quick_xml::Reader::from_reader(xml);
    let (mut out, mut row, mut value, mut buf) = (
        String::new(),
        Vec::<String>::new(),
        String::new(),
        Vec::new(),
    );
    let (mut kind, mut in_value) = (String::new(), false);
    loop {
        match r.read_event_into(&mut buf).map_err(err)? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) => match e.local_name().as_ref() {
                "row" => row.clear(),
                "c" => {
                    kind = e
                        .attributes()
                        .flatten()
                        .find(|a| a.key.local_name().as_ref() == "t")
                        .map(|a| {
                            a.normalized_value(quick_xml::XmlVersion::Explicit1_0)
                                .unwrap_or_default()
                                .into_owned()
                        })
                        .unwrap_or_default();
                    value.clear();
                }
                "v" | "t" => in_value = true,
                _ => {}
            },
            Event::End(e) => match e.local_name().as_ref() {
                "row" => {
                    if row.iter().any(|c| !c.trim().is_empty()) {
                        out.push_str(&row.join("\t"));
                        out.push('\n');
                    }
                    row.clear();
                }
                "c" => {
                    let v = if kind == "s" {
                        value
                            .trim()
                            .parse::<usize>()
                            .ok()
                            .and_then(|i| shared.get(i).cloned())
                            .unwrap_or_default()
                    } else {
                        std::mem::take(&mut value)
                    };
                    row.push(v);
                }
                "v" | "t" => in_value = false,
                _ => {}
            },
            Event::Text(t) if in_value => value.push_str(&t.xml10_content()),
            Event::GeneralRef(e) if in_value => value.push_str(&entity(&e)),
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

/// OpenDocument keeps everything in content.xml.
fn opendocument(z: &Zip) -> Res<Vec<Section>> {
    let text = walk(
        &z.read("content.xml")?,
        &[],
        &["p", "h", "table-row", "list-item"],
        &["table-cell", "tab"],
    )?;
    Ok(crate::ingest::text_sections(&text))
}

pub fn is_office(kind: &str) -> bool {
    matches!(kind, "docx" | "pptx" | "xlsx" | "odt" | "odp" | "ods")
}
/// Sections of an office file, ready for the usual chunking.
pub fn sections(kind: &str, data: &[u8]) -> Res<Vec<Section>> {
    let z = Zip::open(data)?;
    let secs = match kind {
        "docx" => docx(&z)?,
        "pptx" => pptx(&z)?,
        "xlsx" => xlsx(&z)?,
        "odt" | "odp" | "ods" => opendocument(&z)?,
        _ => return Err(format!("Format bureautique inconnu : {kind}")),
    };
    if secs.iter().all(|s| s.text.trim().is_empty()) {
        return Err("No text found: empty document, or content only as images.".into());
    }
    Ok(secs)
}

/// Member table, to diagnose a refused archive.
#[allow(dead_code)]
pub fn listing(data: &[u8]) -> Res<HashMap<String, usize>> {
    let z = Zip::open(data)?;
    Ok(z.entries.iter().map(|e| (e.0.clone(), e.3)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xml_walk_handles_prefixes_and_breaks() {
        let x = br#"<w:document><w:p><w:t>Bonjour</w:t><w:tab/><w:t>monde</w:t></w:p><w:p><w:t>Suite</w:t></w:p></w:document>"#;
        assert_eq!(
            walk(x, &["t"], &["p", "br"], &["tab"]).unwrap(),
            "Bonjour\tmonde\nSuite"
        );
    }
    #[test]
    fn walk_ignores_text_outside_the_carrier_tag() {
        // Style properties must not land in the index.
        let x = br#"<w:p><w:rPr>Arial</w:rPr><w:t>Reel</w:t></w:p>"#;
        assert_eq!(walk(x, &["t"], &["p"], &[]).unwrap(), "Reel");
    }
    #[test]
    fn opendocument_takes_bare_text() {
        let x = br#"<office><text:p>Une ligne</text:p><text:p>Deux</text:p></office>"#;
        assert_eq!(walk(x, &[], &["p", "h"], &[]).unwrap(), "Une ligne\nDeux");
    }
    #[test]
    fn sheet_resolves_shared_strings() {
        let shared = vec!["Nom".to_string(), "Ville".to_string()];
        let x = br#"<sheetData><row><c t="s"><v>0</v></c><c t="s"><v>1</v></c></row><row><c><v>42</v></c><c t="inlineStr"><is><t>Paris</t></is></c></row></sheetData>"#;
        assert_eq!(sheet(x, &shared).unwrap(), "Nom\tVille\n42\tParis\n");
    }
    #[test]
    fn slides_sort_numerically_not_alphabetically() {
        let names = ["ppt/slides/slide10.xml", "ppt/slides/slide2.xml"];
        let num = |s: &str| {
            s.chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
                .parse::<u32>()
                .unwrap_or(0)
        };
        let mut v = names.to_vec();
        v.sort_by_key(|n| num(n));
        assert_eq!(v, ["ppt/slides/slide2.xml", "ppt/slides/slide10.xml"]);
    }
    #[test]
    fn rejects_non_zip() {
        assert!(Zip::open(b"pas une archive du tout").is_err());
    }
}

/// Checks against real files produced by Word, Excel or LibreOffice.
#[cfg(test)]
mod fixtures {
    use super::*;
    #[test]
    #[ignore = "demande des fichiers de test : voir la documentation du module"]
    fn office_fixtures_are_readable() {
        let dir =
            std::env::var("LANGOLIER_OFFICE_FIXTURES").expect("LANGOLIER_OFFICE_FIXTURES not set");
        let mut seen = 0;
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            let Some(kind) = p.extension().and_then(|x| x.to_str()) else {
                continue;
            };
            if !is_office(kind) {
                continue;
            }
            seen += 1;
            let secs = sections(kind, &std::fs::read(&p).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let total: usize = secs.iter().map(|s| s.text.len()).sum();
            println!(
                "{} -> {} section(s), {total} characters",
                p.file_name().unwrap().to_string_lossy(),
                secs.len()
            );
            for s in secs.iter().take(3) {
                println!(
                    "   [{}] {}",
                    s.locator,
                    s.text
                        .replace('\n', " ⏎ ")
                        .chars()
                        .take(110)
                        .collect::<String>()
                );
            }
            assert!(total > 0, "{} : texte vide", p.display());
        }
        assert!(seen > 0, "aucun fichier bureautique dans {dir}");
    }
}
