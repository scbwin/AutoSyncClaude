-- Claude Sync Server Database Schema
-- PostgreSQL 15+

-- 启用必要的扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- 用户表
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true
);

-- 设备表
CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_name VARCHAR(255) NOT NULL,
    device_type VARCHAR(50) NOT NULL, -- 'windows', 'linux', 'macos'
    device_fingerprint VARCHAR(255) UNIQUE NOT NULL,
    last_seen TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true,
    UNIQUE(user_id, device_name)
);

-- Token 表
CREATE TABLE access_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    token_hash VARCHAR(255) UNIQUE NOT NULL,
    token_prefix VARCHAR(10) NOT NULL, -- 前8位用于识别
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_used TIMESTAMP WITH TIME ZONE,
    is_revoked BOOLEAN DEFAULT false
);

-- 同步规则配置表
CREATE TABLE sync_rules (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID REFERENCES devices(id) ON DELETE CASCADE, -- NULL 表示全局规则
    rule_name VARCHAR(255) NOT NULL,
    rule_type VARCHAR(20) NOT NULL, -- 'include', 'exclude'
    pattern TEXT NOT NULL, -- glob 模式或正则表达式
    file_type VARCHAR(50), -- 'agent', 'skill', 'plugin', 'command', 'config'
    priority INTEGER DEFAULT 0, -- 优先级，数字越大优先级越高
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id, device_id, rule_name)
);

-- 文件版本表
CREATE TABLE file_versions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL, -- 相对于 .claude 根目录的路径
    file_hash VARCHAR(64) NOT NULL, -- SHA-256
    file_size BIGINT NOT NULL,
    storage_path TEXT NOT NULL, -- 对象存储中的路径
    version_number INTEGER NOT NULL,
    device_id UUID NOT NULL REFERENCES devices(id), -- 创建此版本的设备
    parent_version_id UUID REFERENCES file_versions(id), -- 父版本
    is_deleted BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id, file_path, version_number)
);

-- 文件同步状态表
CREATE TABLE sync_states (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    local_version_id UUID REFERENCES file_versions(id), -- 本地当前版本
    remote_version_id UUID REFERENCES file_versions(id), -- 远程最新版本
    sync_status VARCHAR(20) NOT NULL, -- 'synced', 'pending_upload', 'pending_download', 'conflict', 'error'
    last_sync_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    UNIQUE(user_id, device_id, file_path)
);

-- 冲突记录表
CREATE TABLE conflicts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    base_version_id UUID NOT NULL REFERENCES file_versions(id),
    local_version_id UUID NOT NULL REFERENCES file_versions(id),
    remote_version_id UUID NOT NULL REFERENCES file_versions(id),
    conflict_type VARCHAR(20) NOT NULL, -- 'modify_modify', 'modify_delete', 'binary_conflict'
    conflict_data JSONB, -- 存储冲突详情（文本文件的行级冲突等）
    resolution_status VARCHAR(20) DEFAULT 'unresolved', -- 'unresolved', 'auto_resolved', 'user_resolved', 'ignored'
    resolved_version_id UUID REFERENCES file_versions(id),
    resolved_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 同步会话表（用于跟踪批量同步）
CREATE TABLE sync_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    session_type VARCHAR(20) NOT NULL, -- 'full', 'incremental', 'selective'
    status VARCHAR(20) NOT NULL, -- 'in_progress', 'completed', 'failed', 'cancelled'
    files_processed INTEGER DEFAULT 0,
    files_succeeded INTEGER DEFAULT 0,
    files_failed INTEGER DEFAULT 0,
    files_skipped INTEGER DEFAULT 0,
    conflicts_detected INTEGER DEFAULT 0,
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT
);

-- === 索引优化 ===

-- 用户表索引
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);

-- 设备表索引
CREATE INDEX idx_devices_user_id ON devices(user_id);
CREATE INDEX idx_devices_fingerprint ON devices(device_fingerprint);
CREATE INDEX idx_devices_last_seen ON devices(last_seen);

-- Token 表索引
CREATE INDEX idx_access_tokens_user_device ON access_tokens(user_id, device_id);
CREATE INDEX idx_access_tokens_hash ON access_tokens(token_hash);
CREATE INDEX idx_access_tokens_prefix ON access_tokens(token_prefix);
CREATE INDEX idx_access_tokens_expires_at ON access_tokens(expires_at);

-- 同步规则索引
CREATE INDEX idx_sync_rules_user_id ON sync_rules(user_id);
CREATE INDEX idx_sync_rules_device_id ON sync_rules(device_id);
CREATE INDEX idx_sync_rules_priority ON sync_rules(priority DESC);

-- 文件版本索引
CREATE INDEX idx_file_versions_user_path ON file_versions(user_id, file_path);
CREATE INDEX idx_file_versions_hash ON file_versions(file_hash);
CREATE INDEX idx_file_versions_user_device ON file_versions(user_id, device_id);
CREATE INDEX idx_file_versions_created_at ON file_versions(created_at DESC);

-- 同步状态索引
CREATE INDEX idx_sync_states_device_status ON sync_states(device_id, sync_status);
CREATE INDEX idx_sync_states_user_path ON sync_states(user_id, file_path);
CREATE INDEX idx_sync_states_status ON sync_states(sync_status);

-- 冲突表索引
CREATE INDEX idx_conflicts_user_status ON conflicts(user_id, resolution_status);
CREATE INDEX idx_conflicts_file_path ON conflicts(file_path);
CREATE INDEX idx_conflicts_created_at ON conflicts(created_at DESC);

-- 同步会话索引
CREATE INDEX idx_sync_sessions_user_device ON sync_sessions(user_id, device_id);
CREATE INDEX idx_sync_sessions_status ON sync_sessions(status);
CREATE INDEX idx_sync_sessions_started_at ON sync_sessions(started_at DESC);

-- === 触发器 ===

-- 更新 updated_at 时间戳
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- 清理过期 Token
CREATE OR REPLACE FUNCTION cleanup_expired_tokens()
RETURNS void AS $$
BEGIN
    DELETE FROM access_tokens
    WHERE expires_at < NOW() - INTERVAL '7 days';
END;
$$ LANGUAGE plpgsql;

-- === 初始数据（可选）===

-- 创建默认管理员用户（密码: admin123，需要在生产环境中修改）
-- 密码哈希使用 bcrypt 生成，这里仅作为示例
INSERT INTO users (username, email, password_hash)
VALUES (
    'admin',
    'admin@claude-sync.local',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYzpLaEmc0u'
) ON CONFLICT (email) DO NOTHING;

-- 创建示例同步规则（为管理员用户）
-- 注意：user_id 需要根据实际插入的 ID 调整
INSERT INTO sync_rules (user_id, rule_name, rule_type, pattern, file_type, priority)
SELECT
    id,
    'include-agents',
    'include',
    'agents/**/*',
    'agent',
    100
FROM users
WHERE email = 'admin@claude-sync.local'
ON CONFLICT DO NOTHING;

INSERT INTO sync_rules (user_id, rule_name, rule_type, pattern, file_type, priority)
SELECT
    id,
    'include-skills',
    'include',
    'skills/**/*',
    'skill',
    100
FROM users
WHERE email = 'admin@claude-sync.local'
ON CONFLICT DO NOTHING;

INSERT INTO sync_rules (user_id, rule_name, rule_type, pattern, priority)
SELECT
    id,
    'exclude-cache',
    'exclude',
    'cache/**/*',
    1000
FROM users
WHERE email = 'admin@claude-sync.local'
ON CONFLICT DO NOTHING;

INSERT INTO sync_rules (user_id, rule_name, rule_type, pattern, priority)
SELECT
    id,
    'exclude-downloads',
    'exclude',
    'downloads/**/*',
    1000
FROM users
WHERE email = 'admin@claude-sync.local'
ON CONFLICT DO NOTHING;

INSERT INTO sync_rules (user_id, rule_name, rule_type, pattern, priority)
SELECT
    id,
    'exclude-image-cache',
    'exclude',
    'image-cache/**/*',
    1000
FROM users
WHERE email = 'admin@claude-sync.local'
ON CONFLICT DO NOTHING;

INSERT INTO sync_rules (user_id, rule_name, rule_type, pattern, priority)
SELECT
    id,
    'exclude-file-history',
    'exclude',
    'file-history/**/*',
    1000
FROM users
WHERE email = 'admin@claude-sync.local'
ON CONFLICT DO NOTHING;

INSERT INTO sync_rules (user_id, rule_name, rule_type, pattern, priority)
SELECT
    id,
    'exclude-shell-snapshots',
    'exclude',
    'shell-snapshots/**/*',
    1000
FROM users
WHERE email = 'admin@claude-sync.local'
ON CONFLICT DO NOTHING;

COMMENT ON TABLE users IS '用户表';
COMMENT ON TABLE devices IS '设备表';
COMMENT ON TABLE access_tokens IS '访问令牌表';
COMMENT ON TABLE sync_rules IS '同步规则配置表';
COMMENT ON TABLE file_versions IS '文件版本历史表';
COMMENT ON TABLE sync_states IS '文件同步状态表';
COMMENT ON TABLE conflicts IS '冲突记录表';
COMMENT ON TABLE sync_sessions IS '同步会话表';

COMMENT ON COLUMN users.password_hash IS 'bcrypt 哈希后的密码';
COMMENT ON COLUMN devices.device_fingerprint IS '设备唯一指纹（SHA-256 哈希）';
COMMENT ON COLUMN access_tokens.token_prefix IS 'Token 前缀（前 8 位），用于识别和显示';
COMMENT ON COLUMN sync_rules.rule_type IS '规则类型：include（包含）或 exclude（排除）';
COMMENT ON COLUMN sync_rules.priority IS '优先级，数字越大优先级越高';
COMMENT ON COLUMN file_versions.storage_path IS '在对象存储中的实际路径';
COMMENT ON COLUMN sync_states.sync_status IS '同步状态：synced, pending_upload, pending_download, conflict, error';
COMMENT ON COLUMN conflicts.resolution_status IS '冲突解决状态：unresolved, auto_resolved, user_resolved, ignored';
