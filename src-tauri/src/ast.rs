use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type SlotId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Slot {
    Field {
        id: SlotId,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<String>,
    },
    Longtext {
        id: SlotId,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
    },
    Pick {
        id: SlotId,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
        options: Vec<PickOption>,
        #[serde(default)]
        allow_other: bool,
    },
    List {
        id: SlotId,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
        #[serde(default)]
        numbered: bool,
    },
}

impl Slot {
    pub fn id(&self) -> &str {
        match self {
            Slot::Field { id, .. }
            | Slot::Longtext { id, .. }
            | Slot::Pick { id, .. }
            | Slot::List { id, .. } => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Slot::Field { name, .. }
            | Slot::Longtext { name, .. }
            | Slot::Pick { name, .. }
            | Slot::List { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PickOption {
    pub code: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Inline {
    Text { text: String },
    Slot { id: SlotId },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Node {
    Heading { level: u8, inlines: Vec<Inline> },
    Paragraph { inlines: Vec<Inline> },
    SlotBlock { id: SlotId },
    ListBlock { numbered: bool, items: Vec<Vec<Inline>> },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TemplateAst {
    pub nodes: Vec<Node>,
    pub slots: BTreeMap<SlotId, Slot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FilledValue {
    Text { text: String },
    Pick { code: String, #[serde(default, skip_serializing_if = "Option::is_none")] custom_text: Option<String> },
    List { items: Vec<String> },
    Unfilled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FilledTemplate {
    pub template_id: String,
    pub values: BTreeMap<SlotId, FilledValue>,
    #[serde(default)]
    pub user_edited: Vec<SlotId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub struct ParseQuality {
    pub total_slots: u32,
    pub parsed_ok: u32,
    pub unfilled: u32,
    pub low: bool,
}

// ---------- Stringify for LLM ----------

/// Render the AST into the string shown to the LLM. Slots become `[[kind:name|...]]`.
pub fn stringify_for_llm(ast: &TemplateAst) -> String {
    let mut out = String::new();
    for (i, node) in ast.nodes.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        render_node_stringified(ast, node, &mut out);
    }
    out
}

fn render_node_stringified(ast: &TemplateAst, node: &Node, out: &mut String) {
    match node {
        Node::Heading { level, inlines } => {
            out.push_str(&"#".repeat(*level as usize));
            out.push(' ');
            render_inlines_stringified(ast, inlines, out);
            out.push('\n');
        }
        Node::Paragraph { inlines } => {
            render_inlines_stringified(ast, inlines, out);
            out.push('\n');
        }
        Node::SlotBlock { id } => {
            if let Some(slot) = ast.slots.get(id) {
                out.push_str(&slot_marker(slot));
                out.push('\n');
            }
        }
        Node::ListBlock { numbered, items } => {
            for (i, item) in items.iter().enumerate() {
                if *numbered {
                    out.push_str(&format!("{}. ", i + 1));
                } else {
                    out.push_str("- ");
                }
                render_inlines_stringified(ast, item, out);
                out.push('\n');
            }
        }
    }
}

fn render_inlines_stringified(ast: &TemplateAst, inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::Slot { id } => {
                if let Some(slot) = ast.slots.get(id) {
                    out.push_str(&slot_marker(slot));
                }
            }
        }
    }
}

fn slot_marker(slot: &Slot) -> String {
    match slot {
        Slot::Field { name, hint, default, .. } => {
            let mut s = format!("[[field:{}", name);
            if let Some(h) = hint {
                s.push_str(&format!("|hint={}", escape_attr(h)));
            }
            if let Some(d) = default {
                s.push_str(&format!("|default={}", escape_attr(d)));
            }
            s.push_str("]]");
            s
        }
        Slot::Longtext { name, hint, .. } => {
            let mut s = format!("[[longtext:{}", name);
            if let Some(h) = hint {
                s.push_str(&format!("|hint={}", escape_attr(h)));
            }
            s.push_str("]]");
            s
        }
        Slot::Pick { name, hint, options, allow_other, .. } => {
            let mut s = format!("[[pick:{}", name);
            for opt in options {
                s.push_str(&format!("|{}={}", opt.code, escape_attr(&opt.text)));
            }
            if *allow_other {
                s.push_str("|other");
            }
            if let Some(h) = hint {
                s.push_str(&format!("|hint={}", escape_attr(h)));
            }
            s.push_str("]]");
            s
        }
        Slot::List { name, hint, numbered, .. } => {
            let mut s = format!("[[list:{}", name);
            if *numbered {
                s.push_str("|numbered=true");
            }
            if let Some(h) = hint {
                s.push_str(&format!("|hint={}", escape_attr(h)));
            }
            s.push_str("]]");
            s
        }
    }
}

fn escape_attr(s: &str) -> String {
    s.replace('|', "\\|").replace("]]", "\\]\\]")
}

// ---------- Render with values (one-shot example output) ----------

/// Render AST to string, substituting each slot marker with its filled value
/// in the same positions. Used to build the one-shot `example_output` shown to the LLM.
pub fn render_with_values(ast: &TemplateAst, filled: &FilledTemplate) -> String {
    let mut out = String::new();
    for (i, node) in ast.nodes.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        render_node_with_values(ast, filled, node, &mut out);
    }
    out
}

fn render_node_with_values(ast: &TemplateAst, filled: &FilledTemplate, node: &Node, out: &mut String) {
    match node {
        Node::Heading { level, inlines } => {
            out.push_str(&"#".repeat(*level as usize));
            out.push(' ');
            render_inlines_with_values(ast, filled, inlines, out);
            out.push('\n');
        }
        Node::Paragraph { inlines } => {
            render_inlines_with_values(ast, filled, inlines, out);
            out.push('\n');
        }
        Node::SlotBlock { id } => {
            if let Some(slot) = ast.slots.get(id) {
                render_slot_value(slot, filled.values.get(id), true, out);
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
        }
        Node::ListBlock { numbered, items } => {
            for (i, item) in items.iter().enumerate() {
                if *numbered {
                    out.push_str(&format!("{}. ", i + 1));
                } else {
                    out.push_str("- ");
                }
                render_inlines_with_values(ast, filled, item, out);
                out.push('\n');
            }
        }
    }
}

fn render_inlines_with_values(ast: &TemplateAst, filled: &FilledTemplate, inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::Slot { id } => {
                if let Some(slot) = ast.slots.get(id) {
                    render_slot_value(slot, filled.values.get(id), false, out);
                }
            }
        }
    }
}

fn render_slot_value(slot: &Slot, value: Option<&FilledValue>, block_context: bool, out: &mut String) {
    let unfilled = "[nie wspomniano]";
    match (slot, value) {
        (_, Some(FilledValue::Unfilled)) | (_, None) => {
            out.push_str(unfilled);
        }
        (Slot::Field { .. } | Slot::Longtext { .. }, Some(FilledValue::Text { text })) => {
            out.push_str(text);
        }
        (Slot::Pick { options, .. }, Some(FilledValue::Pick { code, custom_text })) => {
            if code == "other" {
                if let Some(t) = custom_text {
                    out.push_str(t);
                } else {
                    out.push_str(unfilled);
                }
            } else if code == "X" {
                out.push_str(unfilled);
            } else if let Some(opt) = options.iter().find(|o| o.code == *code) {
                out.push_str(&opt.text);
            } else if let Some(t) = custom_text {
                out.push_str(t);
            } else {
                out.push_str(unfilled);
            }
        }
        (Slot::List { numbered, .. }, Some(FilledValue::List { items })) => {
            if block_context {
                for (i, item) in items.iter().enumerate() {
                    if *numbered {
                        out.push_str(&format!("{}. ", i + 1));
                    } else {
                        out.push_str("- ");
                    }
                    out.push_str(item);
                    out.push('\n');
                }
            } else {
                // Inline list: comma-separated
                out.push_str(&items.join(", "));
            }
        }
        _ => out.push_str(unfilled),
    }
}

// ---------- Display (plain text for Note.text) ----------

/// Human-readable plain text rendering used for Note.text (and clipboard).
/// Identical semantically to render_with_values; separate function so we can
/// tune formatting independently (e.g. bold headings) later.
pub fn render_display(ast: &TemplateAst, filled: &FilledTemplate) -> String {
    let mut out = String::new();
    for (i, node) in ast.nodes.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match node {
            Node::Heading { level, inlines } => {
                if *level == 1 {
                    let mut line = String::new();
                    render_inlines_with_values(ast, filled, inlines, &mut line);
                    out.push_str(&line.to_uppercase());
                } else {
                    render_inlines_with_values(ast, filled, inlines, &mut out);
                }
                out.push('\n');
            }
            Node::Paragraph { inlines } => {
                render_inlines_with_values(ast, filled, inlines, &mut out);
                out.push('\n');
            }
            Node::SlotBlock { id } => {
                if let Some(slot) = ast.slots.get(id) {
                    render_slot_value(slot, filled.values.get(id), true, &mut out);
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                }
            }
            Node::ListBlock { numbered, items } => {
                for (i, item) in items.iter().enumerate() {
                    if *numbered {
                        out.push_str(&format!("{}. ", i + 1));
                    } else {
                        out.push_str("- ");
                    }
                    render_inlines_with_values(ast, filled, item, &mut out);
                    out.push('\n');
                }
            }
        }
    }
    out.trim_end().to_string()
}

// ---------- Slot enumeration in document order ----------

/// All slot IDs in the order they appear when walking the AST.
pub fn slot_order(ast: &TemplateAst) -> Vec<SlotId> {
    let mut ids = Vec::new();
    for node in &ast.nodes {
        collect_slots_from_node(node, &mut ids);
    }
    ids
}

fn collect_slots_from_node(node: &Node, ids: &mut Vec<SlotId>) {
    match node {
        Node::Heading { inlines, .. } | Node::Paragraph { inlines } => {
            for i in inlines {
                if let Inline::Slot { id } = i {
                    ids.push(id.clone());
                }
            }
        }
        Node::SlotBlock { id } => ids.push(id.clone()),
        Node::ListBlock { items, .. } => {
            for item in items {
                for i in item {
                    if let Inline::Slot { id } = i {
                        ids.push(id.clone());
                    }
                }
            }
        }
    }
}

// ---------- Parser: LLM output → FilledTemplate ----------

/// Parse the LLM's output back into a FilledTemplate. Uses the stringified
/// AST as a skeleton: split it on slot markers, then match literal anchors
/// in the output to extract per-slot values.
pub fn parse_llm_output(ast: &TemplateAst, output: &str) -> (FilledTemplate, ParseQuality) {
    let template_str = stringify_for_llm(ast);
    let slot_ids = slot_order(ast);
    let (anchors, _) = split_on_slot_markers(&template_str, &slot_ids);

    let mut filled = FilledTemplate {
        template_id: String::new(),
        values: BTreeMap::new(),
        user_edited: Vec::new(),
    };
    let mut quality = ParseQuality {
        total_slots: slot_ids.len() as u32,
        parsed_ok: 0,
        unfilled: 0,
        low: false,
    };

    // anchors.len() == slot_ids.len() + 1. Walk output matching each anchor's
    // last distinctive word (whitespace-tolerant).
    let mut cursor = 0usize;

    // Skip leading anchor into cursor past its last distinctive word.
    if let Some(first) = anchors.first() {
        if let Some((_, after)) = find_anchor_span(output, cursor, first) {
            cursor = after;
        }
    }

    for (i, slot_id) in slot_ids.iter().enumerate() {
        let next_anchor = anchors.get(i + 1).map(|s| s.as_str()).unwrap_or("");
        let (value_end, next_cursor) = if next_anchor.trim().is_empty() {
            (output.len(), output.len())
        } else {
            find_anchor_span(output, cursor, next_anchor)
                .unwrap_or((output.len(), output.len()))
        };
        let raw_value = output
            .get(cursor..value_end)
            .unwrap_or("")
            .trim()
            .to_string();
        let slot = ast.slots.get(slot_id);
        let (val, ok) = interpret_raw_value(slot, &raw_value);
        if matches!(val, FilledValue::Unfilled) {
            quality.unfilled += 1;
        }
        if ok {
            quality.parsed_ok += 1;
        }
        filled.values.insert(slot_id.clone(), val);
        cursor = next_cursor;
    }

    if quality.total_slots > 0 {
        let ok_ratio = quality.parsed_ok as f32 / quality.total_slots as f32;
        quality.low = ok_ratio < 0.5;
    }

    (filled, quality)
}

fn split_on_slot_markers(template_str: &str, slot_ids: &[SlotId]) -> (Vec<String>, ()) {
    // Split on every `[[...]]` marker. We don't rely on ids matching — marker
    // order in the template string matches slot_order(ast) by construction.
    let mut anchors = Vec::with_capacity(slot_ids.len() + 1);
    let mut rest = template_str;
    loop {
        match rest.find("[[") {
            Some(start) => {
                anchors.push(rest[..start].to_string());
                match rest[start..].find("]]") {
                    Some(end_rel) => {
                        let end = start + end_rel + 2;
                        rest = &rest[end..];
                    }
                    None => {
                        // Malformed; dump rest
                        anchors.last_mut().unwrap().push_str(&rest[start..]);
                        rest = "";
                        break;
                    }
                }
            }
            None => {
                anchors.push(rest.to_string());
                break;
            }
        }
    }
    // Ensure we have slot_ids.len() + 1 anchors (pad with empty if fewer markers than slots).
    while anchors.len() < slot_ids.len() + 1 {
        anchors.push(String::new());
    }
    (anchors, ())
}

/// Find the span of `anchor` in `hay` starting at `from`. Returns
/// `(value_end, cursor_after)` where:
/// - `value_end` is the absolute byte offset of the anchor's first distinctive
///   word (i.e. where the preceding slot value ends),
/// - `cursor_after` is the absolute byte offset just past the anchor's last
///   distinctive word (where the next slot value starts).
///
/// Whitespace-tolerant: we match on non-whitespace words only, which lets
/// the LLM drop or collapse blank lines without breaking the parser.
fn find_anchor_span(hay: &str, from: usize, anchor: &str) -> Option<(usize, usize)> {
    let trimmed = anchor.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Words: split on whitespace, drop very short noise tokens.
    let words: Vec<&str> = trimmed
        .split_whitespace()
        .filter(|w| w.len() >= 2)
        .collect();
    if words.is_empty() {
        // Single-letter anchor: do a direct match on trimmed.
        let pos = hay[from..].find(trimmed)?;
        let start = from + pos;
        return Some((start, start + trimmed.len()));
    }

    // Anchor the FIRST word → gives us value_end.
    let first_word = words[0];
    let first_pos = hay[from..].find(first_word)?;
    let value_end = from + first_pos;

    // Starting from value_end, walk words[1..] greedily to find the LAST word
    // and position cursor past it. If subsequent words aren't found (LLM
    // dropped structure), stop early.
    let mut cursor = value_end + first_word.len();
    for w in words.iter().skip(1) {
        if let Some(pos) = hay[cursor..].find(*w) {
            cursor = cursor + pos + w.len();
        }
    }
    Some((value_end, cursor))
}

fn interpret_raw_value(slot: Option<&Slot>, raw: &str) -> (FilledValue, bool) {
    // LLM often inserts its own punctuation between the label and the value
    // (e.g. "Powód przyjęcia: <value>") even when the template didn't have
    // a colon. Strip leading separators/whitespace from the captured value
    // so they don't leak into field/longtext/list items.
    let trimmed = raw
        .trim()
        .trim_start_matches(|c: char| {
            matches!(c, ':' | '-' | '—' | '–' | '•' | '.' | ',' | ';' | '*')
        })
        .trim();
    if trimmed.is_empty() || trimmed == "[nie wspomniano]" || trimmed == "[[X]]" || trimmed == "X" {
        return (FilledValue::Unfilled, false);
    }
    // If the LLM echoed back the template marker itself, treat as Unfilled.
    if is_only_slot_marker(trimmed) {
        return (FilledValue::Unfilled, false);
    }
    match slot {
        Some(Slot::Field { .. }) | Some(Slot::Longtext { .. }) => {
            (FilledValue::Text { text: trimmed.to_string() }, true)
        }
        Some(Slot::Pick { options, allow_other, .. }) => {
            // Try "other: <text>" pattern first.
            if let Some(rest) = trimmed.strip_prefix("other:").or_else(|| trimmed.strip_prefix("Other:")) {
                return (
                    FilledValue::Pick { code: "other".to_string(), custom_text: Some(rest.trim().to_string()) },
                    *allow_other,
                );
            }
            // Extract first "word" (letter+digits, short) as candidate code.
            let first_token: String = trimmed
                .chars()
                .take_while(|c| c.is_alphanumeric())
                .collect();
            if first_token.is_empty() {
                return (
                    FilledValue::Pick { code: "other".to_string(), custom_text: Some(trimmed.to_string()) },
                    false,
                );
            }
            if options.iter().any(|o| o.code == first_token) {
                (FilledValue::Pick { code: first_token, custom_text: None }, true)
            } else if first_token == "X" {
                (FilledValue::Unfilled, false)
            } else {
                // Fallback: match by option text (case-insensitive contains).
                let lower = trimmed.to_lowercase();
                if let Some(opt) = options.iter().find(|o| lower.contains(&o.text.to_lowercase())) {
                    (FilledValue::Pick { code: opt.code.clone(), custom_text: None }, true)
                } else {
                    (
                        FilledValue::Pick { code: "other".to_string(), custom_text: Some(trimmed.to_string()) },
                        false,
                    )
                }
            }
        }
        Some(Slot::List { .. }) => {
            let items: Vec<String> = trimmed
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|l| strip_list_bullet(l).trim().to_string())
                .filter(|l| {
                    // Skip short punctuation-only noise (e.g. stray ":") that the
                    // anchor matcher might have bled in before the first bullet.
                    !l.is_empty()
                        && l.chars().any(|c| c.is_alphanumeric())
                        && l.len() >= 2
                })
                .collect();
            if items.is_empty() {
                (FilledValue::Unfilled, false)
            } else {
                (FilledValue::List { items }, true)
            }
        }
        None => (FilledValue::Unfilled, false),
    }
}

/// True if the string is a single template marker, optionally with surrounding
/// whitespace/punctuation. Used by the parser to reject LLM outputs that
/// simply echo the template instead of filling slots.
fn is_only_slot_marker(s: &str) -> bool {
    let t = s.trim().trim_matches(|c: char| c == '.' || c == ',' || c == ';' || c == ':');
    if !t.starts_with("[[") || !t.ends_with("]]") {
        return false;
    }
    // Must contain exactly one `[[..]]` pair: the second `[[` must not appear
    // anywhere inside (other than the opening).
    let inner = &t[2..t.len() - 2];
    !inner.contains("[[") && !inner.contains("]]")
}

fn strip_list_bullet(line: &str) -> &str {
    let t = line.trim_start();
    if let Some(rest) = t.strip_prefix("- ") {
        return rest;
    }
    if let Some(rest) = t.strip_prefix("* ") {
        return rest;
    }
    // "1. foo", "12. foo"
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        let after = &t[digits.len()..];
        if let Some(rest) = after.strip_prefix(". ").or_else(|| after.strip_prefix(") ")) {
            return rest;
        }
    }
    t
}

// ---------- AST hash (for KV cache key) ----------

pub fn ast_hash(ast: &TemplateAst) -> String {
    use sha2::{Digest, Sha256};
    let json = serde_json::to_string(ast).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(json.as_bytes());
    hex::encode(h.finalize())[..16].to_string()
}

// ---------- UUIDv5 helper for deterministic built-in slot IDs ----------

pub fn deterministic_slot_id(template_id: &str, slot_name: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"eskulap-slot-v1\0");
    h.update(template_id.as_bytes());
    h.update(b"\0");
    h.update(slot_name.as_bytes());
    let digest = h.finalize();
    let bytes = &digest[..16];
    // Format as UUIDv4-ish string (version/variant bits not strictly set — that's fine for our use).
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11],
        bytes[12], bytes[13], bytes[14], bytes[15],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ast() -> TemplateAst {
        let mut slots = BTreeMap::new();
        slots.insert("s1".into(), Slot::Field {
            id: "s1".into(), name: "powod".into(), hint: Some("1 zdanie".into()), default: None,
        });
        slots.insert("s2".into(), Slot::Pick {
            id: "s2".into(), name: "stan".into(), hint: None,
            options: vec![
                PickOption { code: "A".into(), text: "dobry".into() },
                PickOption { code: "B".into(), text: "średni".into() },
                PickOption { code: "X".into(), text: "nieokreślone".into() },
            ],
            allow_other: true,
        });
        slots.insert("s3".into(), Slot::List {
            id: "s3".into(), name: "zalecenia".into(), hint: None, numbered: true,
        });
        TemplateAst {
            nodes: vec![
                Node::Heading { level: 1, inlines: vec![Inline::Text { text: "WIZYTA".into() }] },
                Node::Paragraph { inlines: vec![
                    Inline::Text { text: "Powód: ".into() },
                    Inline::Slot { id: "s1".into() },
                ]},
                Node::Paragraph { inlines: vec![
                    Inline::Text { text: "Stan: ".into() },
                    Inline::Slot { id: "s2".into() },
                ]},
                Node::Heading { level: 2, inlines: vec![Inline::Text { text: "Zalecenia".into() }] },
                Node::SlotBlock { id: "s3".into() },
            ],
            slots,
        }
    }

    #[test]
    fn stringify_roundtrip() {
        let ast = sample_ast();
        let s = stringify_for_llm(&ast);
        assert!(s.contains("[[field:powod|hint=1 zdanie]]"));
        assert!(s.contains("[[pick:stan|A=dobry|B=średni|X=nieokreślone|other]]"));
        assert!(s.contains("[[list:zalecenia|numbered=true]]"));
    }

    #[test]
    fn parse_clean_output() {
        let ast = sample_ast();
        let output = "# WIZYTA\nPowód: ból głowy\nStan: A\n## Zalecenia\n1. Paracetamol 500mg\n2. Kontrola za tydzień";
        let (filled, q) = parse_llm_output(&ast, output);
        assert_eq!(q.total_slots, 3);
        assert!(matches!(filled.values.get("s1"), Some(FilledValue::Text { text }) if text == "ból głowy"));
        assert!(matches!(filled.values.get("s2"), Some(FilledValue::Pick { code, .. }) if code == "A"));
        if let Some(FilledValue::List { items }) = filled.values.get("s3") {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], "Paracetamol 500mg");
        } else {
            panic!("list slot not parsed");
        }
    }

    #[test]
    fn parse_unfilled_sentinels() {
        let ast = sample_ast();
        let output = "# WIZYTA\nPowód: [[X]]\nStan: X\n## Zalecenia\n";
        let (filled, _q) = parse_llm_output(&ast, output);
        assert!(matches!(filled.values.get("s1"), Some(FilledValue::Unfilled)));
        assert!(matches!(filled.values.get("s2"), Some(FilledValue::Unfilled)));
    }

    #[test]
    fn parse_other_pick() {
        let ast = sample_ast();
        let output = "# WIZYTA\nPowód: grypa\nStan: other: pacjent w szoku septycznym\n## Zalecenia\n- Hospitalizacja";
        let (filled, _) = parse_llm_output(&ast, output);
        if let Some(FilledValue::Pick { code, custom_text }) = filled.values.get("s2") {
            assert_eq!(code, "other");
            assert_eq!(custom_text.as_deref(), Some("pacjent w szoku septycznym"));
        } else {
            panic!("expected Pick other");
        }
    }

    #[test]
    fn render_display_substitutes_values() {
        let ast = sample_ast();
        let mut filled = FilledTemplate::default();
        filled.values.insert("s1".into(), FilledValue::Text { text: "ból głowy".into() });
        filled.values.insert("s2".into(), FilledValue::Pick { code: "A".into(), custom_text: None });
        filled.values.insert("s3".into(), FilledValue::List { items: vec!["Paracetamol".into()] });
        let d = render_display(&ast, &filled);
        assert!(d.contains("WIZYTA"));
        assert!(d.contains("Powód: ból głowy"));
        assert!(d.contains("Stan: dobry"));
        assert!(d.contains("1. Paracetamol"));
    }

    #[test]
    fn ast_hash_is_stable() {
        let ast = sample_ast();
        let h1 = ast_hash(&ast);
        let h2 = ast_hash(&ast);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
    }
}
