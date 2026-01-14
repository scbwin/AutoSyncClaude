// App state
const state = {
    currentView: 'dashboard',
    isLoggedIn: false,
    user: null,
    config: null,
    syncStatus: null,
    rules: [],
    devices: [],
};

// Initialize app
async function init() {
    await loadConfig();
    await checkAuthStatus();
    setupEventListeners();
    setupNavigation();
    updateUI();
}

// Load configuration
async function loadConfig() {
    try {
        const config = await window.__TAURI__.invoke('get_config');
        state.config = config;
        console.log('Config loaded:', config);
    } catch (error) {
        console.error('Failed to load config:', error);
    }
}

// Check authentication status
async function checkAuthStatus() {
    try {
        const status = await window.__TAURI__.invoke('get_status');
        state.isLoggedIn = status.logged_in;
        state.user = {
            id: status.user_id,
            deviceId: status.device_id,
        };
        updateAuthUI();
    } catch (error) {
        console.error('Failed to check auth status:', error);
    }
}

// Setup event listeners
function setupEventListeners() {
    // Login dialog
    document.getElementById('closeLoginDialog').addEventListener('click', () => {
        document.getElementById('loginDialog').classList.remove('active');
    });

    document.getElementById('loginForm').addEventListener('submit', async (e) => {
        e.preventDefault();
        await handleLogin();
    });

    // Rule dialog
    document.getElementById('closeRuleDialog').addEventListener('click', () => {
        document.getElementById('ruleDialog').classList.remove('active');
    });

    document.getElementById('ruleForm').addEventListener('submit', async (e) => {
        e.preventDefault();
        await handleAddRule();
    });

    document.getElementById('addRuleBtn').addEventListener('click', () => {
        document.getElementById('ruleDialog').classList.add('active');
    });

    // User info click
    document.getElementById('userInfo').addEventListener('click', () => {
        if (!state.isLoggedIn) {
            document.getElementById('loginDialog').classList.add('active');
        }
    });

    // Sync controls
    document.getElementById('startSyncBtn').addEventListener('click', async () => {
        await handleStartSync();
    });

    document.getElementById('stopSyncBtn').addEventListener('click', async () => {
        await handleStopSync();
    });

    // Settings
    document.getElementById('saveSettingsBtn').addEventListener('click', async () => {
        await handleSaveSettings();
    });

    document.getElementById('resetSettingsBtn').addEventListener('click', async () => {
        await handleResetSettings();
    });
}

// Setup navigation
function setupNavigation() {
    const navItems = document.querySelectorAll('.nav-item');
    navItems.forEach(item => {
        item.addEventListener('click', (e) => {
            e.preventDefault();
            const view = item.dataset.view;
            switchView(view);
        });
    });
}

// Switch view
function switchView(viewName) {
    // Update nav
    document.querySelectorAll('.nav-item').forEach(item => {
        item.classList.toggle('active', item.dataset.view === viewName);
    });

    // Update views
    document.querySelectorAll('.view').forEach(view => {
        view.classList.toggle('active', view.id === `${viewName}-view`);
    });

    state.currentView = viewName;

    // Load view-specific data
    switch (viewName) {
        case 'dashboard':
            loadDashboardData();
            break;
        case 'sync':
            loadSyncStatus();
            break;
        case 'rules':
            loadRules();
            break;
        case 'devices':
            loadDevices();
            break;
        case 'settings':
            loadSettings();
            break;
    }
}

// Update UI
function updateUI() {
    updateAuthUI();
    loadDashboardData();
}

// Update auth UI
function updateAuthUI() {
    const userInfo = document.getElementById('userInfo');
    if (state.isLoggedIn) {
        userInfo.querySelector('.user-avatar').textContent = state.user?.id?.[0]?.toUpperCase() || '?';
        userInfo.querySelector('.user-name').textContent = '已登录';
        userInfo.querySelector('.user-email').textContent = state.user?.id || '';
    } else {
        userInfo.querySelector('.user-avatar').textContent = '?';
        userInfo.querySelector('.user-name').textContent = '未登录';
        userInfo.querySelector('.user-email').textContent = '点击登录';
    }
}

// Handle login
async function handleLogin() {
    const email = document.getElementById('loginEmail').value;
    const password = document.getElementById('loginPassword').value;
    const deviceName = document.getElementById('deviceName').value;

    try {
        const result = await window.__TAURI__.invoke('login', {
            email,
            password,
            deviceName: deviceName || null,
        });

        console.log('Login result:', result);
        state.isLoggedIn = true;
        state.user = {
            id: result.user_id,
            deviceId: result.device_id,
        };

        document.getElementById('loginDialog').classList.remove('active');
        updateAuthUI();

        // Show success notification
        showNotification('登录成功', 'success');
    } catch (error) {
        console.error('Login failed:', error);
        showNotification('登录失败: ' + error, 'error');
    }
}

// Handle start sync
async function handleStartSync() {
    const mode = document.getElementById('syncMode').value;

    try {
        const result = await window.__TAURI__.invoke('start_sync', { mode });
        console.log('Start sync result:', result);

        document.getElementById('startSyncBtn').disabled = true;
        document.getElementById('stopSyncBtn').disabled = false;
        document.getElementById('syncProgress').style.display = 'block';

        // Start polling sync status
        pollSyncStatus();

        showNotification(result, 'success');
    } catch (error) {
        console.error('Start sync failed:', error);
        showNotification('启动同步失败: ' + error, 'error');
    }
}

// Handle stop sync
async function handleStopSync() {
    try {
        await window.__TAURI__.invoke('stop_sync');
        console.log('Sync stopped');

        document.getElementById('startSyncBtn').disabled = false;
        document.getElementById('stopSyncBtn').disabled = true;
        document.getElementById('syncProgress').style.display = 'none';

        showNotification('同步已停止', 'success');
    } catch (error) {
        console.error('Stop sync failed:', error);
        showNotification('停止同步失败: ' + error, 'error');
    }
}

// Poll sync status
async function pollSyncStatus() {
    const interval = setInterval(async () => {
        try {
            const status = await window.__TAURI__.invoke('get_sync_status');
            state.syncStatus = status;

            // Update progress
            if (status.is_syncing) {
                document.getElementById('syncStatusText').textContent = '同步中...';
                document.getElementById('syncPercentage').textContent = Math.round(status.progress) + '%';
                document.getElementById('progressBar').style.width = status.progress + '%';
            } else {
                clearInterval(interval);
                document.getElementById('startSyncBtn').disabled = false;
                document.getElementById('stopSyncBtn').disabled = true;
                document.getElementById('syncProgress').style.display = 'none';
            }

            // Update dashboard stats
            document.getElementById('syncedCount').textContent = status.synced_files;
            document.getElementById('failedCount').textContent = status.failed_files;
        } catch (error) {
            console.error('Failed to poll sync status:', error);
            clearInterval(interval);
        }
    }, 1000);
}

// Load sync status
async function loadSyncStatus() {
    try {
        const status = await window.__TAURI__.invoke('get_sync_status');
        state.syncStatus = status;

        document.getElementById('startSyncBtn').disabled = status.is_syncing;
        document.getElementById('stopSyncBtn').disabled = !status.is_syncing;

        if (status.is_syncing) {
            document.getElementById('syncProgress').style.display = 'block';
            document.getElementById('syncPercentage').textContent = Math.round(status.progress) + '%';
            document.getElementById('progressBar').style.width = status.progress + '%';
            pollSyncStatus();
        }
    } catch (error) {
        console.error('Failed to load sync status:', error);
    }
}

// Load dashboard data
async function loadDashboardData() {
    try {
        const status = await window.__TAURI__.invoke('get_sync_status');
        document.getElementById('syncedCount').textContent = status.synced_files;
        document.getElementById('failedCount').textContent = status.failed_files;

        if (status.last_sync) {
            const lastSync = new Date(status.last_sync);
            document.getElementById('lastSync').textContent = formatTime(lastSync);
        }

        // Load rules count
        const rules = await window.__TAURI__.invoke('list_rules');
        state.rules = rules;
        document.getElementById('rulesCount').textContent = rules.length;
    } catch (error) {
        console.error('Failed to load dashboard data:', error);
    }
}

// Load rules
async function loadRules() {
    try {
        const rules = await window.__TAURI__.invoke('list_rules');
        state.rules = rules;
        renderRules();
    } catch (error) {
        console.error('Failed to load rules:', error);
    }
}

// Render rules
function renderRules() {
    const container = document.getElementById('rulesList');

    if (state.rules.length === 0) {
        container.innerHTML = '<div class="empty-state">暂无规则</div>';
        return;
    }

    container.innerHTML = state.rules.map(rule => `
        <div class="rule-item">
            <div class="rule-info">
                <div class="rule-name">${escapeHtml(rule.name)}</div>
                <div class="rule-pattern">${escapeHtml(rule.pattern)}</div>
                <div class="rule-meta">
                    <span>类型: ${rule.type === 'exclude' ? '排除' : '包含'}</span>
                    <span>优先级: ${rule.priority}</span>
                    <span>${rule.enabled ? '已启用' : '已禁用'}</span>
                </div>
            </div>
            <div class="rule-actions">
                <button class="btn-icon" onclick="handleRemoveRule('${rule.id}')" title="删除">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                        <polyline points="3 6 5 6 21 6"></polyline>
                        <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                    </svg>
                </button>
            </div>
        </div>
    `).join('');
}

// Handle add rule
async function handleAddRule() {
    const name = document.getElementById('ruleName').value;
    const ruleType = document.getElementById('ruleType').value;
    const pattern = document.getElementById('rulePattern').value;
    const fileType = document.getElementById('ruleFileType').value || null;
    const priority = parseInt(document.getElementById('rulePriority').value);

    try {
        await window.__TAURI__.invoke('add_rule', {
            name,
            ruleType,
            pattern,
            fileType,
            priority,
        });

        document.getElementById('ruleDialog').classList.remove('active');
        document.getElementById('ruleForm').reset();

        await loadRules();
        showNotification('规则添加成功', 'success');
    } catch (error) {
        console.error('Failed to add rule:', error);
        showNotification('添加规则失败: ' + error, 'error');
    }
}

// Handle remove rule
async function handleRemoveRule(ruleId) {
    if (!confirm('确定要删除此规则吗？')) return;

    try {
        await window.__TAURI__.invoke('remove_rule', { ruleId });
        await loadRules();
        showNotification('规则删除成功', 'success');
    } catch (error) {
        console.error('Failed to remove rule:', error);
        showNotification('删除规则失败: ' + error, 'error');
    }
}

// Load devices
async function loadDevices() {
    try {
        const result = await window.__TAURI__.invoke('list_devices');
        state.devices = result.devices || [];
        renderDevices();
    } catch (error) {
        console.error('Failed to load devices:', error);
    }
}

// Render devices
function renderDevices() {
    const container = document.getElementById('devicesList');

    if (state.devices.length === 0) {
        container.innerHTML = '<div class="empty-state">暂无设备</div>';
        return;
    }

    container.innerHTML = state.devices.map(device => `
        <div class="device-item">
            <div class="device-info">
                <div class="device-icon">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                        <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
                        <line x1="8" y1="21" x2="16" y2="21"></line>
                        <line x1="12" y1="17" x2="12" y2="21"></line>
                    </svg>
                </div>
                <div>
                    <div class="device-name">${escapeHtml(device.name)}</div>
                    <div class="device-last-seen">最后上线: ${device.last_seen ? formatTime(new Date(device.last_seen)) : '未知'}</div>
                </div>
            </div>
        </div>
    `).join('');
}

// Load settings
function loadSettings() {
    if (!state.config) return;

    const server = state.config.server || {};
    const sync = state.config.sync || {};
    const ui = state.config.ui || {};

    document.getElementById('serverAddress').value = server.address || 'http://localhost:50051';
    document.getElementById('serverTimeout').value = server.timeout || 30;
    document.getElementById('claudeDir').value = sync.claude_dir || '';
    document.getElementById('syncInterval').value = sync.interval || 60;
    document.getElementById('autoStart').checked = sync.auto_start || false;
    document.getElementById('theme').value = ui.theme || 'system';
    document.getElementById('language').value = ui.language || 'zh-CN';
    document.getElementById('minimizeToTray').checked = ui.minimize_to_tray !== false;
    document.getElementById('showNotifications').checked = ui.show_notifications !== false;
}

// Handle save settings
async function handleSaveSettings() {
    const newConfig = {
        server: {
            address: document.getElementById('serverAddress').value,
            timeout: parseInt(document.getElementById('serverTimeout').value),
        },
        sync: {
            claude_dir: document.getElementById('claudeDir').value,
            interval: parseInt(document.getElementById('syncInterval').value),
            auto_start: document.getElementById('autoStart').checked,
            exclude_patterns: state.config?.sync?.exclude_patterns || [],
        },
        ui: {
            theme: document.getElementById('theme').value,
            language: document.getElementById('language').value,
            minimize_to_tray: document.getElementById('minimizeToTray').checked,
            show_notifications: document.getElementById('showNotifications').checked,
        },
    };

    try {
        await window.__TAURI__.invoke('update_config', { config: newConfig });
        state.config = newConfig;
        showNotification('设置保存成功', 'success');
    } catch (error) {
        console.error('Failed to save settings:', error);
        showNotification('保存设置失败: ' + error, 'error');
    }
}

// Handle reset settings
async function handleResetSettings() {
    if (!confirm('确定要重置为默认设置吗？')) return;

    try {
        const config = await window.__TAURI__.invoke('init_config');
        state.config = config;
        loadSettings();
        showNotification('设置已重置', 'success');
    } catch (error) {
        console.error('Failed to reset settings:', error);
        showNotification('重置设置失败: ' + error, 'error');
    }
}

// Show notification
function showNotification(message, type = 'info') {
    // Simple alert for now, can be enhanced with custom toast notifications
    console.log(`[${type.toUpperCase()}] ${message}`);
    alert(message);
}

// Format time
function formatTime(date) {
    const now = new Date();
    const diff = now - date;

    if (diff < 60000) return '刚刚';
    if (diff < 3600000) return Math.floor(diff / 60000) + '分钟前';
    if (diff < 86400000) return Math.floor(diff / 3600000) + '小时前';
    if (diff < 604800000) return Math.floor(diff / 86400000) + '天前';

    return date.toLocaleDateString('zh-CN');
}

// Escape HTML
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Make handleRemoveRule available globally
window.handleRemoveRule = handleRemoveRule;

// Initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
