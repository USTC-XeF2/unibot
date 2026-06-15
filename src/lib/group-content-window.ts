import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { COMMANDS } from "@/lib/commands";

/**
 * 打开独立的群文件窗口。后端会校验当前用户仍是群成员；失败时
 * （无权限、窗口创建失败等）以 toast 提示，而不是静默吞掉错误。
 */
export async function openGroupFilesWindow(userId: string, groupId: string) {
  try {
    await invoke(COMMANDS.openGroupFilesWindow, { userId, groupId });
  } catch (error) {
    toast.error(`打开群文件失败：${error}`);
  }
}

/** 打开独立的群相册窗口，错误处理同 {@link openGroupFilesWindow}。 */
export async function openGroupAlbumsWindow(userId: string, groupId: string) {
  try {
    await invoke(COMMANDS.openGroupAlbumsWindow, { userId, groupId });
  } catch (error) {
    toast.error(`打开群相册失败：${error}`);
  }
}
