-- ============================================================================
-- Migration 0003: 群文件与群相册本地路径支持
-- ============================================================================

-- 1. 群文件增加本地 file_path（相对 APPDATA/groups/ 的路径）
ALTER TABLE group_files ADD COLUMN file_path TEXT;

-- 2. 群照片增加本地 file_path
ALTER TABLE group_photos ADD COLUMN file_path TEXT;

-- 3. 更新 app_settings schema 版本
UPDATE app_settings
SET setting_value = '0003',
    updated_at = (unixepoch() * 1000)
WHERE setting_key = 'schema.version';
