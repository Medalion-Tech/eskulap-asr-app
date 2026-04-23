import type { JSONContent } from "@tiptap/core";
import type {
  Inline,
  Node as AstNode,
  PickOption,
  Slot,
  SlotId,
  TemplateAst,
} from "../ast-types";

function uuid(): string {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  // Fallback; non-cryptographic.
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    return (c === "x" ? r : (r & 0x3) | 0x8).toString(16);
  });
}

// ---------- AST → Tiptap doc ----------

export function astToDoc(ast: TemplateAst): JSONContent {
  const content: JSONContent[] = [];
  for (const node of ast.nodes) {
    content.push(astNodeToDocNode(node, ast));
  }
  if (content.length === 0) {
    content.push({ type: "paragraph" });
  }
  return { type: "doc", content };
}

function astNodeToDocNode(node: AstNode, ast: TemplateAst): JSONContent {
  switch (node.kind) {
    case "heading":
      return {
        type: "heading",
        attrs: { level: Math.min(Math.max(node.level, 1), 6) },
        content: inlinesToDoc(node.inlines, ast),
      };
    case "paragraph":
      return { type: "paragraph", content: inlinesToDoc(node.inlines, ast) };
    case "slot_block": {
      const slot = ast.slots[node.id];
      return {
        type: "paragraph",
        content: slot ? [slotToDocNode(slot)] : [],
      };
    }
    case "list_block":
      return {
        type: node.numbered ? "orderedList" : "bulletList",
        content: node.items.map((item) => ({
          type: "listItem",
          content: [{ type: "paragraph", content: inlinesToDoc(item, ast) }],
        })),
      };
  }
}

function inlinesToDoc(inlines: Inline[], ast: TemplateAst): JSONContent[] {
  const out: JSONContent[] = [];
  for (const inl of inlines) {
    if (inl.kind === "text") {
      if (inl.text.length > 0) out.push({ type: "text", text: inl.text });
    } else {
      const slot = ast.slots[inl.id];
      if (slot) out.push(slotToDocNode(slot));
    }
  }
  return out;
}

function slotToDocNode(slot: Slot): JSONContent {
  switch (slot.kind) {
    case "field":
      return {
        type: "slot",
        attrs: {
          slotId: slot.id,
          slotKind: "field",
          name: slot.name,
          hint: slot.hint ?? null,
          default: slot.default ?? null,
        },
      };
    case "longtext":
      return {
        type: "slot",
        attrs: {
          slotId: slot.id,
          slotKind: "longtext",
          name: slot.name,
          hint: slot.hint ?? null,
        },
      };
    case "pick":
      return {
        type: "slot",
        attrs: {
          slotId: slot.id,
          slotKind: "pick",
          name: slot.name,
          hint: slot.hint ?? null,
          options: JSON.stringify(slot.options),
          allowOther: String(slot.allow_other),
        },
      };
    case "list":
      return {
        type: "slot",
        attrs: {
          slotId: slot.id,
          slotKind: "list",
          name: slot.name,
          hint: slot.hint ?? null,
          numbered: String(slot.numbered),
        },
      };
  }
}

// ---------- Tiptap doc → AST ----------

export function docToAst(doc: JSONContent): TemplateAst {
  const nodes: AstNode[] = [];
  const slots: Record<SlotId, Slot> = {};
  const seen = new Set<SlotId>();

  const register = (slot: Slot) => {
    if (seen.has(slot.id)) return;
    seen.add(slot.id);
    slots[slot.id] = slot;
  };

  for (const child of doc.content ?? []) {
    if (!child.type) continue;
    switch (child.type) {
      case "heading": {
        const level = Number(child.attrs?.level ?? 1);
        const { inlines, usedSlots } = docInlinesToAst(child.content ?? []);
        usedSlots.forEach(register);
        nodes.push({ kind: "heading", level, inlines });
        break;
      }
      case "paragraph": {
        const { inlines, usedSlots } = docInlinesToAst(child.content ?? []);
        usedSlots.forEach(register);

        // If paragraph holds exactly one block-kind slot (longtext/list) and
        // no text, emit a SlotBlock for cleaner stringification.
        const onlySlot =
          inlines.length === 1 &&
          inlines[0].kind === "slot" &&
          usedSlots.some(
            (s) =>
              s.id === (inlines[0] as { kind: "slot"; id: string }).id &&
              (s.kind === "longtext" || s.kind === "list"),
          );
        if (onlySlot) {
          nodes.push({ kind: "slot_block", id: (inlines[0] as { kind: "slot"; id: string }).id });
        } else {
          nodes.push({ kind: "paragraph", inlines });
        }
        break;
      }
      case "bulletList":
      case "orderedList": {
        const items: Inline[][] = [];
        for (const li of child.content ?? []) {
          const para = (li.content ?? []).find((c) => c.type === "paragraph");
          const { inlines, usedSlots } = docInlinesToAst(para?.content ?? []);
          usedSlots.forEach(register);
          items.push(inlines);
        }
        nodes.push({
          kind: "list_block",
          numbered: child.type === "orderedList",
          items,
        });
        break;
      }
      default:
        // Unknown node type — skip.
        break;
    }
  }

  return { nodes, slots };
}

function docInlinesToAst(content: JSONContent[]): {
  inlines: Inline[];
  usedSlots: Slot[];
} {
  const inlines: Inline[] = [];
  const usedSlots: Slot[] = [];

  for (const child of content) {
    if (child.type === "text") {
      inlines.push({ kind: "text", text: child.text ?? "" });
    } else if (child.type === "slot") {
      const attrs = child.attrs ?? {};
      // Preserve existing slot ID; assign a new one only if missing.
      const slotId: SlotId = (attrs.slotId as string) || uuid();
      const slot = attrsToSlot(slotId, attrs);
      usedSlots.push(slot);
      inlines.push({ kind: "slot", id: slotId });
    }
    // hardBreak, etc: ignored.
  }

  return { inlines, usedSlots };
}

function attrsToSlot(slotId: SlotId, attrs: Record<string, unknown>): Slot {
  const kind = (attrs.slotKind as string) ?? "field";
  const name = sanitizeName((attrs.name as string) ?? "slot");
  const hint = (attrs.hint as string | null | undefined) || null;

  switch (kind) {
    case "longtext":
      return { kind: "longtext", id: slotId, name, hint };
    case "pick": {
      let options: PickOption[] = [];
      try {
        options = JSON.parse((attrs.options as string) ?? "[]");
      } catch {
        options = [];
      }
      const allowOther = String(attrs.allowOther ?? "false") === "true";
      return { kind: "pick", id: slotId, name, hint, options, allow_other: allowOther };
    }
    case "list":
      return {
        kind: "list",
        id: slotId,
        name,
        hint,
        numbered: String(attrs.numbered ?? "false") === "true",
      };
    case "field":
    default:
      return {
        kind: "field",
        id: slotId,
        name,
        hint,
        default: (attrs.default as string | null | undefined) || null,
      };
  }
}

function sanitizeName(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9_]/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_|_$/g, "")
    .slice(0, 40);
}
