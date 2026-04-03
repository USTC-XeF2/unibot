import {
  type JSONContent,
  mergeAttributes,
  Node as TiptapNode,
} from "@tiptap/core";
import Document from "@tiptap/extension-document";
import Hardbreak from "@tiptap/extension-hard-break";
import Link from "@tiptap/extension-link";
import Paragraph from "@tiptap/extension-paragraph";
import Placeholder from "@tiptap/extension-placeholder";
import Text from "@tiptap/extension-text";
import { Dropcursor, TrailingNode, UndoRedo } from "@tiptap/extensions";
import { EditorContent, useEditor } from "@tiptap/react";
import * as React from "react";
import { useEffect, useImperativeHandle, useMemo, useRef } from "react";
import type { FaceDefinition } from "@/lib/face-library";
import { getFaceById } from "@/lib/face-library";
import { cn } from "@/lib/utils";
import type { MessageSegment } from "@/types/chat";

type ChatComposerProps = {
  segments: MessageSegment[];
  onSegmentsChange: (segments: MessageSegment[]) => void;
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

const Face = TiptapNode.create({
  name: "face",
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

function messageSegmentsToEditorContent(
  segments: MessageSegment[],
): JSONContent {
  const content = segments.flatMap((segment) => {
    if (segment.type === "Text") {
      return inlineContentFromText(segment.data.text);
    }

    if (segment.type === "At") {
      return inlineContentFromText(`@${segment.data.target}`);
    }

    if (segment.type === "Face") {
      const face = getFaceById(segment.data.id);
      return [
        {
          type: "face",
          attrs: {
            faceId: segment.data.id,
            label: face?.label ?? "",
          },
        },
      ];
    }

    return inlineContentFromText("[图片]");
  });

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

function pushTextSegment(segments: MessageSegment[], text: string) {
  if (!text) {
    return;
  }

  const previous = segments[segments.length - 1];
  if (previous?.type === "Text") {
    previous.data.text += text;
    return;
  }

  segments.push({ type: "Text", data: { text } });
}

function appendSegmentsFromNode(
  node: JSONContent,
  segments: MessageSegment[],
  allowMentions: boolean,
) {
  if (node.type === "text") {
    pushTextSegment(segments, (node.text ?? "").replace(/\u00A0/g, " "));
    return;
  }

  if (node.type === "hardBreak") {
    pushTextSegment(segments, "\n");
    return;
  }

  if (node.type === "face") {
    const faceId = String(node.attrs?.faceId ?? "");
    if (faceId) {
      segments.push({ type: "Face", data: { id: faceId } });
      return;
    }

    const fallbackLabel =
      String(node.attrs?.label ?? "") || getFaceById(faceId)?.label || "表情";
    pushTextSegment(segments, `[${fallbackLabel}]`);
    return;
  }

  if (!node.content?.length) {
    return;
  }

  for (const child of node.content) {
    appendSegmentsFromNode(child, segments, allowMentions);
  }
}

function editorContentToSegments(
  content: JSONContent,
  allowMentions: boolean,
): MessageSegment[] {
  if (!content.content?.length) {
    return [];
  }

  const segments: MessageSegment[] = [];
  for (const node of content.content) {
    appendSegmentsFromNode(node, segments, allowMentions);
  }

  const lastSegment = segments[segments.length - 1];
  if (lastSegment?.type === "Text") {
    if (!lastSegment.data.text) {
      segments.pop();
    }
  }

  return segments;
}

function areSegmentsEqual(next: MessageSegment[], previous: MessageSegment[]) {
  return JSON.stringify(next) === JSON.stringify(previous);
}

const ChatComposer = React.forwardRef<ChatComposerHandle, ChatComposerProps>(
  (
    {
      segments,
      onSegmentsChange,
      disabled = false,
      allowMentions,
      placeholder,
      className,
      onSubmit,
    },
    ref,
  ) => {
    const onSegmentsChangeRef = useRef(onSegmentsChange);
    const onSubmitRef = useRef(onSubmit);
    const isApplyingExternalUpdateRef = useRef(false);
    const allowMentionsRef = useRef(allowMentions);

    onSegmentsChangeRef.current = onSegmentsChange;
    onSubmitRef.current = onSubmit;

    const content = useMemo(
      () => messageSegmentsToEditorContent(segments),
      [segments],
    );

    const editor = useEditor({
      extensions: [
        Document,
        Paragraph,
        Text,
        Dropcursor,
        TrailingNode,
        UndoRedo,
        Hardbreak,
        Placeholder.configure({
          placeholder,
          emptyEditorClass: "is-editor-empty",
        }),
        Link,
        Face,
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

        onSegmentsChangeRef.current(
          editorContentToSegments(
            nextEditor.getJSON(),
            allowMentionsRef.current,
          ),
        );
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
        areSegmentsEqual(
          editorContentToSegments(editor.getJSON(), allowMentions),
          segments,
        )
      ) {
        return;
      }

      isApplyingExternalUpdateRef.current = true;
      editor.commands.setContent(messageSegmentsToEditorContent(segments), {
        emitUpdate: false,
      });
      queueMicrotask(() => {
        isApplyingExternalUpdateRef.current = false;
      });
    }, [allowMentions, segments, editor]);

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
              type: "face",
              attrs: {
                faceId: face.id,
                label: face.label,
              },
            })
            .run();
        },
        insertMention(label, _userId) {
          editor
            ?.chain()
            .focus()
            .insertContent([
              {
                type: "text",
                text: `@${label} `,
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

export default ChatComposer;
