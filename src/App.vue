<template>
  <main class="h-screen overflow-hidden bg-[#f5f6f7] text-slate-900" :class="themeClass">
    <section v-if="isQuickWindow" class="flex h-screen flex-col overflow-hidden border border-slate-200 bg-white text-slate-900 shadow-xl" :class="themeClass">
      <header class="shrink-0 border-b border-slate-200 bg-white px-3 py-3">
        <div class="mb-3 flex items-center justify-between gap-3">
          <div class="flex min-w-0 items-center gap-2">
            <div class="grid h-8 w-8 place-items-center rounded-md bg-slate-950 text-[11px] font-bold text-white">WP</div>
            <div class="min-w-0">
              <div class="truncate text-sm font-bold text-slate-950">剪切板历史</div>
              <div class="truncate text-[11px] font-semibold text-slate-500">{{ statusLabel }}</div>
            </div>
          </div>
          <button class="grid h-8 w-8 place-items-center rounded-full border border-slate-200 bg-white text-slate-600 hover:bg-slate-50 disabled:opacity-60" type="button" title="刷新" :disabled="historyLoading" @click="loadHistory">
            <RefreshCw :size="15" :class="{ 'animate-spin': historyLoading }" />
          </button>
        </div>
        <div class="flex items-center gap-2">
          <div class="flex min-w-0 flex-1 items-center gap-2 rounded-lg border border-slate-200 bg-slate-50 px-3">
            <Search :size="16" class="shrink-0 text-slate-400" />
            <input v-model.trim="query" class="h-9 min-w-0 flex-1 border-0 bg-transparent text-sm font-medium outline-none placeholder:text-slate-400" placeholder="搜索本地历史" @input="scheduleHistoryLoad" />
            <button v-if="query" class="grid h-5 w-5 shrink-0 place-items-center rounded-full bg-slate-200 text-slate-500 hover:bg-slate-300 hover:text-slate-800" type="button" title="清空搜索" aria-label="清空搜索" @click="clearDesktopSearch">
              <X :size="12" />
            </button>
          </div>
          <button class="grid h-9 w-9 place-items-center rounded-lg bg-slate-950 text-white hover:bg-slate-800 disabled:opacity-60" type="button" title="搜索" :disabled="historyLoading" @click="loadHistory">
            <Search :size="16" />
          </button>
        </div>
      </header>

      <section v-if="!initialized" class="grid min-h-0 flex-1 place-items-center bg-[#f6f7f8]">
        <span class="inline-flex items-center gap-2 rounded-md border border-slate-200 bg-white px-4 py-3 text-sm font-semibold text-slate-600 shadow-sm">
          <LoaderCircle :size="17" class="animate-spin text-blue-600" />
          加载中
        </span>
      </section>
      <section v-else-if="needsOnboarding" class="grid min-h-0 flex-1 place-items-center bg-[#f6f7f8] p-5 text-center">
        <div class="grid gap-3">
          <Cloud :size="26" class="mx-auto text-slate-400" />
          <div class="text-sm font-bold text-slate-950">未登录同步</div>
          <button class="h-9 rounded-md bg-slate-950 px-4 text-sm font-bold text-white" type="button" @click="openMainFromQuick">打开主界面</button>
        </div>
      </section>
      <section v-else class="scrollbar-none min-h-0 flex-1 overflow-y-auto bg-[#f6f7f8] p-2">
        <button
          v-for="item in filteredHistory"
          :key="item.id"
          class="mb-2 grid w-full grid-cols-[34px_minmax(0,1fr)_auto] items-start gap-2 rounded-lg border border-slate-200 bg-white p-2.5 text-left shadow-sm hover:border-slate-300 hover:bg-slate-50"
          type="button"
          @click="quickCopy(item.id)"
        >
          <span :class="typeIconClass(item.item_type)" class="grid h-8 w-8 place-items-center rounded-md">
            <component :is="typeIcon(item.item_type)" :size="16" />
          </span>
          <span class="min-w-0">
            <span class="mb-0.5 flex min-w-0 items-center gap-2">
              <strong class="text-xs font-bold text-slate-950">{{ label(item.item_type) }}</strong>
              <span class="truncate text-[11px] font-semibold text-slate-400">{{ item.source_app || 'Unknown' }}</span>
            </span>
            <span class="line-clamp-2 whitespace-pre-wrap break-words text-xs leading-5 text-slate-600">{{ preview(item) }}</span>
          </span>
          <span class="text-[11px] font-medium text-slate-400">{{ formatDate(item.created_at) }}</span>
        </button>
        <div v-if="historyLoading" class="grid min-h-28 place-items-center rounded-lg border border-slate-200 bg-white text-sm font-semibold text-slate-500">
          <span class="inline-flex items-center gap-2">
            <LoaderCircle :size="16" class="animate-spin" />
            加载中
          </span>
        </div>
        <div v-else-if="filteredHistory.length === 0" class="grid min-h-28 place-items-center rounded-lg border border-dashed border-slate-300 bg-white text-sm font-semibold text-slate-500">
          暂无记录
        </div>
      </section>
    </section>

    <template v-else>
    <section v-if="!initialized" class="grid min-h-screen place-items-center">
      <div class="inline-flex items-center gap-2 rounded-md border border-slate-200 bg-white px-4 py-3 text-sm font-semibold text-slate-600 shadow-sm">
        <LoaderCircle :size="18" class="animate-spin text-blue-600" />
        加载中
      </div>
    </section>

    <section v-else-if="needsOnboarding" class="grid min-h-screen place-items-center px-8 py-6">
      <div class="grid w-full max-w-[980px] overflow-hidden rounded-lg border border-slate-200 bg-white shadow-sm">
        <div class="flex h-12 items-center justify-between border-b border-slate-200 bg-slate-50 px-4">
          <div class="text-sm font-bold text-slate-700">Web Paste Desktop</div>
          <div class="w-[52px]"></div>
        </div>

        <div class="grid grid-cols-[minmax(0,1fr)_360px]">
          <form class="grid gap-5 p-8" @submit.prevent="login">
            <div>
              <h1 class="text-2xl font-bold text-slate-950">登录同步</h1>
              <p class="mt-2 text-sm font-medium text-slate-500">账号密码登录，设备会自动绑定。</p>
            </div>

            <div class="grid gap-3">
              <label class="grid gap-1.5">
                <span class="text-xs font-bold text-slate-500">用户名</span>
                <input v-model.trim="loginForm.username" autocomplete="username" class="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm font-medium outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-100" />
              </label>
              <label class="grid gap-1.5">
                <span class="text-xs font-bold text-slate-500">密码</span>
                <input v-model="loginForm.password" autocomplete="current-password" type="password" class="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm font-medium outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-100" />
              </label>
            </div>

            <p v-if="errorMessage" class="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm font-semibold text-red-700">{{ errorMessage }}</p>

            <div class="flex gap-2">
              <button class="inline-flex h-10 flex-1 items-center justify-center gap-2 rounded-md bg-slate-950 px-4 text-sm font-bold text-white hover:bg-slate-800 disabled:opacity-60" type="submit" :disabled="authLoading">
                <LoaderCircle v-if="authLoading" :size="17" class="animate-spin" />
                <LogIn v-else :size="17" />
                登录
              </button>
              <button class="inline-flex h-10 items-center justify-center gap-2 rounded-md border border-slate-200 bg-white px-4 text-sm font-bold text-slate-700 hover:bg-slate-50 disabled:opacity-60" type="button" :disabled="authLoading" @click="enterOfflineMode">
                <HardDrive :size="17" />
                离线模式
              </button>
            </div>

            <div class="flex flex-wrap gap-3 border-t border-slate-100 pt-4 text-sm font-bold">
              <button class="bg-transparent p-0 text-blue-700 hover:text-blue-900 disabled:opacity-60" type="button" :disabled="authLoading" @click="openWebAuth('register')">
                注册
              </button>
              <button class="bg-transparent p-0 text-slate-600 hover:text-slate-950 disabled:opacity-60" type="button" :disabled="authLoading" @click="openWebAuth('forgot-password')">
                忘记密码
              </button>
            </div>
          </form>

          <aside class="border-l border-slate-200 bg-slate-50 p-6">
            <div class="mb-5 grid h-12 w-12 place-items-center rounded-lg bg-slate-950 text-sm font-bold text-white">WP</div>
            <div class="grid gap-3">
              <div class="rounded-lg border border-slate-200 bg-white p-4">
                <ClipboardList :size="18" class="mb-3 text-blue-600" />
                <div class="text-sm font-bold text-slate-900">本地历史</div>
                <div class="mt-1 text-xs font-medium text-slate-500">文本、图片、路径</div>
              </div>
              <div class="rounded-lg border border-slate-200 bg-white p-4">
                <ShieldCheck :size="18" class="mb-3 text-emerald-600" />
                <div class="text-sm font-bold text-slate-900">隐私规则</div>
                <div class="mt-1 text-xs font-medium text-slate-500">应用名单、正则脱敏</div>
              </div>
              <div class="rounded-lg border border-slate-200 bg-white p-4">
                <Cloud :size="18" class="mb-3 text-violet-600" />
                <div class="text-sm font-bold text-slate-900">云端同步</div>
                <div class="mt-1 text-xs font-medium text-slate-500">登录后静默推送</div>
              </div>
            </div>
          </aside>
        </div>
      </div>
    </section>

    <section v-else class="h-screen overflow-hidden px-5 py-4">
      <div class="mx-auto grid h-[calc(100vh-32px)] max-w-[1240px] grid-cols-[184px_minmax(0,1fr)] overflow-hidden rounded-lg border border-slate-200 bg-white shadow-sm lg:grid-cols-[220px_minmax(0,1fr)]">
        <aside class="flex min-h-0 flex-col border-r border-slate-200 bg-[#fbfbfc]">
          <div class="border-b border-slate-200 px-4 py-4">
            <div class="flex items-center gap-3">
              <div class="grid h-9 w-9 place-items-center rounded-lg bg-slate-950 text-xs font-bold text-white">WP</div>
              <div class="min-w-0">
                <div class="truncate text-sm font-bold text-slate-950">Web Paste</div>
                <div class="truncate text-xs font-medium text-slate-500">{{ accountLabel }}</div>
              </div>
            </div>
          </div>

          <nav class="grid gap-1 p-3">
            <button class="flex h-10 items-center gap-3 rounded-md px-3 text-sm font-semibold" :class="view === 'history' ? 'bg-white text-slate-950 shadow-sm ring-1 ring-slate-200' : 'text-slate-600 hover:bg-white hover:text-slate-950'" type="button" @click="view = 'history'">
              <ClipboardList :size="17" />
              剪切板历史
            </button>
            <button class="flex h-10 items-center gap-3 rounded-md px-3 text-sm font-semibold" :class="view === 'settings' ? 'bg-white text-slate-950 shadow-sm ring-1 ring-slate-200' : 'text-slate-600 hover:bg-white hover:text-slate-950'" type="button" @click="view = 'settings'">
              <Settings :size="17" />
              设置
            </button>
          </nav>

          <div class="mt-auto border-t border-slate-200 p-4">
            <div class="mb-3 rounded-lg border border-slate-200 bg-white p-3">
              <div class="mb-2 flex items-center gap-2">
                <span :class="statusDotClass" class="h-2.5 w-2.5 rounded-full"></span>
                <span class="text-sm font-bold text-slate-900">{{ statusLabel }}</span>
              </div>
              <div class="truncate text-xs font-medium text-slate-500">{{ settings.device_name || 'Desktop' }}</div>
            </div>
            <button class="flex h-10 w-full items-center justify-center gap-2 rounded-md border border-slate-200 bg-white text-sm font-bold text-slate-700 hover:bg-slate-50 disabled:opacity-60" type="button" :disabled="pauseSaving" @click="togglePaused">
              <LoaderCircle v-if="pauseSaving" :size="16" class="animate-spin" />
              <Play v-else-if="paused" :size="16" />
              <Pause v-else :size="16" />
              {{ paused ? '恢复监听' : '暂停监听' }}
            </button>
          </div>
        </aside>

        <div class="flex min-h-0 min-w-0 flex-col overflow-hidden bg-[#f6f7f8]">
          <header class="flex h-[64px] shrink-0 items-center justify-between border-b border-slate-200 bg-white px-5">
            <div>
              <h1 class="text-base font-bold text-slate-950">{{ view === 'history' ? '剪切板历史' : '设置' }}</h1>
              <p class="text-xs font-medium text-slate-500">{{ displayShortcut(settings.global_shortcut) }}</p>
            </div>
            <div class="flex items-center gap-2">
              <button v-if="view === 'history'" class="grid h-9 w-9 place-items-center rounded-md border border-slate-200 bg-white text-slate-600 hover:bg-slate-50 disabled:opacity-60" type="button" :disabled="historyLoading" @click="loadHistory" title="刷新">
                <RefreshCw :size="16" :class="{ 'animate-spin': historyLoading }" />
              </button>
              <button v-if="view === 'history'" class="grid h-9 w-9 place-items-center rounded-md border border-red-200 bg-red-50 text-red-700 hover:bg-red-100 disabled:opacity-60" type="button" :disabled="clearingHistory || history.length === 0" @click="clearLocalHistory" title="清空本地历史">
                <LoaderCircle v-if="clearingHistory" :size="16" class="animate-spin" />
                <Trash2 v-else :size="16" />
              </button>
              <button class="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 bg-white px-3 text-sm font-bold text-slate-700 hover:bg-slate-50 disabled:opacity-60" type="button" :disabled="syncing || settings.offline_mode" @click="syncNow">
                <LoaderCircle v-if="syncing" :size="16" class="animate-spin" />
                <RefreshCw v-else :size="16" />
                同步
              </button>
            </div>
          </header>

          <div v-if="errorMessage" class="mx-5 mt-4 flex shrink-0 items-center gap-3 rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm font-semibold text-red-800">
            <AlertCircle :size="17" />
            <span class="min-w-0 flex-1">{{ errorMessage }}</span>
            <button class="grid h-7 w-7 place-items-center rounded-md text-red-900 hover:bg-red-100" type="button" @click="errorMessage = ''">
              <X :size="15" />
            </button>
          </div>

          <section v-if="view === 'history'" class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)] gap-4 overflow-hidden p-4 lg:p-5 xl:grid-cols-[minmax(0,1fr)_260px]">
            <div class="flex min-h-0 flex-col gap-3">
                <div class="flex items-center gap-3">
                <div class="flex min-w-0 flex-1 items-center gap-2 rounded-lg border border-slate-200 bg-white px-3 shadow-sm">
                  <Search :size="17" class="shrink-0 text-slate-400" />
                  <input v-model.trim="query" class="h-10 min-w-0 flex-1 border-0 bg-transparent text-sm font-medium outline-none placeholder:text-slate-400" placeholder="搜索本地历史" @input="scheduleHistoryLoad" />
                  <button v-if="query" class="grid h-5 w-5 shrink-0 place-items-center rounded-full bg-slate-200 text-slate-500 hover:bg-slate-300 hover:text-slate-800" type="button" title="清空搜索" aria-label="清空搜索" @click="clearDesktopSearch">
                    <X :size="12" />
                  </button>
                </div>
                <select v-model="typeFilter" class="h-10 w-28 rounded-lg border border-slate-200 bg-white px-3 text-sm font-semibold text-slate-700 shadow-sm outline-none">
                  <option value="">全部</option>
                  <option value="text">文本</option>
                  <option value="image">图片</option>
                  <option value="file_path">路径</option>
                </select>
              </div>

              <div class="scrollbar-none min-h-0 flex-1 overflow-y-auto overflow-x-hidden pr-1">
                <div class="grid gap-2">
                  <article v-for="item in filteredHistory" :key="item.id" class="overflow-hidden rounded-lg border border-slate-200 bg-white p-3 shadow-sm transition hover:border-slate-300">
                    <div class="grid min-w-0 grid-cols-[40px_minmax(0,1fr)_auto] items-start gap-3">
                      <div :class="typeIconClass(item.item_type)" class="grid h-10 w-10 shrink-0 place-items-center rounded-md">
                        <component :is="typeIcon(item.item_type)" :size="18" />
                      </div>
                      <div class="min-w-0 flex-1">
                        <div class="mb-1 flex min-w-0 items-center gap-2">
                          <strong class="text-sm font-bold text-slate-950">{{ label(item.item_type) }}</strong>
                          <span :class="item.synced ? 'bg-emerald-50 text-emerald-700' : 'bg-amber-50 text-amber-700'" class="rounded-full px-2 py-0.5 text-[11px] font-bold">
                            {{ item.synced ? '已同步' : '待同步' }}
                          </span>
                          <span class="truncate text-xs font-medium text-slate-400">{{ item.source_app || 'Unknown' }}</span>
                        </div>
                        <div v-if="item.item_type === 'image'" class="mt-2 flex h-24 w-40 items-center justify-center overflow-hidden rounded-md border border-slate-200 bg-slate-50">
                          <img v-if="imagePreviewUrl(item)" :src="imagePreviewUrl(item)" alt="" class="h-full w-full object-contain" loading="lazy" />
                          <span v-else class="px-3 text-center text-xs font-semibold text-slate-400">{{ imagePreviewText(item) }}</span>
                        </div>
                        <pre v-else class="max-h-24 overflow-hidden whitespace-pre-wrap break-words text-sm leading-6 text-slate-700">{{ preview(item) }}</pre>
                      </div>
                      <div class="flex min-w-8 shrink-0 flex-col items-end gap-2">
                        <span class="hidden text-xs font-medium text-slate-400 lg:block">{{ formatDate(item.created_at) }}</span>
                        <button class="grid h-8 w-8 place-items-center rounded-md bg-slate-950 text-white hover:bg-slate-800 disabled:opacity-60 lg:inline-flex lg:w-auto lg:gap-2 lg:px-3 lg:text-xs lg:font-bold" type="button" title="复制" :disabled="copyingId === item.id" @click="copy(item.id)">
                          <LoaderCircle v-if="copyingId === item.id" :size="14" class="animate-spin" />
                          <Copy v-else :size="14" />
                          <span class="hidden lg:inline">复制</span>
                        </button>
                      </div>
                    </div>
                  </article>

                  <div v-if="historyLoading" class="grid min-h-48 place-items-center rounded-lg border border-slate-200 bg-white text-sm font-semibold text-slate-500">
                    <span class="inline-flex items-center gap-2">
                      <LoaderCircle :size="17" class="animate-spin" />
                      加载中
                    </span>
                  </div>
                  <div v-else-if="filteredHistory.length === 0" class="grid min-h-48 place-items-center rounded-lg border border-dashed border-slate-300 bg-white text-sm font-semibold text-slate-500">
                    暂无本地记录
                  </div>
                </div>
              </div>
            </div>

            <aside class="hidden content-start gap-3 xl:grid">
              <section class="rounded-lg border border-slate-200 bg-white p-4 shadow-sm">
                <div class="mb-3 text-xs font-bold text-slate-500">概览</div>
                <div class="grid gap-3">
                  <div class="flex items-center justify-between">
                    <span class="inline-flex items-center gap-2 text-sm font-semibold text-slate-600"><ClipboardList :size="16" />本地记录</span>
                    <strong class="text-lg font-bold text-slate-950">{{ history.length }}</strong>
                  </div>
                  <div class="flex items-center justify-between">
                    <span class="inline-flex items-center gap-2 text-sm font-semibold text-slate-600"><CheckCircle2 :size="16" />已同步</span>
                    <strong class="text-lg font-bold text-emerald-700">{{ syncedCount }}</strong>
                  </div>
                  <div class="flex items-center justify-between">
                    <span class="inline-flex items-center gap-2 text-sm font-semibold text-slate-600"><Database :size="16" />待同步</span>
                    <strong class="text-lg font-bold text-amber-700">{{ pendingCount }}</strong>
                  </div>
                </div>
              </section>

              <section class="rounded-lg border border-slate-200 bg-white p-4 shadow-sm">
                <div class="mb-3 text-xs font-bold text-slate-500">模式</div>
                <div class="flex items-center gap-3">
                  <div class="grid h-10 w-10 place-items-center rounded-md bg-slate-100 text-slate-700">
                    <HardDrive :size="18" />
                  </div>
                  <div>
                    <div class="text-sm font-bold text-slate-950">{{ settings.offline_mode ? '离线模式' : '云端同步' }}</div>
                    <div class="text-xs font-medium text-slate-500">{{ statusLabel }}</div>
                  </div>
                </div>
              </section>
            </aside>
          </section>

          <section v-else class="scrollbar-none min-h-0 flex-1 overflow-y-auto overflow-x-hidden p-5">
            <div class="mx-auto grid max-w-[920px] gap-4">
              <section class="rounded-lg border border-slate-200 bg-white p-5 shadow-sm">
                <div class="mb-4 flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <Cloud :size="18" class="text-blue-600" />
                    <h2 class="text-sm font-bold text-slate-950">账号</h2>
                  </div>
                  <span :class="settings.offline_mode ? 'bg-slate-100 text-slate-700' : 'bg-emerald-50 text-emerald-700'" class="rounded-full px-2 py-1 text-[11px] font-bold">
                    {{ settings.offline_mode ? '离线模式' : '已登录' }}
                  </span>
                </div>
                <div class="flex items-center justify-between gap-3 rounded-lg border border-slate-200 bg-slate-50 p-3">
                  <div class="min-w-0">
                    <div class="text-xs font-bold text-slate-500">当前账号</div>
                    <div class="mt-1 truncate text-sm font-bold text-slate-950">{{ accountLabel }}</div>
                  </div>
                  <div class="flex shrink-0 gap-2">
                    <button class="inline-flex h-9 items-center gap-2 rounded-md border border-slate-200 bg-white px-3 text-sm font-bold text-slate-700 hover:bg-slate-50" type="button" @click="returnToLogin">
                      <LogIn :size="16" />
                      {{ settings.offline_mode ? '登录同步' : '切换账号' }}
                    </button>
                    <button v-if="!settings.offline_mode" class="inline-flex h-9 items-center gap-2 rounded-md border border-red-200 bg-red-50 px-3 text-sm font-bold text-red-700 hover:bg-red-100" type="button" @click="logout">
                      <LogOut :size="16" />
                      退出
                    </button>
                  </div>
                </div>
              </section>

              <section class="rounded-lg border border-slate-200 bg-white p-5 shadow-sm">
                <div class="mb-4 flex items-center gap-2">
                  <Monitor :size="18" class="text-slate-700" />
                  <h2 class="text-sm font-bold text-slate-950">外观</h2>
                </div>
                <div class="grid gap-1.5">
                  <span class="text-xs font-bold text-slate-500">主题</span>
                  <div class="grid grid-cols-2 gap-2">
                    <button
                      class="inline-flex h-10 items-center justify-center gap-2 rounded-md border px-3 text-sm font-bold"
                      :class="uiTheme === 'white' ? 'border-slate-950 bg-slate-950 text-white' : 'border-slate-200 bg-white text-slate-700 hover:bg-slate-50'"
                      type="button"
                      @click="setUiTheme('white')"
                    >
                      <Sun :size="16" />
                      白色
                    </button>
                    <button
                      class="inline-flex h-10 items-center justify-center gap-2 rounded-md border px-3 text-sm font-bold"
                      :class="uiTheme === 'black' ? 'border-slate-950 bg-slate-950 text-white' : 'border-slate-200 bg-white text-slate-700 hover:bg-slate-50'"
                      type="button"
                      @click="setUiTheme('black')"
                    >
                      <Moon :size="16" />
                      黑色
                    </button>
                  </div>
                </div>
              </section>

              <section class="rounded-lg border border-slate-200 bg-white p-5 shadow-sm">
                <div class="mb-4 flex items-center gap-2">
                  <Monitor :size="18" class="text-violet-600" />
                  <h2 class="text-sm font-bold text-slate-950">设备</h2>
                </div>
                <div class="grid gap-3">
                  <label class="grid gap-1.5">
                    <span class="text-xs font-bold text-slate-500">设备名</span>
                    <input v-model.trim="settings.device_name" class="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm font-medium outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-100" />
                  </label>
                  <div class="grid gap-1.5">
                    <span class="text-xs font-bold text-slate-500">快捷键</span>
                    <div class="flex gap-2">
                      <button
                        class="flex h-10 min-w-0 flex-1 items-center justify-between rounded-md border px-3 text-left text-sm font-bold outline-none"
                        :class="shortcutRecording ? 'border-blue-500 bg-blue-50 text-blue-700 ring-2 ring-blue-100' : 'border-slate-200 bg-white text-slate-800 hover:bg-slate-50'"
                        type="button"
                        @click="beginShortcutRecording"
                        @keydown.prevent.stop="recordShortcut"
                      >
                        <span class="truncate">{{ shortcutRecording ? '按下组合键' : displayShortcut(settings.global_shortcut) }}</span>
                        <span class="text-xs font-bold text-slate-400">录制</span>
                      </button>
                      <button class="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm font-bold text-slate-600 hover:bg-slate-50" type="button" @click="resetShortcut">默认</button>
                    </div>
                  </div>
                  <label class="flex items-center justify-between gap-3 rounded-lg border border-slate-200 bg-slate-50 p-3">
                    <span class="min-w-0">
                      <span class="block text-sm font-bold text-slate-950">登录时启动</span>
                      <span class="mt-1 block text-xs font-medium text-slate-500">开机登录系统后自动启动 Web Paste</span>
                    </span>
                    <input v-model="settings.start_on_login" class="h-4 w-4 shrink-0 accent-slate-950" type="checkbox" />
                  </label>
                </div>
              </section>

              <section class="rounded-lg border border-slate-200 bg-white p-5 shadow-sm">
                <div class="mb-4 flex items-center gap-2">
                  <ShieldCheck :size="18" class="text-emerald-600" />
                  <h2 class="text-sm font-bold text-slate-950">隐私策略</h2>
                </div>
                <div class="grid gap-3">
                  <label class="grid gap-1.5">
                    <span class="text-xs font-bold text-slate-500">应用策略</span>
                    <select v-model="settings.privacy_mode" class="h-10 rounded-md border border-slate-200 bg-white px-3 text-sm font-semibold outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-100">
                      <option value="off">关闭</option>
                      <option value="blacklist">黑名单</option>
                      <option value="whitelist">白名单</option>
                    </select>
                  </label>
                  <label class="grid gap-1.5">
                    <span class="text-xs font-bold text-slate-500">应用列表</span>
                    <textarea v-model="appRulesDraft" rows="3" class="resize-y rounded-md border border-slate-200 bg-white p-3 text-sm font-medium leading-5 outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-100" @blur="applyTextSettingsNow" />
                  </label>
                  <label class="grid gap-1.5">
                    <span class="text-xs font-bold text-slate-500">脱敏正则</span>
                    <textarea v-model="maskRulesDraft" rows="3" class="resize-y rounded-md border border-slate-200 bg-white p-3 font-mono text-xs leading-5 outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-100" @blur="applyTextSettingsNow" />
                    <span class="text-xs font-medium leading-5 text-slate-500">每行一条正则；命中内容会替换为等长星号。默认示例用于手机号脱敏。</span>
                  </label>
                </div>
              </section>

              <section class="rounded-lg border border-slate-200 bg-white p-5 shadow-sm">
                <div class="mb-4 flex items-center gap-2">
                  <Webhook :size="18" class="text-amber-600" />
                  <h2 class="text-sm font-bold text-slate-950">Webhook</h2>
                </div>
                <textarea v-model="webhookDraft" rows="4" class="w-full resize-y rounded-md border border-slate-200 bg-white p-3 text-sm font-medium leading-5 outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-100" @blur="applyTextSettingsNow" />
              </section>
            </div>
          </section>
        </div>
      </div>
    </section>
    </template>

    <div v-if="notice" class="fixed bottom-5 right-5 z-50 flex max-w-[420px] items-center gap-3 rounded-lg px-4 py-3 text-sm font-semibold text-white shadow-xl" :class="notice.kind === 'success' ? 'bg-emerald-700' : 'bg-red-700'">
      <CheckCircle2 v-if="notice.kind === 'success'" :size="17" />
      <AlertCircle v-else :size="17" />
      <span class="min-w-0 flex-1">{{ notice.text }}</span>
      <button class="grid h-6 w-6 place-items-center rounded-md bg-white/15" type="button" @click="notice = null">
        <X :size="14" />
      </button>
    </div>
  </main>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { confirm } from '@tauri-apps/api/dialog';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/shell';
import { invoke } from '@tauri-apps/api/tauri';
import {
  AlertCircle,
  CheckCircle2,
  ClipboardList,
  Cloud,
  Copy,
  Database,
  File as FileIcon,
  FileText,
  HardDrive,
  Image as ImageIcon,
  LoaderCircle,
  LogIn,
  LogOut,
  Monitor,
  Moon,
  Pause,
  Play,
  RefreshCw,
  Search,
  Settings,
  ShieldCheck,
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

const defaultApiBase = 'https://paste-api.dangolabs.top';
const defaultWebBase = (import.meta.env.VITE_WEB_BASE_URL || 'https://paste.dangolabs.top').replace(/\/$/, '');
const uiThemeStorageKey = 'web-paste-ui-theme';
const isQuickWindow = new URLSearchParams(window.location.search).get('quick') === '1';

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
const uiTheme = ref<UiTheme>(readStoredUiTheme());
const shortcutRecording = ref(false);
const appRulesDraft = ref('');
const maskRulesDraft = ref('');
const webhookDraft = ref('');
const imagePreviewUrls = ref(new Map<string, string>());
const imagePreviewFailedIds = ref(new Set<string>());
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
const isMacPlatform = /mac/i.test(navigator.platform);

const paused = computed(() => settings.value.paused);
const isSignedIn = computed(() => Boolean(settings.value.token && settings.value.device_id));
const needsOnboarding = computed(() => !settings.value.offline_mode && !isSignedIn.value);
const pendingCount = computed(() => history.value.filter((item) => !item.synced).length);
const syncedCount = computed(() => history.value.filter((item) => item.synced).length);
const themeClass = computed(() => `theme-${uiTheme.value}`);
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

function readStoredUiTheme(): UiTheme {
  try {
    return window.localStorage.getItem(uiThemeStorageKey) === 'black' ? 'black' : 'white';
  } catch {
    return 'white';
  }
}

function setUiTheme(next: UiTheme) {
  uiTheme.value = next;
  try {
    window.localStorage.setItem(uiThemeStorageKey, next);
  } catch {
    // Theme persistence is optional; the in-memory selection still applies.
  }
}

onMounted(async () => {
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
  unlistenHistoryChanged?.();
  unlistenHistoryChanged = null;
  unlistenShowMain?.();
  unlistenShowMain = null;
  clearImagePreviews();
});

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
    if (!isQuickWindow) {
      try {
        await invoke('hide_main_window');
      } catch (err) {
        console.warn('hide main window failed', err);
      }
    }
  } catch (err) {
    errorMessage.value = messageFromError(err, '复制失败');
    showNotice('error', errorMessage.value);
  } finally {
    copyingId.value = '';
  }
}

function label(type: string) {
  return type === 'image' ? '图片' : type === 'file_path' ? '路径' : '文本';
}

function typeIcon(type: string) {
  return type === 'image' ? ImageIcon : type === 'file_path' ? FileIcon : FileText;
}

function typeIconClass(type: string) {
  if (type === 'image') return 'bg-sky-50 text-sky-700';
  if (type === 'file_path') return 'bg-violet-50 text-violet-700';
  return 'bg-teal-50 text-teal-700';
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
