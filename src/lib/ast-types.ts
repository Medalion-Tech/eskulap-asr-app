// Mirror of src-tauri/src/ast.rs. Keep in sync manually.

export type SlotId = string;

export interface PickOption {
  code: string;
  text: string;
}

export type Slot =
  | { kind: "field"; id: SlotId; name: string; hint?: string | null; default?: string | null }
  | { kind: "longtext"; id: SlotId; name: string; hint?: string | null }
  | {
      kind: "pick";
      id: SlotId;
      name: string;
      hint?: string | null;
      options: PickOption[];
      allow_other: boolean;
    }
  | { kind: "list"; id: SlotId; name: string; hint?: string | null; numbered: boolean };

export type Inline = { kind: "text"; text: string } | { kind: "slot"; id: SlotId };

export type Node =
  | { kind: "heading"; level: number; inlines: Inline[] }
  | { kind: "paragraph"; inlines: Inline[] }
  | { kind: "slot_block"; id: SlotId }
  | { kind: "list_block"; numbered: boolean; items: Inline[][] };

export interface TemplateAst {
  nodes: Node[];
  slots: Record<SlotId, Slot>;
}

export type FilledValue =
  | { kind: "text"; text: string }
  | { kind: "pick"; code: string; custom_text?: string | null }
  | { kind: "list"; items: string[] }
  | { kind: "unfilled" };

export interface FilledTemplate {
  template_id: string;
  values: Record<SlotId, FilledValue>;
  user_edited: SlotId[];
}

export interface GenerationResult {
  display_text: string;
  filled: FilledTemplate;
  raw_output: string;
  parse_quality_low: boolean;
  total_slots: number;
  parsed_ok: number;
  cache_hit: boolean;
}

// ---------- Helpers ----------

export function slotOrder(ast: TemplateAst): SlotId[] {
  const ids: SlotId[] = [];
  const pushInlines = (inlines: Inline[]) => {
    for (const i of inlines) {
      if (i.kind === "slot") ids.push(i.id);
    }
  };
  for (const node of ast.nodes) {
    switch (node.kind) {
      case "heading":
      case "paragraph":
        pushInlines(node.inlines);
        break;
      case "slot_block":
        ids.push(node.id);
        break;
      case "list_block":
        for (const item of node.items) pushInlines(item);
        break;
    }
  }
  return ids;
}

/// Is this slot visually a block (owns its own line) or inline in a paragraph?
export function isBlockSlot(ast: TemplateAst, slotId: SlotId): boolean {
  for (const node of ast.nodes) {
    if (node.kind === "slot_block" && node.id === slotId) return true;
  }
  return false;
}

export function renderFilledValueAsText(
  slot: Slot,
  value: FilledValue | undefined,
): string {
  const unfilled = "[nie wspomniano]";
  if (!value || value.kind === "unfilled") return unfilled;
  if (value.kind === "text") return value.text;
  if (value.kind === "list") return value.items.join(", ");
  if (value.kind === "pick") {
    if (value.code === "other") return value.custom_text ?? unfilled;
    if (value.code === "X") return unfilled;
    if (slot.kind === "pick") {
      const opt = slot.options.find((o) => o.code === value.code);
      if (opt) return opt.text;
    }
    return value.custom_text ?? unfilled;
  }
  return unfilled;
}

export function slotStatus(
  filled: FilledTemplate,
  slotId: SlotId,
): "filled" | "unfilled" | "edited" {
  if (filled.user_edited.includes(slotId)) return "edited";
  const v = filled.values[slotId];
  if (!v || v.kind === "unfilled") return "unfilled";
  return "filled";
}
