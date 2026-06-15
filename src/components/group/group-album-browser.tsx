import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ArrowLeft,
  Image,
  Plus,
  RefreshCw,
  Trash2,
  Upload,
} from "lucide-react";
import { useState } from "react";
import { GroupContentError } from "@/components/group/group-content-error";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { confirmDialog } from "@/lib/modal";
import {
  useCreateGroupAlbumMutation,
  useDeleteGroupAlbumMutation,
  useDeleteGroupPhotoMutation,
  useUploadGroupPhotoMutation,
} from "@/lib/mutations";
import { useGroupAlbumsQuery, useGroupPhotosQuery } from "@/lib/query";
import type { GroupAlbum, GroupPhoto } from "@/types/group";

export default function GroupAlbumBrowser({
  userId,
  groupId,
}: {
  userId: string;
  groupId: string;
}) {
  const [selectedAlbumId, setSelectedAlbumId] = useState<string | null>(null);

  if (selectedAlbumId) {
    return (
      <PhotoGrid
        userId={userId}
        groupId={groupId}
        albumId={selectedAlbumId}
        onBack={() => setSelectedAlbumId(null)}
      />
    );
  }

  return (
    <AlbumGrid
      userId={userId}
      groupId={groupId}
      onSelectAlbum={setSelectedAlbumId}
    />
  );
}

function AlbumGrid({
  userId,
  groupId,
  onSelectAlbum,
}: {
  userId: string;
  groupId: string;
  onSelectAlbum: (albumId: string) => void;
}) {
  const {
    data: albums = [],
    isError: albumsError,
    error: albumsErrorValue,
    refetch,
  } = useGroupAlbumsQuery(userId, groupId);
  const createAlbumMutation = useCreateGroupAlbumMutation();
  const deleteAlbumMutation = useDeleteGroupAlbumMutation();

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b p-3">
        <h1 className="font-semibold text-lg">群相册 · {groupId}</h1>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() =>
              createAlbumMutation.mutate({ userId, groupId, name: "新建相册" })
            }
          >
            <Plus className="mr-1 size-4" />
            新建相册
          </Button>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="mr-1 size-4" />
            刷新
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-auto p-4">
        {albumsError ? (
          <GroupContentError
            message={`加载相册失败：${albumsErrorValue}`}
            onRetry={() => refetch()}
          />
        ) : (
          <div className="grid grid-cols-3 gap-4">
            {albums.map((album) => (
              <AlbumCard
                key={album.album_id}
                album={album}
                onClick={() => onSelectAlbum(album.album_id)}
                onDelete={async () => {
                  const confirmed = await confirmDialog({
                    title: "确认删除相册",
                    description: `确定要删除相册 "${album.name}" 吗？其中的 ${album.photo_count} 张照片也会被删除，此操作不可恢复。`,
                    confirmText: "删除",
                  });
                  if (!confirmed) return;

                  deleteAlbumMutation.mutate({
                    userId,
                    groupId,
                    albumId: album.album_id,
                  });
                }}
              />
            ))}
          </div>
        )}
      </div>

      <div className="border-t bg-muted/30 px-4 py-2 text-muted-foreground text-xs">
        已用 0 MB / 10 GB
      </div>
    </div>
  );
}

function AlbumCard({
  album,
  onClick,
  onDelete,
}: {
  album: GroupAlbum;
  onClick: () => void;
  onDelete: () => Promise<void> | void;
}) {
  return (
    <div className="group relative overflow-hidden rounded-xl border hover:shadow-sm">
      <button
        type="button"
        className="block w-full text-left"
        onClick={onClick}
      >
        <div className="flex aspect-square items-center justify-center bg-muted">
          {album.cover_url ? (
            <img
              src={convertFileSrc(album.cover_url)}
              alt={album.name}
              className="size-full object-cover"
            />
          ) : (
            <Image className="size-12 text-muted-foreground" />
          )}
        </div>
        <div className="p-3">
          <div className="font-medium">{album.name}</div>
          <div className="text-muted-foreground text-xs">
            {album.photo_count} 张
          </div>
        </div>
      </button>
      <Button
        variant="ghost"
        size="icon-sm"
        className="absolute top-2 right-2 opacity-0 group-hover:opacity-100"
        onClick={(e) => {
          e.stopPropagation();
          onDelete();
        }}
      >
        <Trash2 className="size-4 text-destructive" />
      </Button>
    </div>
  );
}

function PhotoGrid({
  userId,
  groupId,
  albumId,
  onBack,
}: {
  userId: string;
  groupId: string;
  albumId: string;
  onBack: () => void;
}) {
  const {
    data: photos = [],
    isError: photosError,
    error: photosErrorValue,
    refetch,
  } = useGroupPhotosQuery(userId, groupId, albumId);
  const uploadMutation = useUploadGroupPhotoMutation();
  const deletePhotoMutation = useDeleteGroupPhotoMutation();

  const handleUpload = async () => {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [
        {
          name: "图片",
          extensions: ["jpg", "jpeg", "png", "gif", "webp", "bmp"],
        },
      ],
    });
    if (!selected || Array.isArray(selected)) return;

    try {
      await uploadMutation.mutateAsync({
        userId,
        groupId,
        albumId,
        sourcePath: selected,
      });
    } catch {
      // Toast is owned by the mutation's onError handler.
    }
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b p-3">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="icon-sm" onClick={onBack}>
            <ArrowLeft className="size-4" />
          </Button>
          <h1 className="font-semibold text-lg">相册</h1>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={handleUpload}>
            <Upload className="mr-1 size-4" />
            上传照片
          </Button>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="mr-1 size-4" />
            刷新
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-auto p-4">
        {photosError ? (
          <GroupContentError
            message={`加载照片失败：${photosErrorValue}`}
            onRetry={() => refetch()}
          />
        ) : (
          <div className="grid grid-cols-4 gap-2">
            {photos.map((photo) => (
              <PhotoItem
                key={photo.photo_id}
                photo={photo}
                onDelete={async () => {
                  const confirmed = await confirmDialog({
                    title: "确认删除照片",
                    description: "确定要删除这张照片吗？此操作不可恢复。",
                    confirmText: "删除",
                  });
                  if (!confirmed) return;

                  deletePhotoMutation.mutate({
                    userId,
                    groupId,
                    albumId,
                    photoId: photo.photo_id,
                  });
                }}
              />
            ))}
          </div>
        )}
      </div>

      <div className="border-t bg-muted/30 px-4 py-2 text-muted-foreground text-xs">
        {photos.length} 张照片
      </div>
    </div>
  );
}

function PhotoItem({
  photo,
  onDelete,
}: {
  photo: GroupPhoto;
  onDelete: () => Promise<void> | void;
}) {
  const src = photo.url ? convertFileSrc(photo.url) : "";

  return (
    <Dialog>
      <DialogTrigger asChild>
        <div className="group relative aspect-square overflow-hidden rounded-lg bg-muted">
          {src ? (
            <img
              src={src}
              alt={photo.description || photo.photo_id}
              className="size-full cursor-pointer object-cover"
            />
          ) : (
            <div className="flex size-full items-center justify-center">
              <Image className="size-8 text-muted-foreground" />
            </div>
          )}
          <Button
            variant="ghost"
            size="icon-sm"
            className="absolute top-1 right-1 opacity-0 group-hover:opacity-100"
            onClick={(e) => {
              e.stopPropagation();
              onDelete();
            }}
          >
            <Trash2 className="size-4 text-destructive" />
          </Button>
        </div>
      </DialogTrigger>
      <DialogContent className="max-w-3xl p-1">
        <DialogTitle className="sr-only">
          {photo.description || "照片"}
        </DialogTitle>
        {src ? (
          <img
            src={src}
            alt={photo.description || photo.photo_id}
            className="max-h-[80vh] w-auto rounded-md object-contain"
          />
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
