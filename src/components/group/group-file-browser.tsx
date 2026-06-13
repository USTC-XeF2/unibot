import { open } from "@tauri-apps/plugin-dialog";
import {
  Download,
  File,
  Folder,
  MoreHorizontal,
  Plus,
  RefreshCw,
  Upload,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  useDeleteGroupFileMutation,
  useDownloadGroupFileMutation,
  useUploadGroupFileMutation,
  useUpsertGroupFolderMutation,
} from "@/lib/mutations";
import { useGroupFilesQuery, useGroupFoldersQuery } from "@/lib/query";
import type { GroupFile, GroupFolder } from "@/types/group";

export default function GroupFileBrowser({
  userId,
  groupId,
}: {
  userId: string;
  groupId: string;
}) {
  const [parentFolderId, setParentFolderId] = useState<string | undefined>(
    undefined,
  );
  const [folderStack, setFolderStack] = useState<GroupFolder[]>([]);

  const { data: files = [], refetch: refetchFiles } = useGroupFilesQuery(
    userId,
    groupId,
    parentFolderId,
  );
  const { data: folders = [], refetch: refetchFolders } = useGroupFoldersQuery(
    userId,
    groupId,
  );

  const uploadMutation = useUploadGroupFileMutation();
  const downloadMutation = useDownloadGroupFileMutation();
  const deleteFileMutation = useDeleteGroupFileMutation();
  const createFolderMutation = useUpsertGroupFolderMutation();

  const currentFolders = folders.filter(
    (f) => f.parent_folder_id === (parentFolderId ?? null),
  );

  const handleEnterFolder = (folder: GroupFolder) => {
    setFolderStack((prev) => [...prev, folder]);
    setParentFolderId(folder.folder_id);
  };

  const handleGoBack = () => {
    if (folderStack.length === 0) return;
    const newStack = folderStack.slice(0, -1);
    setFolderStack(newStack);
    setParentFolderId(
      newStack.length > 0 ? newStack[newStack.length - 1].folder_id : undefined,
    );
  };

  const handleGoRoot = () => {
    setFolderStack([]);
    setParentFolderId(undefined);
  };

  const handleUpload = async () => {
    const selected = await open({
      multiple: false,
      directory: false,
    });
    if (!selected || Array.isArray(selected)) return;

    const fileName =
      selected.split("/").pop() || selected.split("\\").pop() || "upload";
    await uploadMutation.mutateAsync({
      userId,
      groupId,
      parentFolderId,
      fileName,
      sourcePath: selected,
    });
  };

  const handleDownload = async (file: GroupFile) => {
    const path = await downloadMutation.mutateAsync({
      userId,
      groupId,
      fileId: file.file_id,
    });
    toast.success(`文件已下载: ${path}`);
  };

  const handleCreateFolder = () => {
    createFolderMutation.mutate({
      userId,
      groupId,
      parentFolderId,
      folderName: "新建文件夹",
    });
  };

  return (
    <div className="flex h-full flex-col">
      {/* 顶部工具栏 */}
      <div className="flex items-center justify-between border-b p-3">
        <div className="flex gap-1 rounded-lg bg-muted p-1">
          <Button variant="default" size="sm">
            文件
          </Button>
          <Button variant="ghost" size="sm" disabled>
            回收站
          </Button>
        </div>
        <div className="flex gap-2">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm">
                <Plus className="mr-1 size-4" />
                新建
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <DropdownMenuItem onClick={handleCreateFolder}>
                新建文件夹
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          <Button variant="outline" size="sm" onClick={handleUpload}>
            <Upload className="mr-1 size-4" />
            上传
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              refetchFiles();
              refetchFolders();
            }}
          >
            <RefreshCw className="mr-1 size-4" />
            刷新
          </Button>
        </div>
      </div>

      {/* 面包屑 */}
      <div className="flex items-center justify-between border-b px-4 py-2 text-sm">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Button
            variant="link"
            size="sm"
            className="h-auto p-0"
            onClick={handleGoBack}
            disabled={folderStack.length === 0}
          >
            返回上一级
          </Button>
          <span>|</span>
          <Button
            variant="link"
            size="sm"
            className="h-auto p-0"
            onClick={handleGoRoot}
          >
            全部
          </Button>
          {folderStack.map((folder) => (
            <span key={folder.folder_id} className="flex items-center gap-2">
              <span>/</span>
              <span>{folder.folder_name}</span>
            </span>
          ))}
        </div>
        <span className="text-muted-foreground text-xs">
          共 {currentFolders.length + files.length} 条
        </span>
      </div>

      {/* 列表 */}
      <div className="flex-1 overflow-auto">
        <table className="w-full text-sm">
          <thead className="bg-muted/50 text-muted-foreground">
            <tr>
              <th className="w-10 px-4 py-2 text-left">
                <input type="checkbox" disabled />
              </th>
              <th className="px-4 py-2 text-left">名称</th>
              <th className="px-4 py-2 text-left">大小</th>
              <th className="px-4 py-2 text-left">修改人</th>
              <th className="px-4 py-2 text-left">修改时间</th>
              <th className="px-4 py-2 text-left">操作</th>
            </tr>
          </thead>
          <tbody>
            {currentFolders.map((folder) => (
              <tr
                key={folder.folder_id}
                className="cursor-pointer border-b hover:bg-muted/50"
                onClick={() => handleEnterFolder(folder)}
              >
                <td className="px-4 py-3">
                  <input type="checkbox" disabled />
                </td>
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    <Folder className="size-5 text-yellow-500" />
                    {folder.folder_name}
                  </div>
                </td>
                <td className="px-4 py-3 text-muted-foreground">-</td>
                <td className="px-4 py-3">{folder.creator_user_id}</td>
                <td className="px-4 py-3 text-muted-foreground">
                  {new Date(folder.updated_at).toLocaleString()}
                </td>
                <td className="px-4 py-3 text-primary">进入</td>
              </tr>
            ))}
            {files.map((file) => (
              <GroupFileRow
                key={file.file_id}
                file={file}
                onDownload={() => handleDownload(file)}
                onDelete={() =>
                  deleteFileMutation.mutate({
                    userId,
                    groupId,
                    fileId: file.file_id,
                    parentFolderId: parentFolderId ?? "",
                  })
                }
              />
            ))}
          </tbody>
        </table>
      </div>

      {/* 底部状态栏 */}
      <div className="border-t bg-muted/30 px-4 py-2 text-muted-foreground text-xs">
        已用 0 MB / 10 GB
      </div>
    </div>
  );
}

function GroupFileRow({
  file,
  onDownload,
  onDelete,
}: {
  file: GroupFile;
  onDownload: () => void;
  onDelete: () => void;
}) {
  return (
    <tr className="border-b hover:bg-muted/50">
      <td className="px-4 py-3">
        <input type="checkbox" disabled />
      </td>
      <td className="px-4 py-3">
        <div className="flex items-center gap-2">
          <File className="size-5 text-muted-foreground" />
          {file.file_name}
        </div>
      </td>
      <td className="px-4 py-3 text-muted-foreground">
        {formatBytes(file.file_size)}
      </td>
      <td className="px-4 py-3">{file.uploader_user_id}</td>
      <td className="px-4 py-3 text-muted-foreground">
        {new Date(file.uploaded_at).toLocaleString()}
      </td>
      <td className="px-4 py-3">
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="icon-sm" onClick={onDownload}>
            <Download className="size-4" />
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon-sm">
                <MoreHorizontal className="size-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <DropdownMenuItem onClick={onDelete}>删除</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </td>
    </tr>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / k ** i).toFixed(2))} ${sizes[i]}`;
}
