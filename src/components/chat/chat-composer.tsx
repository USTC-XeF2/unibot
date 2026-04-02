import {
  type JSONContent,
  mergeAttributes,
  Node as TiptapNode,
} from "@tiptap/core";
import Placeholder from "@tiptap/extension-placeholder";
import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import * as React from "react";
import { useEffect, useImperativeHandle, useMemo, useRef } from "react";
import type { FaceDefinition } from "@/lib/face-library";
import { getFaceById } from "@/lib/face-library";
import { tokenizeDraftForDisplay } from "@/lib/message-content";
import { cn } from "@/lib/utils";

type ChatComposerProps = {
  draft: string;
  onDraftChange: (draft: string) => void;
  disabled?: boolean;
  allowMentions: boolean;
  placeholder: string;
  className?: string;
  onSubmit: () => void;
};

export type ChatComposerHandle = {
  focus: () => void;
  moveCaretToEnd: () => void;
  insertFace: (face: FaceDefinition) => void;
  insertMention: (label: string, userId: number) => void;
};

const ChatFaceNode = TiptapNode.create({
  name: "chatFace",
  group: "inline",
  inline: true,
  atom: true,
  selectable: false,

  addAttributes() {
    return {
      faceId: { default: "" },
      label: { default: "" },
    };
  },

  parseHTML() {
    return [{ tag: "span[data-face-id]" }];
  },

  renderHTML({ HTMLAttributes }) {
    const faceId = String(HTMLAttributes.faceId ?? "");
    const label = String(HTMLAttributes.label ?? "");
    const face = getFaceById(faceId);

    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-face-id": faceId,
        contenteditable: "false",
        class:
          "mx-0.5 inline-flex select-none items-center justify-center align-[-0.25em]",
      }),
      [
        "img",
        {
          src: face?.src ?? "",
          alt: label || face?.label || "表情",
          draggable: "false",
          class: "size-4.5 object-contain",
        },
      ],
    ];
  },
});

const ChatMentionNode = TiptapNode.create({
  name: "chatMention",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,

  addAttributes() {
    return {
      label: { default: "" },
      userId: { default: 0 },
    };
  },

  parseHTML() {
    return [{ tag: "span[data-mention-user-id]" }];
  },

  renderHTML({ HTMLAttributes }) {
    const label = String(HTMLAttributes.label ?? "");
    const userId = Number(HTMLAttributes.userId ?? 0);

    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-mention-user-id": userId,
        contenteditable: "false",
        class:
          "mx-0.5 inline-flex select-none items-center rounded-md bg-sky-500/10 px-1.5 py-0.5 font-medium text-[13px] text-sky-700 dark:text-sky-300",
      }),
      `@${label}`,
    ];
  },
});

function inlineContentFromText(text: string): JSONContent[] {
  if (!text) {
    return [];
  }

  const parts = text.split("\n");
  const content: JSONContent[] = [];

  for (const [index, part] of parts.entries()) {
    if (part) {
      content.push({ type: "text", text: part });
    }
    if (index < parts.length - 1) {
      content.push({ type: "hardBreak" });
    }
  }

  return content;
}

function draftToEditorContent(
  draft: string,
  allowMentions: boolean,
): JSONContent {
  const content = tokenizeDraftForDisplay(draft, { allowMentions }).flatMap(
    (token) => {
      if (token.kind === "text") {
        return inlineContentFromText(token.text);
      }

      if (token.kind === "mention") {
        return [
          {
            type: "chatMention",
            attrs: {
              label: token.label,
              userId: token.userId,
            },
          },
        ];
      }

      const face = getFaceById(token.faceId);
      return [
        {
          type: "chatFace",
          attrs: {
            faceId: token.faceId,
            label: face?.label ?? "",
          },
        },
      ];
    },
  );

  return {
    type: "doc",
    content: [
      {
        type: "paragraph",
        content,
      },
    ],
  };
}

function draftFromNode(node: JSONContent): string {
  if (node.type === "text") {
    return node.text ?? "";
  }

  if (node.type === "hardBreak") {
    return "\n";
  }

  if (node.type === "chatFace") {
    const faceId = String(node.attrs?.faceId ?? "");
    const label =
      String(node.attrs?.label ?? "") || getFaceById(faceId)?.label || "表情";
    return `[${label}]`;
  }

  if (node.type === "chatMention") {
    const label = String(node.attrs?.label ?? "");
    const userId = Number(node.attrs?.userId ?? 0);
    return `[@${label}:${userId}]`;
  }

  if (!node.content?.length) {
    return "";
  }

  const inner = node.content.map(draftFromNode).join("");
  if (node.type === "paragraph") {
    return inner;
  }

  return inner;
}

function editorContentToDraft(content: JSONContent): string {
  if (!content.content?.length) {
    return "";
  }

  return content.content
    .map((node) => draftFromNode(node))
    .join("\n")
    .replace(/\u00A0/g, " ");
}

const ChatComposer = React.forwardRef<ChatComposerHandle, ChatComposerProps>(
  (
    {
      draft,
      onDraftChange,
      disabled = false,
      allowMentions,
      placeholder,
      className,
      onSubmit,
    },
    ref,
  ) => {
    const onDraftChangeRef = useRef(onDraftChange);
    const onSubmitRef = useRef(onSubmit);
    const isApplyingExternalUpdateRef = useRef(false);
    const allowMentionsRef = useRef(allowMentions);

    onDraftChangeRef.current = onDraftChange;
    onSubmitRef.current = onSubmit;

    const content = useMemo(
      () => draftToEditorContent(draft, allowMentions),
      [draft, allowMentions],
    );

    const editor = useEditor({
      extensions: [
        StarterKit.configure({
          blockquote: false,
          bulletList: false,
          code: false,
          codeBlock: false,
          dropcursor: false,
          gapcursor: false,
          heading: false,
          horizontalRule: false,
          listItem: false,
          orderedList: false,
          strike: false,
        }),
        Placeholder.configure({
          placeholder,
          emptyEditorClass: "is-editor-empty",
        }),
        ChatFaceNode,
        ChatMentionNode,
      ],
      content,
      immediatelyRender: false,
      editorProps: {
        attributes: {
          class: cn(
            "min-h-28 max-h-28 overflow-auto px-2.5 py-2 text-base leading-6 outline-none md:text-sm md:leading-5",
            "whitespace-pre-wrap break-words",
            "[&_.is-editor-empty:first-child::before]:pointer-events-none",
            "[&_.is-editor-empty:first-child::before]:float-left",
            "[&_.is-editor-empty:first-child::before]:h-0",
            "[&_.is-editor-empty:first-child::before]:text-muted-foreground",
            "[&_.is-editor-empty:first-child::before]:content-[attr(data-placeholder)]",
          ),
        },
        handleKeyDown(view, event) {
          if (event.key !== "Enter") {
            return false;
          }

          event.preventDefault();

          if (!event.shiftKey) {
            onSubmitRef.current();
            return true;
          }

          const hardBreak = view.state.schema.nodes.hardBreak;
          if (!hardBreak) {
            return false;
          }

          view.dispatch(
            view.state.tr
              .replaceSelectionWith(hardBreak.create())
              .scrollIntoView(),
          );
          return true;
        },
        handlePaste(view, event) {
          const text = event.clipboardData?.getData("text/plain");
          if (text === undefined) {
            return false;
          }

          event.preventDefault();
          view.dispatch(view.state.tr.insertText(text));
          return true;
        },
      },
      onUpdate({ editor: nextEditor }) {
        if (isApplyingExternalUpdateRef.current) {
          return;
        }

        onDraftChangeRef.current(editorContentToDraft(nextEditor.getJSON()));
      },
    });

    useEffect(() => {
      if (!editor) {
        return;
      }

      editor.setEditable(!disabled);
    }, [disabled, editor]);

    useEffect(() => {
      if (!editor) {
        return;
      }

      const allowMentionsChanged = allowMentionsRef.current !== allowMentions;
      allowMentionsRef.current = allowMentions;

      if (
        !allowMentionsChanged &&
        editorContentToDraft(editor.getJSON()) === draft
      ) {
        return;
      }

      isApplyingExternalUpdateRef.current = true;
      editor.commands.setContent(draftToEditorContent(draft, allowMentions), {
        emitUpdate: false,
      });
      queueMicrotask(() => {
        isApplyingExternalUpdateRef.current = false;
      });
    }, [allowMentions, draft, editor]);

    useImperativeHandle(
      ref,
      () => ({
        focus() {
          editor?.commands.focus();
        },
        moveCaretToEnd() {
          editor?.commands.focus("end");
        },
        insertFace(face) {
          editor
            ?.chain()
            .focus()
            .insertContent({
              type: "chatFace",
              attrs: {
                faceId: face.id,
                label: face.label,
              },
            })
            .run();
        },
        insertMention(label, userId) {
          editor
            ?.chain()
            .focus()
            .insertContent([
              {
                type: "chatMention",
                attrs: {
                  label,
                  userId,
                },
              },
              {
                type: "text",
                text: " ",
              },
            ])
            .run();
        },
      }),
      [editor],
    );

    return (
      <EditorContent
        editor={editor}
        className={cn(
          "rounded-lg border-0 bg-transparent shadow-none focus-within:ring-0",
          disabled && "opacity-50",
          className,
        )}
      />
    );
  },
);

ChatComposer.displayName = "ChatComposer";

export default ChatComposer;
