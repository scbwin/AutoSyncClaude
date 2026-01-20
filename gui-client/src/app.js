// App state
const state = {
    currentView: 'dashboard',
    isLoggedIn: false,
    user: null,
    config: null,
    syncStatus: null,
    rules: [],
    pendingFiles: [],
    devices: [],
    connectionStatus: {
        connected: false,
        message: '未连接',
        checking: false
    },
    connectionCheckInterval: null,
    username: null,  // 用户名用于显示
    // 文件树相关状态
    fileTree: null,
    expandedFolders: new Set(['root']),
    selectedForSync: new Set(),
    contextMenuTarget: null,
    customIgnorePatterns: [],
    // 日志系统
    logs: [],
    maxLogs: 1000,
    logLevelFilter: 'all',
    autoScrollLogs: true,
};

// ========== 日志系统 ==========

const LogLevel = {
    DEBUG: 'DEBUG',
    INFO: 'INFO',
    WARN: 'WARN',
    ERROR: 'ERROR'
};

// 初始化日志系统
function initLogSystem() {
    // 拦截 console.log
    const originalLog = console.log;
    const originalWarn = console.warn;
    const originalError = console.error;
    const originalDebug = console.debug;

    console.log = function(...args) {
        originalLog.apply(console, args);
        addLog(LogLevel.INFO, formatLogMessage(args));
    };

    console.warn = function(...args) {
        originalWarn.apply(console, args);
        addLog(LogLevel.WARN, formatLogMessage(args));
    };

    console.error = function(...args) {
        originalError.apply(console, args);
        addLog(LogLevel.ERROR, formatLogMessage(args));
    };

    console.debug = function(...args) {
        originalDebug.apply(console, args);
        addLog(LogLevel.DEBUG, formatLogMessage(args));
    };

    // 捕获未处理的错误
    window.addEventListener('error', (event) => {
        addLog(LogLevel.ERROR, `未捕获的错误: ${event.message} (${event.filename}:${event.lineno})`);
    });

    window.addEventListener('unhandledrejection', (event) => {
        addLog(LogLevel.ERROR, `未处理的 Promise 拒绝: ${event.reason}`);
    });

    addLog(LogLevel.INFO, '日志系统已初始化');
}

// 格式化日志消息
function formatLogMessage(args) {
    return args.map(arg => {
        if (typeof arg === 'string') return arg;
        if (arg instanceof Error) return arg.stack || arg.message;
        try {
            return JSON.stringify(arg, null, 2);
        } catch {
            return String(arg);
        }
    }).join(' ');
}

// 添加日志
function addLog(level, message) {
    const timestamp = new Date();
    const log = {
        timestamp,
        level,
        message,
        id: Date.now() + Math.random()
    };

    state.logs.push(log);

    // 限制日志数量
    if (state.logs.length > state.maxLogs) {
        state.logs.shift();
    }

    // 更新 UI（如果日志视图可见）
    const logsContent = document.getElementById('logsContent');
    if (logsContent) {
        appendLogToUI(log);
    }

    return log;
}

// 添加日志到 UI
function appendLogToUI(log) {
    const logsContent = document.getElementById('logsContent');
    if (!logsContent) return;

    // 应用级别过滤
    if (state.logLevelFilter !== 'all' && log.level !== state.logLevelFilter) {
        return;
    }

    const entry = document.createElement('div');
    entry.className = `log-entry log-${log.level.toLowerCase()}`;
    entry.dataset.logId = log.id;

    const time = document.createElement('span');
    time.className = 'log-time';
    time.textContent = formatTime(log.timestamp);

    const level = document.createElement('span');
    level.className = `log-level ${log.level}`;

    const message = document.createElement('span');
    message.className = 'log-message';
    message.textContent = log.message;

    entry.appendChild(time);
    entry.appendChild(level);
    entry.appendChild(message);

    logsContent.appendChild(entry);

    // 自动滚动到底部
    if (state.autoScrollLogs) {
        logsContent.scrollTop = logsContent.scrollHeight;
    }
}

// 格式化时间
function formatTime(date) {
    return date.toLocaleTimeString('zh-CN', { hour12: false });
}

// 清空日志
function clearLogs() {
    state.logs = [];
    const logsContent = document.getElementById('logsContent');
    if (logsContent) {
        logsContent.innerHTML = '<div class="log-entry log-info"><span class="log-time">--:--:--</span><span class="log-level">INFO</span><span class="log-message">日志已清空</span></div>';
    }
    addLog(LogLevel.INFO, '日志已清空');
}

// 导出日志
function exportLogs() {
    const content = state.logs.map(log => {
        return `[${log.timestamp.toISOString()}] [${log.level}] ${log.message}`;
    }).join('\n');

    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `claude-sync-logs-${Date.now()}.txt`;
    a.click();
    URL.revokeObjectURL(url);

    addLog(LogLevel.INFO, '日志已导出');
}

// 刷新日志显示（应用过滤）
function refreshLogsDisplay() {
    const logsContent = document.getElementById('logsContent');
    if (!logsContent) return;

    logsContent.innerHTML = '';

    for (const log of state.logs) {
        if (state.logLevelFilter === 'all' || log.level === state.logLevelFilter) {
            appendLogToUI(log);
        }
    }

    if (state.logs.length === 0) {
        logsContent.innerHTML = '<div class="log-entry log-info"><span class="log-time">--:--:--</span><span class="log-level">INFO</span><span class="log-message">暂无日志</span></div>';
    }
}

// 设置日志面板的事件监听
function setupLogsPanelListeners() {
    // 清空按钮
    document.getElementById('clearLogsBtn')?.addEventListener('click', clearLogs);

    // 导出按钮
    document.getElementById('exportLogsBtn')?.addEventListener('click', exportLogs);

    // 自动滚动复选框
    document.getElementById('autoScrollLogs')?.addEventListener('change', (e) => {
        state.autoScrollLogs = e.target.checked;
    });

    // 日志级别过滤
    document.getElementById('logLevelFilter')?.addEventListener('change', (e) => {
        state.logLevelFilter = e.target.value;
        refreshLogsDisplay();
    });
}

// 切换登录/注册模式
function switchAuthMode(mode) {
    if (mode === 'register') {
        document.getElementById('loginDialog').classList.remove('active');
        document.getElementById('registerDialog').classList.add('active');
    } else {
        document.getElementById('registerDialog').classList.remove('active');
        document.getElementById('loginDialog').classList.add('active');
    }
}

// Initialize app
async function init() {
    // 首先初始化日志系统（这样后面的 console.log 都会被捕获）
    initLogSystem();
    setupLogsPanelListeners();

    await loadConfig();
    await loadSyncStateFromDisk();
    await checkAuthStatus();
    setupEventListeners();
    setupNavigation();
    startConnectionCheck();
    updateUI();
}

// Load sync state from disk
async function loadSyncStateFromDisk() {
    try {
        await window.__TAURI__.invoke('load_sync_state_from_disk');
        console.log('Sync state loaded from disk');
    } catch (error) {
        console.error('Failed to load sync state:', error);
    }
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

    // Register dialog
    document.getElementById('closeRegisterDialog').addEventListener('click', () => {
        document.getElementById('registerDialog').classList.remove('active');
    });

    document.getElementById('registerForm').addEventListener('submit', async (e) => {
        e.preventDefault();
        await handleRegister();
    });

    // 切换登录/注册
    document.getElementById('switchToRegister').addEventListener('click', (e) => {
        e.preventDefault();
        switchAuthMode('register');
    });

    document.getElementById('switchToLogin').addEventListener('click', (e) => {
        e.preventDefault();
        switchAuthMode('login');
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

    // User info click - 支持退出登录
    document.getElementById('userInfo').addEventListener('click', async () => {
        if (state.isLoggedIn) {
            if (confirm('是否要退出登录？')) {
                await handleLogout();
            }
        } else {
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

    document.getElementById('refreshTreeBtn').addEventListener('click', async () => {
        await loadFileTree();
    });

    document.getElementById('editIgnoreBtn').addEventListener('click', async () => {
        await openIgnoreDialog();
    });

    // 忽略对话框
    document.getElementById('closeIgnoreDialog').addEventListener('click', closeIgnoreDialog);
    document.getElementById('closeIgnoreDialogBtn').addEventListener('click', closeIgnoreDialog);
    document.getElementById('addIgnorePatternBtn').addEventListener('click', handleAddIgnorePattern);
    document.getElementById('newIgnorePattern').addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            handleAddIgnorePattern();
        }
    });

    // 右键菜单
    document.getElementById('ctxAddToIgnore').addEventListener('click', handleAddToIgnore);
    document.getElementById('ctxDeleteFromServer').addEventListener('click', handleDeleteFromServer);

    // 点击其他地方关闭右键菜单
    document.addEventListener('click', hideContextMenu);
    document.addEventListener('contextmenu', (e) => {
        // 如果不是在文件树上，则隐藏右键菜单
        if (!e.target.closest('#fileTreeContainer')) {
            hideContextMenu();
        }
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
            loadFileTree();
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
        const displayName = state.username || state.user?.id || '用户';
        userInfo.querySelector('.user-avatar').textContent = displayName[0].toUpperCase();
        userInfo.querySelector('.user-name').textContent = displayName;
        userInfo.querySelector('.user-email').textContent = '点击退出登录';
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

// Handle register
async function handleRegister() {
    const username = document.getElementById('registerUsername').value.trim();
    const email = document.getElementById('registerEmail').value.trim();
    const password = document.getElementById('registerPassword').value;
    const confirmPassword = document.getElementById('registerConfirmPassword').value;

    // 验证
    if (password !== confirmPassword) {
        showNotification('两次输入的密码不一致', 'error');
        return;
    }
    if (password.length < 8) {
        showNotification('密码至少需要8位字符', 'error');
        return;
    }
    if (username.length < 3) {
        showNotification('用户名至少需要3位字符', 'error');
        return;
    }

    try {
        const result = await window.__TAURI__.invoke('register', {
            username,
            email,
            password,
        });

        console.log('Register result:', result);
        showNotification('注册成功，请登录', 'success');

        // 切换到登录界面并预填邮箱
        switchAuthMode('login');
        document.getElementById('loginEmail').value = email;
        document.getElementById('registerForm').reset();
    } catch (error) {
        console.error('Register failed:', error);
        showNotification('注册失败: ' + error, 'error');
    }
}

// Handle logout
async function handleLogout() {
    try {
        await window.__TAURI__.invoke('logout');
        state.isLoggedIn = false;
        state.user = null;
        state.username = null;
        updateAuthUI();
        showNotification('已退出登录', 'success');
    } catch (error) {
        console.error('Logout failed:', error);
        showNotification('退出失败: ' + error, 'error');
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

                // 同步完成后刷新文件列表
                await loadPendingFiles();
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

// Load pending files (已弃用，保留用于兼容)
async function loadPendingFiles() {
    try {
        const files = await window.__TAURI__.invoke('get_pending_files');
        state.pendingFiles = files;
    } catch (error) {
        console.error('Failed to load pending files:', error);
    }
}

// ========== 文件树相关函数 ==========

// 加载文件树
async function loadFileTree() {
    const container = document.getElementById('fileTreeContainer');
    container.innerHTML = '<div class="empty-state">加载中...</div>';

    try {
        console.log('[DEBUG] 开始加载文件树...');
        const tree = await window.__TAURI__.invoke('get_file_tree');
        console.log('[DEBUG] 文件树加载完成，根目录子节点数:', tree.children?.length || 0);
        state.fileTree = tree;
        renderFileTree();
        updateFileSummary();
    } catch (error) {
        console.error('Failed to load file tree:', error);
        container.innerHTML = '<div class="empty-state">加载失败: ' + escapeHtml(error) + '</div>';
    }
}

// 渲染文件树
function renderFileTree() {
    const container = document.getElementById('fileTreeContainer');

    if (!state.fileTree) {
        container.innerHTML = '<div class="empty-state">暂无文件</div>';
        return;
    }

    container.innerHTML = renderTreeNode(state.fileTree, 0, '');
}

// 渲染树节点
function renderTreeNode(node, depth, parentPath) {
    const fullPath = parentPath ? `${parentPath}/${node.path}` : node.path;
    const isExpanded = state.expandedFolders.has(fullPath || 'root');
    const hasChildren = node.children && node.children.length > 0;
    const isChecked = state.selectedForSync.has(fullPath) || (!state.selectedForSync.has(fullPath) && node.checked);

    // 节点缩进
    const indentStyle = `padding-left: ${depth * 20 + 8}px;`;

    // 图标
    let iconSvg = '';
    if (node.node_type === 'directory') {
        iconSvg = `
            <svg class="tree-icon-folder" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
            </svg>
        `;
    } else {
        iconSvg = `
            <svg class="tree-icon-file" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                <polyline points="14 2 14 8 20 8"></polyline>
            </svg>
        `;
    }

    // 状态图标
    let statusIcon = '';
    switch (node.sync_status) {
        case 'synced':
            statusIcon = `
                <svg class="tree-status synced" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
                    <polyline points="22 4 12 14.01 9 11.01"></polyline>
                </svg>
            `;
            break;
        case 'pending':
            statusIcon = `
                <svg class="tree-status pending" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <circle cx="12" cy="12" r="10"></circle>
                    <circle cx="12" cy="12" r="3"></circle>
                </svg>
            `;
            break;
        case 'not_on_server':
            statusIcon = `
                <svg class="tree-status not-on-server" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <circle cx="12" cy="12" r="10"></circle>
                </svg>
            `;
            break;
    }

    // Chevron 图标（仅用于有子项的目录）
    let chevronHtml = '';
    if (node.node_type === 'directory') {
        const chevronClass = isExpanded ? 'expanded' : '';
        chevronHtml = `
            <svg class="tree-chevron ${chevronClass}" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                 data-path="${escapeHtml(fullPath || 'root')}"
                 onclick="event.stopPropagation(); toggleFolder('${escapeHtml(fullPath || 'root')}')">
                <polyline points="9 18 15 12 9 6"></polyline>
            </svg>
        `;
    }

    // 构建节点 HTML
    let html = `
        <div class="tree-node"
             data-path="${escapeHtml(fullPath || 'root')}"
             data-type="${node.node_type}"
             data-exists-on-server="${node.exists_on_server}"
             style="${indentStyle}"
             oncontextmenu="showContextMenu(event, '${escapeHtml(fullPath || 'root')}', '${node.node_type}', ${node.exists_on_server})">
            ${chevronHtml}
            <input type="checkbox"
                   class="tree-checkbox"
                   ${isChecked ? 'checked' : ''}
                   data-path="${escapeHtml(fullPath || 'root')}"
                   data-type="${node.node_type}"
                   onchange="handleCheckboxChange('${escapeHtml(fullPath || 'root')}', this.checked, '${node.node_type}')">
            <div class="tree-icon">${iconSvg}</div>
            <span class="tree-name" title="${escapeHtml(node.path || node.name)}">${escapeHtml(node.name)}</span>
            <span class="tree-status-icon">${statusIcon}</span>
            ${node.node_type === 'file' ? `<span class="tree-size">${formatSize(node.size)}</span>` : ''}
        </div>
    `;

    // 渲染子节点（如果展开）
    if (hasChildren && isExpanded) {
        html += '<div class="tree-children">';
        for (const child of node.children) {
            html += renderTreeNode(child, depth + 1, fullPath || 'root');
        }
        html += '</div>';
    }

    return html;
}

// 切换文件夹展开/折叠
function toggleFolder(path) {
    if (state.expandedFolders.has(path)) {
        state.expandedFolders.delete(path);
    } else {
        state.expandedFolders.add(path);
    }
    renderFileTree();
}

// 处理复选框变化
function handleCheckboxChange(path, checked, nodeType) {
    // 更新选中状态
    if (checked) {
        state.selectedForSync.add(path);
    } else {
        state.selectedForSync.delete(path);
    }

    // 如果是目录，递归更新所有子项
    if (state.fileTree) {
        updateChildrenCheckedState(state.fileTree, path, checked);
    }

    renderFileTree();
    updateFileSummary();
}

// 递归更新子项的选中状态
function updateChildrenCheckedState(node, path, checked) {
    const nodePath = node.path || (node === state.fileTree ? 'root' : '');
    const fullPath = nodePath;

    if (fullPath === path || (path === 'root' && node === state.fileTree)) {
        // 找到目标节点，更新其所有子项
        setAllChecked(node, checked);
        return true;
    }

    if (node.children) {
        for (const child of node.children) {
            if (updateChildrenCheckedState(child, path, checked)) {
                return true;
            }
        }
    }
    return false;
}

// 设置节点及其所有子项的选中状态
function setAllChecked(node, checked) {
    const nodePath = node.path || (node === state.fileTree ? 'root' : '');
    if (checked) {
        state.selectedForSync.add(nodePath);
    } else {
        state.selectedForSync.delete(nodePath);
    }
    node.checked = checked;

    if (node.children) {
        for (const child of node.children) {
            setAllChecked(child, checked);
        }
    }
}

// 更新文件摘要信息
function updateFileSummary() {
    const summaryEl = document.getElementById('fileSummary');
    if (!state.fileTree) {
        summaryEl.innerHTML = '';
        return;
    }

    const stats = countFiles(state.fileTree);
    summaryEl.innerHTML = `
        <span>总文件: ${stats.total}</span>
        <span>待同步: ${stats.pending}</span>
        <span>已同步: ${stats.synced}</span>
        <span>大小: ${formatSize(stats.size)}</span>
    `;
}

// 统计文件数量
function countFiles(node) {
    let total = 0;
    let pending = 0;
    let synced = 0;
    let size = 0;

    if (node.node_type === 'file') {
        total++;
        size += node.size;
        if (node.sync_status === 'pending') pending++;
        else if (node.sync_status === 'synced') synced++;
    }

    if (node.children) {
        for (const child of node.children) {
            const childStats = countFiles(child);
            total += childStats.total;
            pending += childStats.pending;
            synced += childStats.synced;
            size += childStats.size;
        }
    }

    return { total, pending, synced, size };
}

// ========== 右键菜单相关函数 ==========

// 显示右键菜单
function showContextMenu(event, path, nodeType, existsOnServer) {
    event.preventDefault();
    event.stopPropagation();

    state.contextMenuTarget = { path, nodeType, existsOnServer };

    const menu = document.getElementById('contextMenu');

    // 根据文件是否在服务器上显示/隐藏"从服务器删除"选项
    const deleteOption = document.getElementById('ctxDeleteFromServer');
    if (existsOnServer) {
        deleteOption.style.display = 'flex';
    } else {
        deleteOption.style.display = 'none';
    }

    menu.style.display = 'block';
    menu.style.left = event.pageX + 'px';
    menu.style.top = event.pageY + 'px';
}

// 隐藏右键菜单
function hideContextMenu() {
    const menu = document.getElementById('contextMenu');
    menu.style.display = 'none';
    state.contextMenuTarget = null;
}

// 添加到忽略列表
async function handleAddToIgnore() {
    if (!state.contextMenuTarget) return;

    const { path } = state.contextMenuTarget;
    let pattern = path;

    // 如果是目录，添加 /** 后缀
    if (state.contextMenuTarget.nodeType === 'directory') {
        pattern = `${path}/**`;
    } else if (path.includes('/')) {
        // 如果是文件，可以只忽略文件名或完整路径
        const parts = path.split('/');
        const fileName = parts[parts.length - 1];
        if (parts.length > 1) {
            pattern = path; // 使用完整路径
        }
    }

    try {
        console.log('[DEBUG] 添加忽略模式:', pattern);
        await window.__TAURI__.invoke('add_ignore_pattern', { pattern });
        console.log('[DEBUG] 忽略模式已添加，重新加载文件树...');
        showNotification(`已添加到忽略列表: ${pattern}`, 'success');
        hideContextMenu();
        await loadFileTree(); // 重新加载文件树
    } catch (error) {
        console.error('Failed to add ignore pattern:', error);
        showNotification('添加忽略模式失败: ' + error, 'error');
    }
}

// 从服务器删除
async function handleDeleteFromServer() {
    if (!state.contextMenuTarget) return;

    const { path } = state.contextMenuTarget;

    if (!confirm(`确定要从服务器删除 "${path}" 吗？\n\n注意：这只会删除服务器上的文件，本地文件将保留。`)) {
        return;
    }

    try {
        await window.__TAURI__.invoke('delete_file_from_server', { filePath: path });
        showNotification(`已从服务器删除: ${path}`, 'success');
        hideContextMenu();
        await loadFileTree(); // 重新加载文件树
    } catch (error) {
        console.error('Failed to delete from server:', error);
        showNotification('删除失败: ' + error, 'error');
    }
}

// ========== 忽略列表对话框相关函数 ==========

// 打开忽略列表对话框
async function openIgnoreDialog() {
    document.getElementById('ignoreDialog').classList.add('active');
    await loadIgnorePatterns();
}

// 关闭忽略列表对话框
function closeIgnoreDialog() {
    document.getElementById('ignoreDialog').classList.remove('active');
    document.getElementById('newIgnorePattern').value = '';
}

// 加载忽略模式列表
async function loadIgnorePatterns() {
    try {
        const patterns = await window.__TAURI__.invoke('get_ignore_patterns');
        console.log('[DEBUG] 加载的忽略模式:', patterns);
        state.customIgnorePatterns = patterns;
        renderIgnorePatterns();
    } catch (error) {
        console.error('Failed to load ignore patterns:', error);
    }
}

// 渲染忽略模式列表
function renderIgnorePatterns() {
    const container = document.getElementById('ignorePatternsList');
    const countEl = document.getElementById('ignoreCount');

    countEl.textContent = `${state.customIgnorePatterns.length} 个模式`;

    if (state.customIgnorePatterns.length === 0) {
        container.innerHTML = '<div class="empty-state">暂无忽略模式</div>';
        return;
    }

    container.innerHTML = state.customIgnorePatterns.map((p, index) => `
        <div class="ignore-pattern-item">
            <code class="ignore-pattern-code">${escapeHtml(p.pattern)}</code>
            <button class="btn-icon" onclick="handleRemoveIgnorePattern(${index})" title="删除">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
            </button>
        </div>
    `).join('');
}

// 添加忽略模式
async function handleAddIgnorePattern() {
    const input = document.getElementById('newIgnorePattern');
    const pattern = input.value.trim();

    if (!pattern) {
        showNotification('请输入忽略模式', 'error');
        return;
    }

    try {
        await window.__TAURI__.invoke('add_ignore_pattern', { pattern });
        input.value = '';
        await loadIgnorePatterns();
        showNotification('忽略模式添加成功', 'success');
    } catch (error) {
        console.error('Failed to add ignore pattern:', error);
        showNotification('添加失败: ' + error, 'error');
    }
}

// 删除忽略模式
async function handleRemoveIgnorePattern(index) {
    const pattern = state.customIgnorePatterns[index]?.pattern;
    if (!pattern) return;

    try {
        await window.__TAURI__.invoke('remove_ignore_pattern', { pattern });
        await loadIgnorePatterns();
        showNotification('忽略模式已删除', 'success');
    } catch (error) {
        console.error('Failed to remove ignore pattern:', error);
        showNotification('删除失败: ' + error, 'error');
    }
}

// Format file size
function formatSize(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
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
                    <div class="device-name">
                        ${escapeHtml(device.name)}
                        ${device.is_current ? '<span class="badge-current">当前设备</span>' : ''}
                    </div>
                    <div class="device-meta">
                        <span>${device.device_type || '未知类型'}</span>
                        <span>${device.status === 'online' ? '在线' : '离线'}</span>
                    </div>
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
    document.getElementById('healthCheckAddress').value = server.health_check_address || 'http://localhost:8080';
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
            health_check_address: document.getElementById('healthCheckAddress').value,
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

        // 保存设置后重新检查连接状态
        await checkConnection();
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

// Check connection status
async function checkConnection() {
    if (state.connectionStatus.checking) return;

    state.connectionStatus.checking = true;
    updateConnectionStatusUI();

    try {
        const result = await window.__TAURI__.invoke('check_connection');
        state.connectionStatus.connected = result.connected;
        state.connectionStatus.message = result.message;
        console.log('Connection check result:', result);
    } catch (error) {
        console.error('Connection check failed:', error);
        state.connectionStatus.connected = false;
        state.connectionStatus.message = '检查失败';
    } finally {
        state.connectionStatus.checking = false;
        updateConnectionStatusUI();
    }
}

// Update connection status UI
function updateConnectionStatusUI() {
    const statusEl = document.getElementById('connectionStatus');
    if (!statusEl) return;

    const dotEl = statusEl.querySelector('.status-dot');
    const textEl = statusEl.querySelector('.status-text');

    // 移除所有状态类
    dotEl.classList.remove('connected', 'disconnected', 'checking');

    if (state.connectionStatus.checking) {
        dotEl.classList.add('checking');
        textEl.textContent = '检查中...';
    } else if (state.connectionStatus.connected) {
        dotEl.classList.add('connected');
        textEl.textContent = '已连接';
    } else {
        dotEl.classList.add('disconnected');
        textEl.textContent = state.connectionStatus.message || '未连接';
    }
}

// Start periodic connection check
function startConnectionCheck() {
    // 立即执行一次检查
    checkConnection();

    // 每 30 秒检查一次
    if (state.connectionCheckInterval) {
        clearInterval(state.connectionCheckInterval);
    }

    state.connectionCheckInterval = setInterval(() => {
        checkConnection();
    }, 30000);
}

// Stop connection check
function stopConnectionCheck() {
    if (state.connectionCheckInterval) {
        clearInterval(state.connectionCheckInterval);
        state.connectionCheckInterval = null;
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

// Make tree-related functions available globally
window.toggleFolder = toggleFolder;
window.handleCheckboxChange = handleCheckboxChange;
window.showContextMenu = showContextMenu;
window.handleAddToIgnore = handleAddToIgnore;
window.handleDeleteFromServer = handleDeleteFromServer;
window.handleRemoveIgnorePattern = handleRemoveIgnorePattern;

// Initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
