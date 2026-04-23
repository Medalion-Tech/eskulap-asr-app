import { Node, mergeAttributes } from "@tiptap/core";

/**
 * A single polymorphic Tiptap node type that represents any slot in the AST.
 * Attributes:
 *  - slotId: stable UUID; generated on insert, preserved on edit.
 *  - slotKind: "field" | "longtext" | "pick" | "list".
 *  - name, hint, default: free-form strings (JSON-unsafe content rejected upstream).
 *  - options: JSON string of PickOption[] (only for pick).
 *  - allowOther: bool string "true"/"false" (only for pick).
 *  - numbered: bool string (only for list).
 *
 * We intentionally use ONE node type with `kind` as an attribute rather than
 * four node types — simpler PM schema, simpler serializers, one NodeView.
 */
export const SlotNode = Node.create({
  name: "slot",

  // Inline for field/pick (sit in paragraphs), block for longtext/list.
  // We treat all as inline atomic + wrap block slots in their own paragraph
  // during serde to keep the schema single-typed.
  inline: true,
  group: "inline",
  atom: true,
  selectable: true,
  draggable: false,

  addAttributes() {
    return {
      slotId: { default: "" },
      slotKind: { default: "field" },
      name: { default: "nowy_slot" },
      hint: { default: null },
      default: { default: null },
      options: { default: "[]" },
      allowOther: { default: "false" },
      numbered: { default: "false" },
    };
  },

  parseHTML() {
    return [{ tag: "span[data-slot]" }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-slot": "true",
        class: "slot-chip-edit",
      }),
      `[${HTMLAttributes.slotKind}: ${HTMLAttributes.name}]`,
    ];
  },
});
