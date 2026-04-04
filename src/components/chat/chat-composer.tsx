import {
  type JSONContent,
  mergeAttributes,
  Node as TiptapNode,
} from "@tiptap/core";
import Document from "@tiptap/extension-document";
import Hardbreak from "@tiptap/extension-hard-break";
import Link from "@tiptap/extension-link";
import Mention, { type MentionNodeAttrs } from "@tiptap/extension-mention";
import Paragraph from "@tiptap/extension-paragraph";
import Placeholder from "@tiptap/extension-placeholder";
import Text from "@tiptap/extension-text";
import { Dropcursor, TrailingNode, UndoRedo } from "@tiptap/extensions";
import { EditorContent, ReactRenderer, useEditor } from "@tiptap/react";
import type { SuggestionOptions, SuggestionProps } from "@tiptap/suggestion";
import * as React from "react";
import { useEffect, useImperativeHandle, useMemo, useRef } from "react";
import tippy, { type Instance } from "tippy.js";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { getFaceById } from "@/lib/face-library";
import { cn } from "@/lib/utils";
import type { MessageSegment } from "@/types/chat";

export interface MentionableUser {
  id: number | "all";
  name: string;
  avatar?: string;
}

export type MentionSupportLevel = "none" | "user" | "all";

const MENTION_ALL_MEMBER: MentionableUser = {
  id: "all",
  name: "全体成员",
};

type ChatComposerProps = {
  segments: MessageSegment[];
  onSegmentsChange: (segments: MessageSegment[]) => void;
  disabled?: boolean;
  mentionSupport: MentionSupportLevel;
  mentionableMembers: MentionableUser[];
  placeholder: string;
  className?: string;
  onSubmit: () => void;
};

export type ChatComposerHandle = {
  focus: () => void;
  moveCaretToEnd: () => void;
  insertFace: (faceId: string) => void;
  insertMention: (target: number | "all") => void;
  clear: () => void;
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
  mentionableMembers: MentionableUser[],
): JSONContent {
  const mentionLabelById = new Map<number, string>(
    mentionableMembers
      .filter(
        (member): member is MentionableUser & { id: number } =>
          typeof member.id === "number",
      )
      .map((member) => [member.id, member.name]),
  );

  const content = segments.flatMap((segment) => {
    if (segment.type === "Text") {
      return inlineContentFromText(segment.data.text);
    }

    if (segment.type === "At") {
      return [
        {
          type: "mention",
          attrs: {
            id: String(segment.data.target),
            label:
              mentionLabelById.get(segment.data.target) ??
              String(segment.data.target),
          },
        },
      ];
    }

    if (segment.type === "AtAll") {
      return [
        {
          type: "mention",
          attrs: {
            id: "all",
            label: MENTION_ALL_MEMBER.name,
          },
        },
      ];
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
  mentionSupport: MentionSupportLevel,
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

  if (node.type === "mention") {
    const mentionId = String(node.attrs?.id ?? "");
    if (mentionId === "all" && mentionSupport === "all") {
      segments.push({ type: "AtAll" });
      return;
    }

    const userId = Number(mentionId);
    if (mentionSupport !== "none" && Number.isInteger(userId) && userId > 0) {
      segments.push({ type: "At", data: { target: userId } });
      return;
    }

    const fallbackLabel = String(node.attrs?.label ?? mentionId);
    if (fallbackLabel) {
      pushTextSegment(segments, `@${fallbackLabel}`);
    }
    return;
  }

  if (!node.content?.length) {
    return;
  }

  for (const child of node.content) {
    appendSegmentsFromNode(child, segments, mentionSupport);
  }
}

function editorContentToSegments(
  content: JSONContent,
  mentionSupport: MentionSupportLevel,
): MessageSegment[] {
  if (!content.content?.length) {
    return [];
  }

  const segments: MessageSegment[] = [];
  for (const node of content.content) {
    appendSegmentsFromNode(node, segments, mentionSupport);
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

function MentionList(
  props: SuggestionProps<MentionableUser, MentionNodeAttrs>,
) {
  return (
    <Command className="w-48 border shadow-md">
      <CommandList>
        <CommandEmpty>未找到用户</CommandEmpty>
        <CommandGroup>
          {props.items.map((item) => (
            <CommandItem
              key={item.id}
              onSelect={() =>
                props.command({ id: String(item.id), label: item.name })
              }
              className="gap-2"
            >
              {item.id !== "all" && (
                <Avatar size="sm">
                  <AvatarImage src={item.avatar} alt={item.name} />
                  <AvatarFallback>{item.name.slice(0, 1)}</AvatarFallback>
                </Avatar>
              )}
              <span>{item.name}</span>
            </CommandItem>
          ))}
        </CommandGroup>
      </CommandList>
    </Command>
  );
}

const ChatComposer = React.forwardRef<ChatComposerHandle, ChatComposerProps>(
  (
    {
      segments,
      onSegmentsChange,
      disabled = false,
      mentionSupport,
      mentionableMembers,
      placeholder,
      className,
      onSubmit,
    },
    ref,
  ) => {
    const onSegmentsChangeRef = useRef(onSegmentsChange);
    const onSubmitRef = useRef(onSubmit);
    const isApplyingExternalUpdateRef = useRef(false);
    const mentionSupportRef = useRef(mentionSupport);
    const mentionCandidatesRef = useRef<MentionableUser[]>([]);

    onSegmentsChangeRef.current = onSegmentsChange;
    onSubmitRef.current = onSubmit;

    const mentionCandidates = useMemo(() => {
      if (mentionSupport === "none") {
        return [];
      }

      if (mentionSupport === "all") {
        return [MENTION_ALL_MEMBER, ...mentionableMembers];
      }

      return mentionableMembers;
    }, [mentionSupport, mentionableMembers]);

    mentionCandidatesRef.current = mentionCandidates;

    const mentionSuggestion: Omit<
      SuggestionOptions<MentionableUser, MentionNodeAttrs>,
      "editor"
    > = {
      allow: () => mentionSupportRef.current !== "none",
      items: ({ query }) => {
        const normalizedQuery = query.trim().toLowerCase();
        if (!normalizedQuery) {
          return mentionCandidatesRef.current;
        }

        return mentionCandidatesRef.current.filter((member) =>
          member.name.toLowerCase().includes(normalizedQuery),
        );
      },

      render: () => {
        let component: ReactRenderer;
        let popup: Instance[];

        return {
          onStart: (props) => {
            component = new ReactRenderer(MentionList, {
              props,
              editor: props.editor,
            });

            if (!props.clientRect) return;

            popup = tippy("body", {
              getReferenceClientRect: props.clientRect as () => DOMRect,
              appendTo: () => document.body,
              content: component.element,
              showOnCreate: true,
              interactive: true,
              trigger: "manual",
              placement: "top-start",
            });
          },

          onUpdate(props) {
            component.updateProps(props);
            if (!props.clientRect) return;
            popup[0].setProps({
              getReferenceClientRect: props.clientRect as () => DOMRect,
            });
          },

          onExit() {
            popup[0].destroy();
            component.destroy();
          },
        };
      },
    };

    const content = useMemo(
      () => messageSegmentsToEditorContent(segments, mentionableMembers),
      [segments, mentionableMembers],
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
        Mention.configure({
          HTMLAttributes: {
            class: "text-sky-600 dark:text-sky-300",
          },
          suggestion: mentionSuggestion,
        }),
      ],
      content,
      immediatelyRender: false,
      editorProps: {
        attributes: {
          class: cn(
            "h-full min-h-0 overflow-y-auto p-2 text-base leading-6 outline-none md:text-sm md:leading-5",
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
            mentionSupportRef.current,
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

      const mentionSupportChanged =
        mentionSupportRef.current !== mentionSupport;
      mentionSupportRef.current = mentionSupport;

      if (
        !mentionSupportChanged &&
        areSegmentsEqual(
          editorContentToSegments(editor.getJSON(), mentionSupport),
          segments,
        )
      ) {
        return;
      }

      isApplyingExternalUpdateRef.current = true;
      editor.commands.setContent(
        messageSegmentsToEditorContent(segments, mentionableMembers),
        {
          emitUpdate: false,
        },
      );
      queueMicrotask(() => {
        isApplyingExternalUpdateRef.current = false;
      });
    }, [mentionSupport, mentionableMembers, segments, editor]);

    useImperativeHandle(
      ref,
      () => ({
        focus() {
          editor?.commands.focus();
        },
        moveCaretToEnd() {
          editor?.commands.focus("end");
        },
        insertFace(faceId) {
          const face = getFaceById(faceId);
          if (!face) {
            return;
          }

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
        insertMention(target) {
          if (mentionSupportRef.current === "none") {
            return;
          }

          if (target === "all") {
            if (mentionSupportRef.current !== "all") {
              return;
            }

            editor
              ?.chain()
              .focus()
              .insertContent([
                {
                  type: "mention",
                  attrs: {
                    id: "all",
                    label: MENTION_ALL_MEMBER.name,
                  },
                },
                { type: "text", text: " " },
              ])
              .run();
            return;
          }

          const targetMember = mentionCandidatesRef.current.find(
            (member): member is MentionableUser & { id: number } =>
              typeof member.id === "number" && member.id === target,
          );
          const label = targetMember?.name ?? String(target);

          editor
            ?.chain()
            .focus()
            .insertContent([
              {
                type: "mention",
                attrs: {
                  id: String(target),
                  label,
                },
              },
              { type: "text", text: " " },
            ])
            .run();
        },
        clear() {
          editor?.commands.clearContent(true);
        },
      }),
      [editor],
    );

    return <EditorContent editor={editor} className={className} />;
  },
);

export default ChatComposer;
