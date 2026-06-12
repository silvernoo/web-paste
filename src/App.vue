<template>
  <main
    class="app-shell"
    :class="[themeClass, { 'is-quick': isQuickWindow }]"
    @click="closeContextMenu"
    @contextmenu.prevent="openContextMenu($event)"
  >
    <section v-if="isQuickWindow" class="quick-window" :class="themeClass">
      <header class="quick-titlebar" data-tauri-drag-region>
        <div class="brand-mark" data-tauri-drag-region>WP</div>
        <div class="title-copy" data-tauri-drag-region>
          <strong data-tauri-drag-region>剪切板历史</strong>
          <span data-tauri-drag-region>{{ statusLabel }}</span>
        </div>
        <button class="icon-button" type="button" title="刷新" :disabled="historyLoading" @click.stop="loadHistory">
          <RefreshCw :size="15" :class="{ 'animate-spin': historyLoading }" />
        </button>
      </header>

      <div class="quick-search">
        <div class="search-field compact">
          <Search :size="15" />
          <input v-model.trim="query" placeholder="搜索本地历史" @input="scheduleHistoryLoad" />
          <button v-if="query" class="field-clear" type="button" title="清空搜索" @click.stop="clearDesktopSearch">
            <X :size="12" />
          </button>
        </div>
      </div>

      <section v-if="!initialized" class="state-pane">
        <span class="loading-pill">
          <LoaderCircle :size="16" class="animate-spin" />
          加载中
        </span>
      </section>

      <section v-else-if="needsOnboarding" class="state-pane">
        <div class="empty-state">
          <Cloud :size="28" />
          <strong>未登录同步</strong>
          <button class="primary-button" type="button" @click="openMainFromQuick">打开主界面</button>
        </div>
      </section>

      <section v-else class="quick-list scrollbar-none">
        <button
          v-for="item in filteredHistory"
          :key="item.id"
          class="quick-item"
          type="button"
          @click.stop="quickCopy(item.id)"
          @contextmenu.prevent.stop="openContextMenu($event, item)"
        >
          <span :class="typeIconClass(item.item_type)" class="type-icon small">
            <component :is="typeIcon(item.item_type)" :size="15" />
          </span>
          <span class="quick-item-body">
            <span class="quick-item-meta">
              <strong>{{ label(item.item_type) }}</strong>
              <span>{{ item.source_app || 'Unknown' }}</span>
            </span>
            <span class="quick-item-preview">{{ preview(item) }}</span>
          </span>
          <span class="quick-time">{{ formatDate(item.created_at) }}</span>
        </button>

        <div v-if="historyLoading" class="inline-state">
          <LoaderCircle :size="16" class="animate-spin" />
          加载中
        </div>
        <div v-else-if="filteredHistory.length === 0" class="inline-state empty">暂无记录</div>
      </section>
    </section>

    <template v-else>
      <section v-if="!initialized" class="desktop-window boot-window" :class="themeClass">
        <header class="window-titlebar" data-tauri-drag-region>
          <WindowTraffic v-if="isMacPlatform" @minimize="minimizeWindow" @close="hideMainWindow" />
          <div v-else class="titlebar-spacer" data-tauri-drag-region></div>
          <div class="window-title" data-tauri-drag-region>Web Paste</div>
          <WindowActions v-if="!isMacPlatform" @minimize="minimizeWindow" @close="hideMainWindow" />
          <div v-else class="titlebar-spacer" data-tauri-drag-region></div>
        </header>
        <div class="state-pane">
          <span class="loading-pill">
            <LoaderCircle :size="18" class="animate-spin" />
            加载中
          </span>
        </div>
      </section>

      <section v-else-if="needsOnboarding" class="desktop-window onboarding-window" :class="themeClass">
        <header class="window-titlebar" data-tauri-drag-region>
          <WindowTraffic v-if="isMacPlatform" @minimize="minimizeWindow" @close="hideMainWindow" />
          <div v-else class="titlebar-spacer" data-tauri-drag-region></div>
          <div class="window-title" data-tauri-drag-region>Web Paste</div>
          <WindowActions v-if="!isMacPlatform" @minimize="minimizeWindow" @close="hideMainWindow" />
          <div v-else class="titlebar-spacer" data-tauri-drag-region></div>
        </header>

        <div class="login-layout">
          <aside class="login-side">
            <div class="brand-lockup">
              <div class="brand-mark large">WP</div>
              <div>
                <strong>Web Paste</strong>
                <span>{{ settings.device_name || 'Desktop' }}</span>
              </div>
            </div>
            <div class="login-card-list">
              <div class="feature-row">
                <ClipboardList :size="17" />
                <span>
                  <strong>本地历史</strong>
                  <small>文本、图片、路径自动归档</small>
                </span>
              </div>
              <div class="feature-row">
                <ShieldCheck :size="17" />
                <span>
                  <strong>隐私规则</strong>
                  <small>应用名单和正则脱敏</small>
                </span>
              </div>
              <div class="feature-row">
                <Cloud :size="17" />
                <span>
                  <strong>云端同步</strong>
                  <small>多设备实时推送</small>
                </span>
              </div>
            </div>
          </aside>

          <form class="login-form" @submit.prevent="login">
            <div class="section-heading">
              <span class="eyebrow">Account</span>
              <h1>登录同步</h1>
              <p>账号密码登录，设备会自动绑定。</p>
            </div>

            <div class="form-stack">
              <label class="field">
                <span>用户名</span>
                <input v-model.trim="loginForm.username" autocomplete="username" spellcheck="false" />
              </label>
              <label class="field">
                <span>密码</span>
                <input v-model="loginForm.password" autocomplete="current-password" type="password" />
              </label>
            </div>

            <p v-if="errorMessage" class="error-banner">
              <AlertCircle :size="16" />
              {{ errorMessage }}
            </p>

            <div class="login-actions">
              <button class="primary-button" type="submit" :disabled="authLoading">
                <LoaderCircle v-if="authLoading" :size="17" class="animate-spin" />
                <LogIn v-else :size="17" />
                登录
              </button>
              <button class="secondary-button" type="button" :disabled="authLoading" @click="enterOfflineMode">
                <HardDrive :size="17" />
                离线模式
              </button>
            </div>

            <div class="link-row">
              <button type="button" :disabled="authLoading" @click="openWebAuth('register')">注册</button>
              <button type="button" :disabled="authLoading" @click="openWebAuth('forgot-password')">忘记密码</button>
            </div>
          </form>
        </div>
      </section>

      <section v-else class="desktop-window" :class="themeClass">
        <header class="window-titlebar" data-tauri-drag-region>
          <WindowTraffic v-if="isMacPlatform" @minimize="minimizeWindow" @close="hideMainWindow" />
          <div v-else class="titlebar-spacer" data-tauri-drag-region></div>
          <div class="window-title" data-tauri-drag-region>
            <span>{{ view === 'history' ? '剪切板历史' : '设置' }}</span>
            <kbd>{{ displayShortcut(settings.global_shortcut) }}</kbd>
          </div>
          <WindowActions v-if="!isMacPlatform" @minimize="minimizeWindow" @close="hideMainWindow" />
          <div v-else class="titlebar-spacer" data-tauri-drag-region></div>
        </header>

        <div class="workspace">
          <aside class="sidebar" data-tauri-drag-region>
            <div class="sidebar-brand">
              <div class="brand-mark">WP</div>
              <div class="title-copy">
                <strong>Web Paste</strong>
                <span>{{ accountLabel }}</span>
              </div>
            </div>

            <nav class="nav-list">
              <button :class="{ active: view === 'history' }" type="button" title="历史" aria-label="历史" @click="view = 'history'">
                <ClipboardList :size="17" />
              </button>
              <button :class="{ active: view === 'settings' }" type="button" title="设置" aria-label="设置" @click="view = 'settings'">
                <Settings :size="17" />
              </button>
            </nav>

            <div class="sidebar-status">
              <div class="status-card">
                <span :class="statusDotClass" class="status-dot"></span>
                <div>
                  <strong>{{ statusLabel }}</strong>
                  <span>{{ settings.device_name || 'Desktop' }}</span>
                </div>
              </div>
              <button class="secondary-button full" type="button" :disabled="pauseSaving" @click="togglePaused">
                <LoaderCircle v-if="pauseSaving" :size="16" class="animate-spin" />
                <Play v-else-if="paused" :size="16" />
                <Pause v-else :size="16" />
                {{ paused ? '恢复监听' : '暂停监听' }}
              </button>
            </div>
          </aside>

          <div class="content">
            <header class="content-toolbar">
              <div class="toolbar-title">
                <h1>{{ view === 'history' ? '剪切板历史' : '设置' }}</h1>
                <span>{{ toolbarSubtitle }}</span>
              </div>
              <div class="toolbar-actions">
                <button class="icon-button" type="button" title="命令面板" @click="commandOpen = true">
                  <Command :size="16" />
                </button>
                <button v-if="view === 'history'" class="icon-button" type="button" title="刷新" :disabled="historyLoading" @click="loadHistory">
                  <RefreshCw :size="16" :class="{ 'animate-spin': historyLoading }" />
                </button>
                <button
                  v-if="view === 'history'"
                  class="icon-button danger"
                  type="button"
                  title="清空本地历史"
                  :disabled="clearingHistory || history.length === 0"
                  @click="clearLocalHistory"
                >
                  <LoaderCircle v-if="clearingHistory" :size="16" class="animate-spin" />
                  <Trash2 v-else :size="16" />
                </button>
                <button class="secondary-button" type="button" :disabled="syncing || settings.offline_mode" @click="syncNow">
                  <LoaderCircle v-if="syncing" :size="16" class="animate-spin" />
                  <RefreshCw v-else :size="16" />
                  同步
                </button>
              </div>
            </header>

            <div v-if="errorMessage" class="error-banner floating">
              <AlertCircle :size="17" />
              <span>{{ errorMessage }}</span>
              <button type="button" @click="errorMessage = ''">
                <X :size="14" />
              </button>
            </div>

            <section v-if="view === 'history'" class="history-view">
              <div class="history-main">
                <div class="filterbar">
                  <div class="search-field">
                    <Search :size="17" />
                    <input v-model.trim="query" placeholder="搜索文本、路径、来源应用" spellcheck="false" @input="scheduleHistoryLoad" />
                    <button v-if="query" class="field-clear" type="button" title="清空搜索" @click="clearDesktopSearch">
                      <X :size="12" />
                    </button>
                  </div>

                  <div class="segmented-control">
                    <button v-for="option in typeOptions" :key="option.value" :class="{ active: typeFilter === option.value }" type="button" @click="typeFilter = option.value">
                      {{ option.label }}
                    </button>
                  </div>
                </div>

                <div class="history-list scrollbar-none">
                  <article
                    v-for="item in filteredHistory"
                    :key="item.id"
                    class="history-item"
                    tabindex="0"
                    @dblclick="copy(item.id)"
                    @keydown.enter.prevent="copy(item.id)"
                    @contextmenu.prevent.stop="openContextMenu($event, item)"
                  >
                    <div :class="typeIconClass(item.item_type)" class="type-icon">
                      <component :is="typeIcon(item.item_type)" :size="18" />
                    </div>
                    <div class="item-body">
                      <div class="item-meta">
                        <strong>{{ label(item.item_type) }}</strong>
                        <span :class="item.synced ? 'synced' : 'pending'" class="sync-badge">
                          {{ item.synced ? '已同步' : '待同步' }}
                        </span>
                        <span class="source">{{ item.source_app || 'Unknown' }}</span>
                      </div>

                      <div v-if="item.item_type === 'image'" class="image-preview">
                        <img v-if="imagePreviewUrl(item)" :src="imagePreviewUrl(item)" alt="" loading="lazy" draggable="false" />
                        <span v-else>{{ imagePreviewText(item) }}</span>
                      </div>
                      <pre v-else class="item-preview selectable-text">{{ preview(item) }}</pre>
                    </div>

                    <div class="item-actions">
                      <span>{{ formatDate(item.created_at) }}</span>
                      <button class="primary-button small" type="button" :disabled="copyingId === item.id" @click.stop="copy(item.id)">
                        <LoaderCircle v-if="copyingId === item.id" :size="14" class="animate-spin" />
                        <Copy v-else :size="14" />
                        复制
                      </button>
                    </div>
                  </article>

                  <div v-if="historyLoading" class="large-state">
                    <LoaderCircle :size="18" class="animate-spin" />
                    加载中
                  </div>
                  <div v-else-if="filteredHistory.length === 0" class="large-state empty">
                    <ClipboardList :size="24" />
                    暂无本地记录
                  </div>
                </div>
              </div>

            </section>

            <section v-else class="settings-view scrollbar-none">
              <div class="settings-grid">
                <section class="panel">
                  <div class="panel-title">
                    <Cloud :size="17" />
                    <strong>账号</strong>
                    <span :class="settings.offline_mode ? 'muted' : 'good'" class="panel-badge">
                      {{ settings.offline_mode ? '离线模式' : '已登录' }}
                    </span>
                  </div>
                  <div class="account-row">
                    <div>
                      <small>当前账号</small>
                      <strong>{{ accountLabel }}</strong>
                    </div>
                    <div class="row-actions">
                      <button class="secondary-button" type="button" @click="returnToLogin">
                        <LogIn :size="16" />
                        {{ settings.offline_mode ? '登录同步' : '切换账号' }}
                      </button>
                      <button v-if="!settings.offline_mode" class="secondary-button danger-text" type="button" @click="logout">
                        <LogOut :size="16" />
                        退出
                      </button>
                    </div>
                  </div>
                </section>

                <section class="panel">
                  <div class="panel-title">
                    <Monitor :size="17" />
                    <strong>外观</strong>
                  </div>
                  <div class="setting-row">
                    <div>
                      <strong>主题</strong>
                      <small>跟随桌面材质的浅色或深色界面</small>
                    </div>
                    <div class="segmented-control">
                      <button :class="{ active: uiThemePreference === 'system' }" type="button" @click="setUiThemePreference('system')">
                        <Monitor :size="15" />
                        系统
                      </button>
                      <button :class="{ active: uiThemePreference === 'white' }" type="button" @click="setUiThemePreference('white')">
                        <Sun :size="15" />
                        浅色
                      </button>
                      <button :class="{ active: uiThemePreference === 'black' }" type="button" @click="setUiThemePreference('black')">
                        <Moon :size="15" />
                        深色
                      </button>
                    </div>
                  </div>
                </section>

                <section class="panel">
                  <div class="panel-title">
                    <Keyboard :size="17" />
                    <strong>设备</strong>
                    <span v-if="saving" class="panel-badge muted">保存中</span>
                  </div>
                  <div class="form-stack">
                    <label class="field">
                      <span>设备名</span>
                      <input v-model.trim="settings.device_name" spellcheck="false" />
                    </label>
                    <div class="field">
                      <span>快捷键</span>
                      <div class="shortcut-row">
                        <button
                          class="shortcut-recorder"
                          :class="{ recording: shortcutRecording }"
                          type="button"
                          @click="beginShortcutRecording"
                          @keydown.prevent.stop="recordShortcut"
                        >
                          <span>{{ shortcutRecording ? '按下组合键' : displayShortcut(settings.global_shortcut) }}</span>
                          <kbd>录制</kbd>
                        </button>
                        <button class="secondary-button" type="button" @click="resetShortcut">默认</button>
                      </div>
                    </div>
                    <label class="switch-row">
                      <span>
                        <strong>登录时启动</strong>
                        <small>开机登录系统后自动启动 Web Paste</small>
                      </span>
                      <input v-model="settings.start_on_login" type="checkbox" />
                    </label>
                  </div>
                </section>

                <section class="panel">
                  <div class="panel-title">
                    <ShieldCheck :size="17" />
                    <strong>隐私策略</strong>
                  </div>
                  <div class="form-stack">
                    <label class="field">
                      <span>应用策略</span>
                      <select v-model="settings.privacy_mode">
                        <option value="off">关闭</option>
                        <option value="blacklist">黑名单</option>
                        <option value="whitelist">白名单</option>
                      </select>
                    </label>
                    <label class="field">
                      <span>应用列表</span>
                      <textarea v-model="appRulesDraft" rows="3" spellcheck="false" @blur="applyTextSettingsNow" />
                    </label>
                    <label class="field">
                      <span>脱敏正则</span>
                      <textarea v-model="maskRulesDraft" rows="3" class="mono" spellcheck="false" @blur="applyTextSettingsNow" />
                    </label>
                  </div>
                </section>

                <section class="panel">
                  <div class="panel-title">
                    <Webhook :size="17" />
                    <strong>Webhook</strong>
                  </div>
                  <label class="field">
                    <span>推送地址</span>
                    <textarea v-model="webhookDraft" rows="4" spellcheck="false" @blur="applyTextSettingsNow" />
                  </label>
                </section>
              </div>
            </section>
          </div>
        </div>
      </section>
    </template>

    <div v-if="notice" class="toast" :class="notice.kind">
      <CheckCircle2 v-if="notice.kind === 'success'" :size="17" />
      <AlertCircle v-else :size="17" />
      <span>{{ notice.text }}</span>
      <button type="button" @click.stop="notice = null">
        <X :size="14" />
      </button>
    </div>

    <div v-if="contextMenu.open" class="context-menu" :style="contextMenuStyle" @click.stop>
      <button type="button" :disabled="!contextMenu.item" @click="copyFromContext">
        <Copy :size="14" />
        复制
        <kbd>Enter</kbd>
      </button>
      <button type="button" @click="loadHistory">
        <RefreshCw :size="14" />
        刷新
        <kbd>⌘R</kbd>
      </button>
      <hr />
      <button type="button" @click="commandOpen = true">
        <Command :size="14" />
        命令面板
        <kbd>⌘K</kbd>
      </button>
    </div>

    <div v-if="commandOpen" class="command-overlay" @click.self="commandOpen = false">
      <section class="command-panel" role="dialog" aria-modal="true">
        <div class="command-input">
          <Command :size="17" />
          <input ref="commandInputRef" v-model.trim="commandQuery" placeholder="输入命令或搜索历史" @keydown.escape="commandOpen = false" @keydown.enter.prevent="runFirstCommand" />
        </div>
        <div class="command-list">
          <button v-for="action in visibleCommands" :key="action.id" type="button" @click="runCommand(action.id)">
            <component :is="action.icon" :size="16" />
            <span>{{ action.label }}</span>
            <kbd v-if="action.shortcut">{{ action.shortcut }}</kbd>
          </button>
        </div>
      </section>
    </div>
  </main>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import { confirm } from '@tauri-apps/api/dialog';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/shell';
import { invoke } from '@tauri-apps/api/tauri';
import { appWindow } from '@tauri-apps/api/window';
import {
  AlertCircle,
  CheckCircle2,
  ClipboardList,
  Cloud,
  Command,
  Copy,
  File as FileIcon,
  FileText,
  HardDrive,
  Image as ImageIcon,
  Keyboard,
  LoaderCircle,
  LogIn,
  LogOut,
  Minus,
  Monitor,
  Moon,
  Pause,
  Play,
  RefreshCw,
  Search,
  Settings,
  ShieldCheck,
  Square,
  Sun,
  Trash2,
  Webhook,
  X
} from '@lucide/vue';

interface ClientSettings {
  api_base: string;
  token: string;
  refresh_token: string;
  device_id: string;
  device_name: string;
  device_fingerprint: string;
  account_name: string;
  offline_mode: boolean;
  poll_interval_ms: number;
  paused: boolean;
  privacy_mode: string;
  app_rules: string[];
  mask_rules: string[];
  webhook_urls: string[];
  global_shortcut: string;
  start_on_login: boolean;
}

interface LocalItem {
  id: string;
  item_type: string;
  content?: string;
  file_path?: string;
  object_path?: string;
  mime_type?: string;
  source_app?: string;
  synced: boolean;
  created_at: string;
}

type Notice = {
  kind: 'success' | 'error';
  text: string;
};

type UiTheme = 'white' | 'black';
type UiThemePreference = UiTheme | 'system';
type CommandId = 'history' | 'settings' | 'sync' | 'pause' | 'theme' | 'refresh';

const WindowTraffic = defineComponent({
  name: 'WindowTraffic',
  emits: ['minimize', 'close'],
  setup(_, { emit }) {
    return () =>
      h('div', { class: 'traffic-lights' }, [
        h('button', { class: 'traffic close', type: 'button', title: '关闭', onClick: () => emit('close') }),
        h('button', { class: 'traffic minimize', type: 'button', title: '最小化', onClick: () => emit('minimize') }),
        h('span', { class: 'traffic disabled', title: '不支持最大化' })
      ]);
  }
});

const WindowActions = defineComponent({
  name: 'WindowActions',
  emits: ['minimize', 'close'],
  setup(_, { emit }) {
    return () =>
      h('div', { class: 'window-actions' }, [
        h(
          'button',
          {
            class: 'window-action',
            type: 'button',
            title: '最小化',
            onClick: () => emit('minimize')
          },
          [h(Minus, { size: 15 })]
        ),
        h(
          'button',
          {
            class: 'window-action',
            type: 'button',
            title: '不支持最大化',
            disabled: true,
            'aria-disabled': 'true'
          },
          [h(Square, { size: 13 })]
        ),
        h(
          'button',
          {
            class: 'window-action close',
            type: 'button',
            title: '关闭',
            onClick: () => emit('close')
          },
          [h(X, { size: 15 })]
        )
      ]);
  }
});

const defaultApiBase = 'https://paste-api.dangolabs.top';
const defaultWebBase = (import.meta.env.VITE_WEB_BASE_URL || 'https://paste.dangolabs.top').replace(/\/$/, '');
const uiThemeStorageKey = 'web-paste-ui-theme';
const isQuickWindow = new URLSearchParams(window.location.search).get('quick') === '1';
const isMacPlatform = /mac/i.test(navigator.platform);

const settings = ref<ClientSettings>({
  api_base: defaultApiBase,
  token: '',
  refresh_token: '',
  device_id: '',
  device_name: 'Desktop',
  device_fingerprint: '',
  account_name: '',
  offline_mode: false,
  poll_interval_ms: 800,
  paused: false,
  privacy_mode: 'off',
  app_rules: [],
  mask_rules: ['\\b1[3-9]\\d{9}\\b'],
  webhook_urls: [],
  global_shortcut: 'CommandOrControl+Shift+V',
  start_on_login: false
});

const loginForm = ref({
  username: '',
  password: ''
});

const initialized = ref(false);
const view = ref<'history' | 'settings'>('history');
const history = ref<LocalItem[]>([]);
const query = ref('');
const typeFilter = ref('');
const online = ref(false);
const historyLoading = ref(false);
const authLoading = ref(false);
const saving = ref(false);
const syncing = ref(false);
const pauseSaving = ref(false);
const clearingHistory = ref(false);
const copyingId = ref('');
const errorMessage = ref('');
const notice = ref<Notice | null>(null);
const systemDark = ref(window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false);
const uiThemePreference = ref<UiThemePreference>(readStoredUiThemePreference());
const shortcutRecording = ref(false);
const appRulesDraft = ref('');
const maskRulesDraft = ref('');
const webhookDraft = ref('');
const imagePreviewUrls = ref(new Map<string, string>());
const imagePreviewFailedIds = ref(new Set<string>());
const commandOpen = ref(false);
const commandQuery = ref('');
const commandInputRef = ref<HTMLInputElement | null>(null);
const contextMenu = reactive<{
  open: boolean;
  x: number;
  y: number;
  item: LocalItem | null;
}>({
  open: false,
  x: 0,
  y: 0,
  item: null
});

const imagePreviewLoadingIds = new Set<string>();
let imagePreviewQueue = Promise.resolve();
let searchTimer = 0;
let noticeTimer = 0;
let historyReloadTimer = 0;
let settingsSaveTimer = 0;
let textSettingsTimer = 0;
let lastSavedSettingsSnapshot = settingsSnapshot(settingsForSave());
let settingsSaveRequestActive = false;
let unlistenHistoryChanged: UnlistenFn | null = null;
let unlistenShowMain: UnlistenFn | null = null;
let syncingTextDrafts = false;
const systemThemeMedia = window.matchMedia?.('(prefers-color-scheme: dark)');

const typeOptions = [
  { value: '', label: '全部' },
  { value: 'text', label: '文本' },
  { value: 'image', label: '图片' },
  { value: 'file_path', label: '路径' }
];

const paused = computed(() => settings.value.paused);
const isSignedIn = computed(() => Boolean(settings.value.token && settings.value.device_id));
const needsOnboarding = computed(() => !settings.value.offline_mode && !isSignedIn.value);
const pendingCount = computed(() => history.value.filter((item) => !item.synced).length);
const syncedCount = computed(() => history.value.filter((item) => item.synced).length);
const resolvedUiTheme = computed<UiTheme>(() => (uiThemePreference.value === 'system' ? (systemDark.value ? 'black' : 'white') : uiThemePreference.value));
const themeClass = computed(() => `theme-${resolvedUiTheme.value}`);
const accountLabel = computed(() => {
  if (settings.value.offline_mode) return '离线模式';
  return settings.value.account_name || '未登录';
});
const statusLabel = computed(() => {
  if (settings.value.offline_mode) return '离线模式';
  if (paused.value) return '已暂停';
  if (syncing.value) return '同步中';
  if (online.value) return '在线';
  return pendingCount.value > 0 ? '待同步' : '待机';
});
const statusDotClass = computed(() => {
  if (settings.value.offline_mode) return 'bg-slate-400';
  if (paused.value) return 'bg-amber-400';
  if (online.value) return 'bg-emerald-400';
  return pendingCount.value > 0 ? 'bg-amber-400' : 'bg-teal-400';
});
const filteredHistory = computed(() => {
  if (!typeFilter.value) return history.value;
  return history.value.filter((item) => item.item_type === typeFilter.value);
});
const toolbarSubtitle = computed(() => {
  if (view.value === 'settings') return saving.value ? '设置保存中' : '本机偏好与同步策略';
  return `${filteredHistory.value.length} 条记录 · ${statusLabel.value}`;
});
const contextMenuStyle = computed(() => ({
  left: `${contextMenu.x}px`,
  top: `${contextMenu.y}px`
}));
const commands = computed(() => [
  { id: 'history' as const, label: '打开剪切板历史', icon: ClipboardList, shortcut: '⌘1' },
  { id: 'settings' as const, label: '打开设置', icon: Settings, shortcut: '⌘,' },
  { id: 'refresh' as const, label: '刷新历史', icon: RefreshCw, shortcut: '⌘R' },
  { id: 'sync' as const, label: '立即同步', icon: Cloud, shortcut: '' },
  { id: 'pause' as const, label: paused.value ? '恢复监听' : '暂停监听', icon: paused.value ? Play : Pause, shortcut: '' },
  { id: 'theme' as const, label: resolvedUiTheme.value === 'white' ? '切换深色主题' : '切换浅色主题', icon: resolvedUiTheme.value === 'white' ? Moon : Sun, shortcut: '' }
]);
const visibleCommands = computed(() => {
  const q = commandQuery.value.toLowerCase();
  if (!q) return commands.value;
  return commands.value.filter((item) => item.label.toLowerCase().includes(q));
});

watch(
  settings,
  () => {
    if (!initialized.value || needsOnboarding.value) return;
    scheduleSettingsSave();
  },
  { deep: true }
);

watch([appRulesDraft, maskRulesDraft, webhookDraft], () => {
  if (!initialized.value || syncingTextDrafts) return;
  scheduleTextSettingsApply();
});

watch(commandOpen, async (open) => {
  if (!open) {
    commandQuery.value = '';
    return;
  }
  await nextTick();
  commandInputRef.value?.focus();
});

function readStoredUiThemePreference(): UiThemePreference {
  try {
    const stored = window.localStorage.getItem(uiThemeStorageKey);
    if (stored === 'black' || stored === 'white' || stored === 'system') return stored;
  } catch {
    // Theme persistence is optional.
  }
  return 'system';
}

function setUiThemePreference(next: UiThemePreference) {
  uiThemePreference.value = next;
  try {
    window.localStorage.setItem(uiThemeStorageKey, next);
  } catch {
    // Theme persistence is optional; the in-memory selection still applies.
  }
}

onMounted(async () => {
  installWebviewGuards();
  window.addEventListener('keydown', handleGlobalKeydown);
  systemThemeMedia?.addEventListener('change', handleSystemThemeChange);
  unlistenHistoryChanged = await listen('clipboard-history-changed', () => {
    scheduleHistoryReload();
  });
  unlistenShowMain = await listen<{ view?: 'history' | 'settings' }>('show-main', (event) => {
    const nextView = event.payload?.view === 'settings' ? 'settings' : 'history';
    view.value = nextView;
    shortcutRecording.value = false;
    if (nextView === 'history') scheduleHistoryReload();
  });
  await loadInitial();
});

onBeforeUnmount(() => {
  window.clearTimeout(searchTimer);
  window.clearTimeout(noticeTimer);
  window.clearTimeout(historyReloadTimer);
  window.clearTimeout(settingsSaveTimer);
  window.clearTimeout(textSettingsTimer);
  window.removeEventListener('keydown', handleGlobalKeydown);
  systemThemeMedia?.removeEventListener('change', handleSystemThemeChange);
  unlistenHistoryChanged?.();
  unlistenHistoryChanged = null;
  unlistenShowMain?.();
  unlistenShowMain = null;
  clearImagePreviews();
});

function installWebviewGuards() {
  document.addEventListener('contextmenu', (event) => {
    if (event.defaultPrevented) return;
    event.preventDefault();
  });
  document.addEventListener('dragstart', (event) => event.preventDefault());
  document.addEventListener(
    'wheel',
    (event) => {
      if (event.ctrlKey || event.metaKey) event.preventDefault();
    },
    { passive: false }
  );
}

function handleSystemThemeChange(event: MediaQueryListEvent) {
  systemDark.value = event.matches;
}

function handleGlobalKeydown(event: KeyboardEvent) {
  const commandKey = isMacPlatform ? event.metaKey : event.ctrlKey;
  if ((event.ctrlKey || event.metaKey) && ['+', '=', '-', '0'].includes(event.key)) {
    event.preventDefault();
    return;
  }
  if (commandKey && event.key.toLowerCase() === 'k') {
    event.preventDefault();
    commandOpen.value = true;
    return;
  }
  if (commandKey && event.key.toLowerCase() === 'r') {
    event.preventDefault();
    void loadHistory();
    return;
  }
  if (commandKey && event.key === '1') {
    event.preventDefault();
    view.value = 'history';
    return;
  }
  if (commandKey && event.key === ',') {
    event.preventDefault();
    view.value = 'settings';
  }
}

async function minimizeWindow() {
  try {
    await appWindow.minimize();
  } catch (err) {
    console.warn('minimize window failed', err);
  }
}

async function hideMainWindow() {
  try {
    await invoke('hide_main_window');
  } catch (err) {
    console.warn('hide main window failed', err);
  }
}

async function loadInitial() {
  errorMessage.value = '';
  try {
    applyPersistedSettings(await invoke<ClientSettings>('get_client_settings'));
    if (!needsOnboarding.value) await loadHistory();
  } catch (err) {
    errorMessage.value = messageFromError(err, '加载桌面端配置失败');
  } finally {
    initialized.value = true;
  }
}

async function login() {
  authLoading.value = true;
  errorMessage.value = '';
  try {
    applyPersistedSettings(await invoke<ClientSettings>('login_with_password', {
      username: loginForm.value.username,
      password: loginForm.value.password
    }));
    loginForm.value.password = '';
    view.value = 'history';
    await loadHistory();
    showNotice('success', '登录成功');
  } catch (err) {
    errorMessage.value = messageFromError(err, '登录失败');
  } finally {
    authLoading.value = false;
  }
}

async function openWebAuth(path: 'register' | 'forgot-password') {
  errorMessage.value = '';
  try {
    await open(`${defaultWebBase}/${path}`);
  } catch (err) {
    errorMessage.value = messageFromError(err, '无法打开浏览器');
  }
}

async function enterOfflineMode() {
  authLoading.value = true;
  errorMessage.value = '';
  try {
    applyPersistedSettings(await invoke<ClientSettings>('use_offline_mode'));
    view.value = 'history';
    await loadHistory();
    showNotice('success', '已进入离线模式');
  } catch (err) {
    errorMessage.value = messageFromError(err, '进入离线模式失败');
  } finally {
    authLoading.value = false;
  }
}

async function logout() {
  errorMessage.value = '';
  try {
    applyPersistedSettings(await invoke<ClientSettings>('logout_client'));
    view.value = 'history';
    showNotice('success', '已退出登录');
  } catch (err) {
    errorMessage.value = messageFromError(err, '退出登录失败');
  }
}

async function returnToLogin() {
  await logout();
}

function clearDesktopSearch() {
  if (!query.value) return;
  query.value = '';
  void loadHistory();
}

async function quickCopy(id: string) {
  copyingId.value = id;
  errorMessage.value = '';
  try {
    await invoke('recopy_item', { id });
    await invoke('close_quick_window');
  } catch (err) {
    errorMessage.value = messageFromError(err, '复制失败');
    showNotice('error', errorMessage.value);
  } finally {
    copyingId.value = '';
  }
}

async function openMainFromQuick() {
  try {
    await invoke('open_main_window');
    await invoke('close_quick_window');
  } catch (err) {
    errorMessage.value = messageFromError(err, '打开主界面失败');
  }
}

function scheduleHistoryLoad() {
  window.clearTimeout(searchTimer);
  searchTimer = window.setTimeout(() => {
    loadHistory();
  }, 180);
}

function scheduleHistoryReload() {
  window.clearTimeout(historyReloadTimer);
  historyReloadTimer = window.setTimeout(() => {
    if (!needsOnboarding.value) loadHistory({ silent: true });
  }, 120);
}

function scheduleSettingsSave(delay = 450) {
  const snapshot = settingsSnapshot(settingsForSave());
  if (snapshot === lastSavedSettingsSnapshot) return;
  window.clearTimeout(settingsSaveTimer);
  settingsSaveTimer = window.setTimeout(() => {
    void saveSettingsNow();
  }, delay);
}

function scheduleTextSettingsApply(delay = 550) {
  window.clearTimeout(textSettingsTimer);
  textSettingsTimer = window.setTimeout(() => {
    applyTextSettingsNow();
  }, delay);
}

function applyTextSettingsNow() {
  window.clearTimeout(textSettingsTimer);
  settings.value.app_rules = lines(appRulesDraft.value);
  settings.value.mask_rules = lines(maskRulesDraft.value);
  settings.value.webhook_urls = lines(webhookDraft.value);
}

async function saveSettingsNow() {
  const next = settingsForSave();
  const snapshot = settingsSnapshot(next);
  if (snapshot === lastSavedSettingsSnapshot || settingsSaveRequestActive) return;

  settingsSaveRequestActive = true;
  saving.value = true;
  errorMessage.value = '';
  try {
    await invoke('save_client_settings', { next });
    lastSavedSettingsSnapshot = snapshot;
    settings.value.poll_interval_ms = next.poll_interval_ms;
  } catch (err) {
    errorMessage.value = messageFromError(err, '自动保存失败');
    showNotice('error', errorMessage.value);
  } finally {
    saving.value = false;
    settingsSaveRequestActive = false;
    if (settingsSnapshot(settingsForSave()) !== snapshot) scheduleSettingsSave(200);
  }
}

async function loadHistory(options: { silent?: boolean } = {}) {
  if (historyLoading.value) return;
  const silent = Boolean(options.silent);
  if (!silent) {
    historyLoading.value = true;
    errorMessage.value = '';
  }
  try {
    history.value = await invoke<LocalItem[]>('get_history', { query: query.value, limit: 100, offset: 0 });
    refreshImagePreviews();
  } catch (err) {
    if (!silent) errorMessage.value = messageFromError(err, '加载本地历史失败');
  } finally {
    if (!silent) historyLoading.value = false;
  }
}

function beginShortcutRecording(event: MouseEvent) {
  shortcutRecording.value = true;
  (event.currentTarget as HTMLButtonElement | null)?.focus();
}

function recordShortcut(event: KeyboardEvent) {
  if (!shortcutRecording.value) return;
  if (event.key === 'Escape') {
    shortcutRecording.value = false;
    return;
  }

  const key = shortcutKey(event);
  if (!key) return;

  const parts: string[] = [];
  if ((isMacPlatform && event.metaKey) || (!isMacPlatform && event.ctrlKey)) {
    parts.push('CommandOrControl');
  }
  if (isMacPlatform && event.ctrlKey) parts.push('Control');
  if (!isMacPlatform && event.metaKey) parts.push('Super');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  parts.push(key);

  settings.value.global_shortcut = [...new Set(parts)].join('+');
  shortcutRecording.value = false;
}

function shortcutKey(event: KeyboardEvent) {
  if (['Control', 'Meta', 'Shift', 'Alt'].includes(event.key)) return '';
  if (/^Key[A-Z]$/.test(event.code)) return event.code.slice(3);
  if (/^Digit[0-9]$/.test(event.code)) return event.code.slice(5);
  if (/^F\d{1,2}$/.test(event.key)) return event.key;

  const named: Record<string, string> = {
    ' ': 'Space',
    ArrowUp: 'ArrowUp',
    ArrowDown: 'ArrowDown',
    ArrowLeft: 'ArrowLeft',
    ArrowRight: 'ArrowRight',
    Backspace: 'Backspace',
    Delete: 'Delete',
    Enter: 'Enter',
    Home: 'Home',
    End: 'End',
    PageUp: 'PageUp',
    PageDown: 'PageDown',
    Insert: 'Insert',
    Tab: 'Tab'
  };
  if (named[event.key]) return named[event.key];
  if (event.key.length === 1) return event.key.toUpperCase();
  return event.key;
}

function resetShortcut() {
  settings.value.global_shortcut = 'CommandOrControl+Shift+V';
  shortcutRecording.value = false;
}

function displayShortcut(shortcut: string) {
  return shortcut
    .split('+')
    .filter(Boolean)
    .map((part) => {
      if (part === 'CommandOrControl') return isMacPlatform ? 'Command' : 'Ctrl';
      if (part === 'Control') return 'Ctrl';
      if (part === 'Super') return isMacPlatform ? 'Command' : 'Win';
      if (part === 'Alt') return isMacPlatform ? 'Option' : 'Alt';
      return part;
    })
    .join('+');
}

async function togglePaused() {
  pauseSaving.value = true;
  errorMessage.value = '';
  const next = !settings.value.paused;
  const wasClean = settingsSnapshot(settingsForSave()) === lastSavedSettingsSnapshot;
  try {
    await invoke('set_paused', { paused: next });
    settings.value.paused = next;
    if (wasClean) rememberPersistedSettings();
    showNotice('success', next ? '已暂停监听' : '已恢复监听');
  } catch (err) {
    errorMessage.value = messageFromError(err, '切换监听状态失败');
    showNotice('error', errorMessage.value);
  } finally {
    pauseSaving.value = false;
  }
}

async function syncNow() {
  if (settings.value.offline_mode) return;
  syncing.value = true;
  errorMessage.value = '';
  try {
    online.value = await invoke<boolean>('force_sync');
    await loadHistory();
    showNotice(online.value ? 'success' : 'error', online.value ? '同步完成' : '同步未完成，请检查连接配置');
  } catch (err) {
    online.value = false;
    errorMessage.value = messageFromError(err, '同步失败');
    showNotice('error', errorMessage.value);
  } finally {
    syncing.value = false;
  }
}

async function clearLocalHistory() {
  const confirmed = await confirm('确认清空桌面端本地历史？这不会删除服务端记录。', {
    title: '清空本地历史',
    type: 'warning'
  });
  if (!confirmed) return;
  clearingHistory.value = true;
  errorMessage.value = '';
  try {
    await invoke('clear_history');
    history.value = [];
    clearImagePreviews();
    showNotice('success', '本地历史已清空');
  } catch (err) {
    errorMessage.value = messageFromError(err, '清空失败');
    showNotice('error', errorMessage.value);
  } finally {
    clearingHistory.value = false;
  }
}

async function copy(id: string) {
  copyingId.value = id;
  errorMessage.value = '';
  try {
    await invoke('recopy_item', { id });
    showNotice('success', '已复制到剪切板');
    if (!isQuickWindow) await hideMainWindow();
  } catch (err) {
    errorMessage.value = messageFromError(err, '复制失败');
    showNotice('error', errorMessage.value);
  } finally {
    copyingId.value = '';
  }
}

function openContextMenu(event: MouseEvent, item: LocalItem | null = null) {
  contextMenu.open = true;
  contextMenu.item = item;
  contextMenu.x = Math.min(event.clientX, window.innerWidth - 190);
  contextMenu.y = Math.min(event.clientY, window.innerHeight - 138);
}

function closeContextMenu() {
  contextMenu.open = false;
}

async function copyFromContext() {
  if (!contextMenu.item) return;
  const id = contextMenu.item.id;
  closeContextMenu();
  await copy(id);
}

function runFirstCommand() {
  const first = visibleCommands.value[0];
  if (first) runCommand(first.id);
}

function runCommand(id: CommandId) {
  commandOpen.value = false;
  switch (id) {
    case 'history':
      view.value = 'history';
      break;
    case 'settings':
      view.value = 'settings';
      break;
    case 'refresh':
      void loadHistory();
      break;
    case 'sync':
      void syncNow();
      break;
    case 'pause':
      void togglePaused();
      break;
    case 'theme':
      setUiThemePreference(resolvedUiTheme.value === 'white' ? 'black' : 'white');
      break;
  }
}

function label(type: string) {
  return type === 'image' ? '图片' : type === 'file_path' ? '路径' : '文本';
}

function typeIcon(type: string) {
  return type === 'image' ? ImageIcon : type === 'file_path' ? FileIcon : FileText;
}

function typeIconClass(type: string) {
  if (type === 'image') return 'type-sky';
  if (type === 'file_path') return 'type-violet';
  return 'type-teal';
}

function preview(item: LocalItem) {
  if (item.item_type === 'image') return item.object_path ? '图片预览不可用' : '图片文件缺失';
  const value = item.content || item.file_path || item.object_path || '';
  return value.trim() || '无内容';
}

function refreshImagePreviews() {
  const imageIds = new Set(history.value.filter((item) => item.item_type === 'image').map((item) => item.id));
  imagePreviewUrls.value = new Map([...imagePreviewUrls.value].filter(([id]) => imageIds.has(id)));
  imagePreviewFailedIds.value = new Set([...imagePreviewFailedIds.value].filter((id) => imageIds.has(id)));
  for (const item of history.value) {
    if (item.item_type === 'image') loadImagePreview(item);
  }
}

async function loadImagePreview(item: LocalItem) {
  if (imagePreviewUrls.value.has(item.id) || imagePreviewFailedIds.value.has(item.id) || imagePreviewLoadingIds.has(item.id)) return;
  imagePreviewLoadingIds.add(item.id);
  imagePreviewQueue = imagePreviewQueue.finally(async () => {
    if (!history.value.some((current) => current.id === item.id)) {
      imagePreviewLoadingIds.delete(item.id);
      return;
    }
    try {
      const dataUrl = await invoke<string>('get_image_preview_data_url', { id: item.id });
      const next = new Map(imagePreviewUrls.value);
      next.set(item.id, dataUrl);
      imagePreviewUrls.value = next;
    } catch {
      const failed = new Set(imagePreviewFailedIds.value);
      failed.add(item.id);
      imagePreviewFailedIds.value = failed;
    } finally {
      imagePreviewLoadingIds.delete(item.id);
    }
  });
  await imagePreviewQueue;
}

function imagePreviewUrl(item: LocalItem) {
  return imagePreviewUrls.value.get(item.id) || '';
}

function imagePreviewText(item: LocalItem) {
  if (imagePreviewFailedIds.value.has(item.id)) return item.object_path ? '图片预览失败' : '图片文件缺失';
  return '图片加载中';
}

function clearImagePreviews() {
  imagePreviewUrls.value = new Map();
  imagePreviewFailedIds.value = new Set();
  imagePreviewLoadingIds.clear();
  imagePreviewQueue = Promise.resolve();
}

function formatDate(value: string) {
  if (!value) return '-';
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  }).format(new Date(value));
}

function lines(value: string) {
  return value.split(/\r?\n/).map((item) => item.trim()).filter(Boolean);
}

function normalizeSettings(next: ClientSettings): ClientSettings {
  return {
    ...next,
    api_base: defaultApiBase,
    refresh_token: next.refresh_token || '',
    account_name: next.account_name || '',
    offline_mode: Boolean(next.offline_mode),
    start_on_login: Boolean(next.start_on_login)
  };
}

function settingsForSave(next: ClientSettings = settings.value): ClientSettings {
  return normalizeSettings({
    ...next,
    poll_interval_ms: Math.max(300, Number(next.poll_interval_ms) || 800)
  });
}

function settingsSnapshot(next: ClientSettings) {
  return JSON.stringify(next);
}

function rememberPersistedSettings(next: ClientSettings = settings.value) {
  lastSavedSettingsSnapshot = settingsSnapshot(settingsForSave(next));
}

function syncTextDraftsFromSettings() {
  syncingTextDrafts = true;
  appRulesDraft.value = settings.value.app_rules.join('\n');
  maskRulesDraft.value = settings.value.mask_rules.join('\n');
  webhookDraft.value = settings.value.webhook_urls.join('\n');
  queueMicrotask(() => {
    syncingTextDrafts = false;
  });
}

function applyPersistedSettings(next: ClientSettings) {
  settings.value = normalizeSettings(next);
  syncTextDraftsFromSettings();
  rememberPersistedSettings();
}

function showNotice(kind: Notice['kind'], text: string) {
  window.clearTimeout(noticeTimer);
  notice.value = { kind, text };
  noticeTimer = window.setTimeout(() => {
    notice.value = null;
  }, 2600);
}

function messageFromError(err: unknown, fallback: string) {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === 'string' && err) return err;
  return fallback;
}
</script>
