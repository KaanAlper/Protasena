//! The bill of materials as CSV (RFC 4180: comma separated, CRLF line ends, fields quoted when
//! they need it) and as an HTML page (one table, our own plain styling).
//!
//! Design data is hostile: HTML text is escaped, and a CSV field that a spreadsheet would read
//! as a formula (starting with `=`, `+`, `@`, a tab, a carriage return, or `-` not followed by a
//! digit) gets a leading `'`.

use crate::bom::Bom;

/// The column headers, in order.
pub const BOM_COLUMNS: [&str; 6] = ["Quantity", "References", "Value", "Device", "Package", "Description"];

fn cells(line: &crate::bom::BomLine) -> [String; 6] {
    [line.quantity.to_string(), line.references.join(", "), line.value.clone(), line.device.clone(), line.package.clone(), line.description.clone()]
}

fn csv_field(s: &str) -> String {
    let mut chars = s.chars();
    let risky = match chars.next() {
        Some('=' | '+' | '@' | '\t' | '\r') => true,
        Some('-') => !chars.next().is_some_and(|c| c.is_ascii_digit() || c == '.'),
        _ => false,
    };
    let s = if risky { format!("'{s}") } else { s.to_owned() };
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s }
}

/// The bill of materials as CSV, with a header row.
pub fn bom_to_csv(bom: &Bom) -> String {
    let mut out = BOM_COLUMNS.join(",");
    out.push_str("\r\n");
    for line in &bom.lines {
        out.push_str(&cells(line).iter().map(|c| csv_field(c)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// The bill of materials as a self-contained HTML page.
pub fn bom_to_html(bom: &Bom) -> String {
    let title = format!("Bill of Materials: {}", bom.design);
    let mut out = String::from("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    out.push_str(&format!("<title>{}</title>\n", escape(&title)));
    out.push_str(
        "<style>body{font-family:sans-serif;margin:2em;color:#222}table{border-collapse:collapse}\
         th,td{border:1px solid #bbb;padding:4px 10px;text-align:left;vertical-align:top}th{background:#eee}\
         td.n{text-align:right}</style>\n</head>\n<body>\n",
    );
    out.push_str(&format!("<h1>{}</h1>\n<p>{} parts, {} lines</p>\n<table>\n<tr>", escape(&title), bom.total(), bom.lines.len()));
    for h in BOM_COLUMNS {
        out.push_str(&format!("<th>{h}</th>"));
    }
    out.push_str("</tr>\n");
    for line in &bom.lines {
        out.push_str("<tr>");
        for (i, c) in cells(line).iter().enumerate() {
            let class = if i == 0 { " class=\"n\"" } else { "" };
            out.push_str(&format!("<td{class}>{}</td>", escape(c)));
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</table>\n</body>\n</html>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bom::BomLine;

    fn bom() -> Bom {
        let line = |q, refs: &[&str], value: &str| BomLine {
            quantity: q,
            references: refs.iter().map(|r| (*r).to_owned()).collect(),
            value: value.into(),
            device: "RES".into(),
            package: "RES40".into(),
            description: "Resistor".into(),
        };
        Bom { design: "d<1>".into(), lines: vec![line(2, &["R1", "R2"], "10k"), line(1, &["R3"], "=1+1"), line(1, &["R4"], "say \"hi\"")] }
    }

    #[test]
    fn csv_quotes_and_defuses_formulas() {
        let csv = bom_to_csv(&bom());
        let lines: Vec<&str> = csv.split("\r\n").collect();
        assert_eq!(lines[0], "Quantity,References,Value,Device,Package,Description");
        assert_eq!(lines[1], "2,\"R1, R2\",10k,RES,RES40,Resistor");
        assert_eq!(lines[2], "1,R3,'=1+1,RES,RES40,Resistor");
        assert_eq!(lines[3], "1,R4,\"say \"\"hi\"\"\",RES,RES40,Resistor");
        assert_eq!(csv_field("-5V"), "-5V");
        assert_eq!(csv_field("-cmd"), "'-cmd");
    }

    #[test]
    fn html_escapes_design_text() {
        let html = bom_to_html(&bom());
        assert!(html.contains("<title>Bill of Materials: d&lt;1&gt;</title>"));
        assert!(html.contains("<td>say &quot;hi&quot;</td>"));
        assert!(html.contains("<p>4 parts, 3 lines</p>"));
        assert!(!html.contains("d<1>"));
    }
}
