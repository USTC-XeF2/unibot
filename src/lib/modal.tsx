import NiceModal, { useModal } from "@ebay/nice-modal-react";
import { useMemo, useState } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";

type DialogProps = {
  title: string;
  description: string;
  confirmText: string;
};

const ConfirmDialogModal = NiceModal.create((props: DialogProps) => {
  const modal = useModal();
  const { title, description, confirmText } = props;

  const closeWith = async (result: boolean) => {
    modal.resolve(result);
    await modal.hide();
  };

  return (
    <AlertDialog
      open={modal.visible}
      onOpenChange={(open) => {
        if (!open) {
          void closeWith(false);
        }
      }}
    >
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel onClick={() => void closeWith(false)}>
            取消
          </AlertDialogCancel>
          <AlertDialogAction
            variant="destructive"
            onClick={() => void closeWith(true)}
          >
            {confirmText}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
});

const CONFIRM_DIALOG_MODAL_ID = "app-confirm-dialog";
NiceModal.register(CONFIRM_DIALOG_MODAL_ID, ConfirmDialogModal);

const PromptDialogModal = NiceModal.create((props: DialogProps) => {
  const modal = useModal();
  const { title, description, confirmText } = props;
  const [value, setValue] = useState("");

  const closeWith = async (result: string | null) => {
    setValue("");
    modal.resolve(result);
    await modal.hide();
  };

  const trimmedValue = useMemo(() => value.trim(), [value]);

  return (
    <Dialog
      open={modal.visible}
      onOpenChange={(open) => {
        if (!open) {
          closeWith(null);
        }
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {description ? (
            <DialogDescription>{description}</DialogDescription>
          ) : null}
        </DialogHeader>

        <Input
          value={value}
          onChange={(event) => setValue(event.target.value)}
          autoFocus
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              closeWith(trimmedValue);
            }
          }}
        />

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => closeWith(null)}
          >
            取消
          </Button>
          <Button type="button" onClick={() => closeWith(trimmedValue)}>
            {confirmText}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
});

const PROMPT_DIALOG_MODAL_ID = "app-prompt-dialog";
NiceModal.register(PROMPT_DIALOG_MODAL_ID, PromptDialogModal);

export function confirmDialog(props: DialogProps): Promise<boolean> {
  return NiceModal.show<boolean>(CONFIRM_DIALOG_MODAL_ID, props);
}

export function promptDialog(props: DialogProps): Promise<string | null> {
  return NiceModal.show<string | null>(PROMPT_DIALOG_MODAL_ID, props);
}
