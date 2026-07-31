<template>
  <div class="app-container">
    <!-- 顶部导航 -->
    <header class="app-header">
      <div class="header-left">
        <div class="logo-icon">🎵</div>
        <h1>抖音采集器</h1>
      </div>
      <div class="header-right">
        <div v-if="loginStatus === 'connected'" class="status-badge connected">
          <span class="dot"></span> 已登录
        </div>
        <div v-else-if="loginStatus === 'connecting'" class="status-badge connecting">
          <span class="dot pulse"></span> 连接中
        </div>
        <div v-else class="status-badge disconnected">
          <span class="dot"></span> 未登录
        </div>
      </div>
    </header>

    <!-- 主区域 -->
    <main class="main-content">
      <!-- 左侧：登录 + 配置 -->
      <aside class="sidebar">
        <!-- 登录卡片 -->
        <div class="card login-card">
          <div class="card-title">
            <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
              <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
            </svg>
            账号登录
          </div>

          <el-tabs v-model="loginTab" class="login-tabs">
            <!-- 扫码登录 -->
            <el-tab-pane label="扫码登录" name="qrcode">
              <div class="qrcode-section">
                <p class="hint">点击按钮在新窗口中打开抖音扫码登录页面</p>
                <div class="qrcode-wrapper">
                  <div class="qrcode-placeholder">
                    <div class="qrcode-dummy-icon">📱</div>
                    <p class="qrcode-tip">使用抖音 APP 扫码</p>
                  </div>
                </div>
                <el-button
                  type="primary"
                  size="large"
                  class="qrcode-btn"
                  @click="handleOpenLogin"
                >
                  打开扫码登录
                </el-button>
              </div>
            </el-tab-pane>

            <!-- Cookie 手动输入 -->
            <el-tab-pane label="Cookie 登录" name="cookie">
              <div class="cookie-section">
                <p class="hint">从浏览器开发者工具复制 Cookie</p>
                <el-input
                  v-model="cookieInput"
                  type="textarea"
                  :rows="4"
                  placeholder="粘贴完整的 Cookie 字符串"
                />
                <el-input
                  v-model="userAgentInput"
                  placeholder="User-Agent（可选，留空自动生成）"
                  class="ua-input"
                />
                <el-button
                  type="primary"
                  size="large"
                  class="cookie-login-btn"
                  @click="handleCookieLogin"
                >
                  登录
                </el-button>
              </div>
            </el-tab-pane>
          </el-tabs>

          <el-divider style="margin:8px 0" />

          <!-- 登录状态 -->
          <div v-if="loginStatus === 'connected'" class="login-success">
            <el-alert title="登录成功" type="success" :closable="false" show-icon>
              <template #default>
                <span>已获取抖音网页版 Cookie，可以开始采集</span>
              </template>
            </el-alert>
          </div>
        </div>

        <!-- 爬取配置 -->
        <div class="card config-card">
          <div class="card-title">
            <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="3"/>
              <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
            </svg>
            采集配置
          </div>

          <!-- 模式选择 -->
          <div class="mode-selector">
            <button
              v-for="m in modes"
              :key="m.key"
              :class="['mode-btn', { active: form.mode === m.key }]"
              @click="form.mode = m.key"
            >
              <span class="mode-icon">{{ m.icon }}</span>
              <span class="mode-label">{{ m.label }}</span>
              <span class="mode-desc">{{ m.desc }}</span>
            </button>
          </div>

          <!-- 搜索模式 -->
          <template v-if="form.mode === 'Search'">
            <div class="field">
              <label>搜索关键词</label>
              <el-input
                v-model="keywordInput"
                placeholder="多个关键词用逗号分隔，如：AI编程,副业"
              />
            </div>
            <div class="field-row">
              <div class="field half">
                <label>排序方式</label>
                <el-select v-model="form.sortType" style="width:100%">
                  <el-option label="综合排序" value="General" />
                  <el-option label="最多点赞" value="MostLike" />
                  <el-option label="最新发布" value="Latest" />
                </el-select>
              </div>
              <div class="field half">
                <label>发布时间</label>
                <el-select v-model="form.publishTime" style="width:100%">
                  <el-option label="不限" value="Unlimited" />
                  <el-option label="一天内" value="OneDay" />
                  <el-option label="一周内" value="OneWeek" />
                  <el-option label="半年内" value="SixMonths" />
                </el-select>
              </div>
            </div>
          </template>

          <!-- 指定视频 -->
          <template v-if="form.mode === 'Detail'">
            <div class="field">
              <label>视频链接（每行一个）</label>
              <el-input
                v-model="videoUrlsInput"
                type="textarea"
                :rows="4"
                placeholder="https://www.douyin.com/video/xxx
https://v.douyin.com/xxx
或直接输入视频ID"
              />
            </div>
          </template>

          <!-- 创作者 -->
          <template v-if="form.mode === 'Creator'">
            <div class="field">
              <label>创作者主页链接</label>
              <el-input
                v-model="creatorUrlsInput"
                type="textarea"
                :rows="3"
                placeholder="https://www.douyin.com/user/MS4wLjABAAAA..."
              />
            </div>
          </template>

          <!-- 通用配置 -->
          <div class="field-row">
            <div class="field half">
              <label>最大视频数</label>
              <el-input-number v-model="form.maxVideos" :min="1" :max="500" style="width:100%" />
            </div>
            <div class="field half">
              <label>请求间隔(秒)</label>
              <el-input-number v-model="form.sleepSecs" :min="0" :max="10" :step="0.5" style="width:100%" />
            </div>
          </div>

          <div class="field-checklist">
            <el-checkbox v-model="form.enableComments">获取评论</el-checkbox>
            <el-checkbox v-model="form.enableSubComments" :disabled="!form.enableComments">二级评论</el-checkbox>
            <el-checkbox v-model="form.enableMedia">下载媒体</el-checkbox>
          </div>

          <div class="field" v-if="form.enableComments">
            <label>每条视频最大评论数</label>
            <el-input-number v-model="form.maxComments" :min="1" :max="500" style="width:100%" />
          </div>

          <div class="field">
            <label>输出目录</label>
            <div class="dir-field">
              <el-input v-model="form.outputDir" readonly class="dir-input" />
              <el-button @click="handlePickDir">
                <el-icon><FolderOpened /></el-icon>
              </el-button>
            </div>
          </div>

          <!-- 开始按钮 -->
          <el-button
            type="primary"
            size="large"
            class="start-btn"
            :disabled="!canStart || isRunning"
            @click="handleStart"
          >
            <el-icon v-if="!isRunning"><VideoPlay /></el-icon>
            <el-icon v-else class="is-loading"><Loading /></el-icon>
            {{ isRunning ? '采集中...' : '开始采集' }}
          </el-button>
          <el-button
            v-if="isRunning"
            type="danger"
            size="large"
            class="stop-btn"
            @click="handleStop"
          >
            停止
          </el-button>
        </div>
      </aside>

      <!-- 右侧：进度与结果 -->
      <section class="main-panel">
        <!-- 进度卡片 -->
        <div class="card progress-card">
          <div class="card-title">
            <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 12a9 9 0 1 1-6.219-8.56"/>
              <polyline points="21 3 21 9 15 9"/>
            </svg>
            采集进度
            <span v-if="progress.status === 'Completed'" class="tag tag-success">完成</span>
            <span v-else-if="isRunning" class="tag tag-running">进行中</span>
            <span v-else class="tag tag-idle">就绪</span>
          </div>

          <div class="progress-stats">
            <div class="stat-card">
              <div class="stat-num">{{ progress.fetched_videos }}</div>
              <div class="stat-label">视频</div>
            </div>
            <div class="stat-card">
              <div class="stat-num">{{ progress.fetched_comments }}</div>
              <div class="stat-label">评论</div>
            </div>
            <div class="stat-card">
              <div class="stat-num">{{ progress.downloaded_media }}</div>
              <div class="stat-label">媒体</div>
            </div>
            <div class="stat-card">
              <div class="stat-num">{{ progress.errors.length }}</div>
              <div class="stat-label">错误</div>
            </div>
          </div>

          <div class="progress-bar-section">
            <div class="progress-label">
              <span>总体进度</span>
              <span>{{ progress.fetched_videos }} / {{ form.maxVideos }}</span>
            </div>
            <div class="progress-track">
              <div class="progress-fill" :style="{ width: videoPercent + '%' }"></div>
            </div>
          </div>

          <div v-if="progress.current_keyword" class="current-kw">
            <el-tag size="small" effect="plain">当前: {{ progress.current_keyword }}</el-tag>
            <el-link
              v-if="progress.output_dir"
              type="primary"
              :underline="false"
              size="small"
              @click="handleOpenDir"
            >
              📂 {{ progress.output_dir }}
            </el-link>
          </div>
        </div>

        <!-- 错误日志 -->
        <div v-if="progress.errors.length > 0" class="card error-card">
          <div class="card-title">
            <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="#f56c6c" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <line x1="12" y1="8" x2="12" y2="12"/>
              <line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            错误日志（{{ progress.errors.length }}）
          </div>
          <div class="error-list">
            <div v-for="(err, i) in progress.errors" :key="i" class="error-item">
              <span class="error-idx">#{{ i + 1 }}</span>
              {{ err }}
            </div>
          </div>
        </div>

        <!-- 空状态 -->
        <div v-if="progress.fetched_videos === 0 && !isRunning" class="empty-state">
          <div class="empty-icon">📊</div>
          <h3>等待开始采集</h3>
          <p>配置好参数后点击"开始采集"</p>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { ElMessage } from "element-plus";
import {
  startCrawl,
  stopCrawl,
  onProgress,
  onError,
  pickDirectory,
  openOutputDir,
  openLoginWindow,
  getLoginCookies,
  type CrawlProgress,
} from "./api";

// ===== 登录相关 =====
const loginTab = ref("qrcode");
const loginStatus = ref<"disconnected" | "connecting" | "connected">("disconnected");
const cookieInput = ref("");
const userAgentInput = ref("");
let cookiePollTimer: ReturnType<typeof setInterval> | null = null;

async function handleOpenLogin() {
  try {
    loginStatus.value = "connecting";
    await openLoginWindow();
    ElMessage.success("请在新窗口中扫码登录");
    
    // 开始轮询 Cookie 状态
    startCookiePolling();
  } catch (e: any) {
    loginStatus.value = "disconnected";
    ElMessage.error(`打开登录窗口失败: ${e}`);
  }
}

function startCookiePolling() {
  if (cookiePollTimer) clearInterval(cookiePollTimer);
  let pollCount = 0;
  const maxPolls = 120; // 最多轮询 120 次 (约 6 分钟)
  
  cookiePollTimer = setInterval(async () => {
    pollCount++;
    if (pollCount > maxPolls) {
      clearInterval(cookiePollTimer!);
      cookiePollTimer = null;
      loginStatus.value = "disconnected";
      return;
    }
    
    try {
      const result = await getLoginCookies();
      // 精确匹配 Cookie 名称，避免 passport_auth_mix_state 误判
      const cookieNames = result.cookies.split(";").map((c: string) => c.trim().split("=")[0]);
      const hasLogin = cookieNames.includes("sessionid") || 
                       cookieNames.includes("sid_tt");
      
      if (hasLogin && result.cookies.length > 50) {
        clearInterval(cookiePollTimer!);
        cookiePollTimer = null;
        cookieInput.value = result.cookies;
        loginStatus.value = "connected";
        ElMessage.success("扫码登录成功！登录窗口保持打开以供搜索使用");
        // 注意：不关闭登录窗口，搜索功能需要用到浏览器环境
      }
    } catch (e) {
      // 窗口可能已关闭
      console.warn("轮询 Cookie 失败:", e);
    }
  }, 3000);
}

async function handleCookieLogin() {
  if (!cookieInput.value.trim()) {
    ElMessage.warning("请输入 Cookie");
    return;
  }
  loginStatus.value = "connected";
  ElMessage.success("Cookie 已保存");
}

// ===== 表单 =====
const form = ref({
  mode: "Search" as "Search" | "Detail" | "Creator",
  maxVideos: 20,
  maxComments: 10,
  enableComments: true,
  enableSubComments: false,
  enableMedia: false,
  sortType: "General",
  publishTime: "Unlimited",
  outputDir: "",
  sleepSecs: 2,
});

const modes = [
  { key: "Search", icon: "🔍", label: "关键词搜索", desc: "按关键词搜索视频" },
  { key: "Detail", icon: "📹", label: "指定视频", desc: "输入视频链接采集" },
  { key: "Creator", icon: "👤", label: "创作者", desc: "采集创作者全部作品" },
] as const;

const keywordInput = ref("");
const videoUrlsInput = ref("");
const creatorUrlsInput = ref("");

// ===== 进度 =====
const progress = ref<CrawlProgress>({
  status: "Idle",
  current_keyword: "",
  total_videos: 0,
  fetched_videos: 0,
  total_comments: 0,
  fetched_comments: 0,
  downloaded_media: 0,
  errors: [],
  output_dir: "",
});

const isRunning = computed(() => progress.value.status === "Running");
const canStart = computed(() => {
  if (loginStatus.value !== "connected") return false;
  if (form.value.mode === "Search" && !keywordInput.value.trim()) return false;
  if (form.value.mode === "Detail" && !videoUrlsInput.value.trim()) return false;
  if (form.value.mode === "Creator" && !creatorUrlsInput.value.trim()) return false;
  return true;
});
const videoPercent = computed(() => {
  if (form.value.maxVideos === 0) return 0;
  return Math.min(100, (progress.value.fetched_videos / form.value.maxVideos) * 100);
});

// ===== 操作 =====
async function handleStart() {
  const config = {
    mode: form.value.mode,
    keywords: form.value.mode === "Search"
      ? keywordInput.value.split(",").map((k) => k.trim()).filter(Boolean)
      : undefined,
    video_urls: form.value.mode === "Detail"
      ? videoUrlsInput.value.split("\n").map((u) => u.trim()).filter(Boolean)
      : undefined,
    creator_urls: form.value.mode === "Creator"
      ? creatorUrlsInput.value.split("\n").map((u) => u.trim()).filter(Boolean)
      : undefined,
    max_videos: form.value.maxVideos,
    max_comments: form.value.maxComments,
    enable_comments: form.value.enableComments,
    enable_sub_comments: form.value.enableSubComments,
    enable_media: form.value.enableMedia,
    sort_type: form.value.sortType as "General" | "MostLike" | "Latest",
    publish_time: form.value.publishTime as "Unlimited" | "OneDay" | "OneWeek" | "SixMonths",
    output_dir: form.value.outputDir || "./data",
    sleep_secs: form.value.sleepSecs,
  };

  try {
    const msg = await startCrawl(
      cookieInput.value,
      userAgentInput.value || "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
      null,
      config
    );
    ElMessage.success(msg);
  } catch (e: any) {
    ElMessage.error(`启动失败: ${e}`);
  }
}

async function handleStop() {
  await stopCrawl();
}

async function handlePickDir() {
  const dir = await pickDirectory();
  if (dir) form.value.outputDir = dir;
}

async function handleOpenDir() {
  if (progress.value.output_dir) {
    await openOutputDir(progress.value.output_dir);
  }
}

// ===== 生命周期 =====
let unlistenProgress: (() => void) | null = null;
let unlistenError: (() => void) | null = null;

onMounted(async () => {
  unlistenProgress = await onProgress((p) => { progress.value = p; });
  unlistenError = await onError((err) => {
    ElMessage.error(err);
    progress.value.status = { Error: err };
  });
});

onUnmounted(() => {
  unlistenProgress?.();
  unlistenError?.();
  if (cookiePollTimer) clearInterval(cookiePollTimer);
});
</script>

<style>
/* 全局重置 */
* { margin: 0; padding: 0; box-sizing: border-box; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
  background: #f5f6fa;
  color: #1d2129;
  -webkit-font-smoothing: antialiased;
}
#app { height: 100vh; }
</style>

<style scoped>
.app-container {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f5f6fa;
  overflow: hidden;
}

/* ===== 顶栏 ===== */
.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 24px;
  height: 56px;
  background: #fff;
  border-bottom: 1px solid #e5e6eb;
  flex-shrink: 0;
  z-index: 10;
}
.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}
.logo-icon { font-size: 24px; }
.header-left h1 {
  font-size: 18px;
  font-weight: 700;
  background: linear-gradient(135deg, #fe2c55 0%, #ff6b81 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}
.status-badge {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  padding: 4px 12px;
  border-radius: 20px;
}
.status-badge .dot {
  width: 8px; height: 8px;
  border-radius: 50%;
}
.status-badge.connected {
  background: #e8f8ee;
  color: #00b42a;
}
.status-badge.connected .dot { background: #00b42a; }
.status-badge.connecting {
  background: #e8f4ff;
  color: #165dff;
}
.status-badge.connecting .dot { background: #165dff; }
.status-badge.disconnected {
  background: #f2f3f5;
  color: #86909c;
}
.status-badge.disconnected .dot { background: #c9cdd4; }
.pulse {
  animation: pulse 1.5s infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

/* ===== 主布局 ===== */
.main-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}
.sidebar {
  width: 400px;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex-shrink: 0;
  border-right: 1px solid #e5e6eb;
  background: #fff;
}
.main-panel {
  flex: 1;
  overflow-y: auto;
  padding: 16px 24px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* ===== 卡片 ===== */
.card {
  background: #fff;
  border-radius: 12px;
  border: 1px solid #e5e6eb;
  padding: 16px;
}
.card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: #1d2129;
  margin-bottom: 16px;
}
.card-icon {
  width: 20px;
  height: 20px;
  color: #fe2c55;
}

/* ===== 登录卡片 ===== */
.login-tabs :deep(.el-tabs__header) { margin-bottom: 12px; }
.login-tabs :deep(.el-tabs__item) {
  font-size: 14px;
  font-weight: 500;
}
.login-tabs :deep(.el-tabs__active-bar) { background: #fe2c55; }
.login-tabs :deep(.el-tabs__item.is-active) { color: #fe2c55; }

.qrcode-section { text-align: center; }
.hint { font-size: 13px; color: #86909c; margin-bottom: 12px; }
.qrcode-wrapper {
  width: 200px; height: 200px;
  margin: 0 auto 16px;
  position: relative;
}
.qrcode-placeholder {
  width: 200px; height: 200px;
  border-radius: 12px;
  background: #f7f8fa;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
}
.qrcode-dummy-icon { font-size: 48px; }
.qrcode-tip { font-size: 14px; color: #86909c; }
.qrcode-btn { width: 200px; }

.cookie-section .ua-input { margin: 10px 0; }
.cookie-login-btn { width: 100%; }

.login-success { margin-top: 8px; }

/* ===== 模式选择器 ===== */
.mode-selector {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}
.mode-btn {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 12px 8px;
  border: 2px solid #e5e6eb;
  border-radius: 10px;
  background: #fff;
  cursor: pointer;
  transition: all 0.2s;
  font-family: inherit;
}
.mode-btn:hover { border-color: #ffb3c1; background: #fff5f7; }
.mode-btn.active {
  border-color: #fe2c55;
  background: #fff5f7;
}
.mode-icon { font-size: 22px; }
.mode-label { font-size: 13px; font-weight: 600; color: #1d2129; }
.mode-desc { font-size: 11px; color: #86909c; }

/* ===== 表单 ===== */
.field { margin-bottom: 12px; }
.field label {
  display: block;
  font-size: 13px;
  font-weight: 500;
  color: #4e5969;
  margin-bottom: 4px;
}
.field-row { display: flex; gap: 10px; margin-bottom: 12px; }
.field.half { flex: 1; margin-bottom: 0; }

.field-checklist {
  display: flex;
  gap: 16px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.dir-field { display: flex; gap: 8px; }
.dir-input { flex: 1; }

.start-btn { width: 100%; margin-top: 4px; height: 44px; font-size: 16px; }
.stop-btn { width: 100%; margin-top: 8px; }

/* ===== 进度 ===== */
.progress-stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 10px;
  margin-bottom: 16px;
}
.stat-card {
  background: #f7f8fa;
  border-radius: 10px;
  padding: 14px 8px;
  text-align: center;
}
.stat-num {
  font-size: 26px;
  font-weight: 700;
  color: #1d2129;
  line-height: 1.2;
}
.stat-label {
  font-size: 12px;
  color: #86909c;
  margin-top: 2px;
}

.progress-bar-section { margin-bottom: 12px; }
.progress-label {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  margin-bottom: 6px;
  color: #4e5969;
}
.progress-track {
  height: 10px;
  background: #e5e6eb;
  border-radius: 5px;
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #fe2c55, #ff6b81);
  border-radius: 5px;
  transition: width 0.4s ease;
}

.current-kw {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 8px;
}

/* tags */
.tag {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  font-weight: 500;
  margin-left: auto;
}
.tag-success { background: #e8f8ee; color: #00b42a; }
.tag-running { background: #e8f4ff; color: #165dff; }
.tag-idle { background: #f2f3f5; color: #86909c; }

/* ===== 错误日志 ===== */
.error-list { max-height: 200px; overflow-y: auto; }
.error-item {
  font-size: 12px;
  color: #f53f3f;
  padding: 6px 0;
  border-bottom: 1px solid #f2f3f5;
  word-break: break-all;
  display: flex;
  gap: 8px;
}
.error-idx { color: #c9cdd4; flex-shrink: 0; }

/* ===== 空状态 ===== */
.empty-state {
  text-align: center;
  padding: 80px 20px;
  color: #86909c;
}
.empty-icon { font-size: 64px; margin-bottom: 16px; }
.empty-state h3 { font-size: 18px; color: #4e5969; margin-bottom: 8px; }
.empty-state p { font-size: 14px; }

/* 滚动条 */
::-webkit-scrollbar { width: 5px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: #c9cdd4; border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: #86909c; }

/* Element Plus 主题覆盖 */
:deep(.el-button--primary) {
  --el-button-bg-color: #fe2c55;
  --el-button-border-color: #fe2c55;
  --el-button-hover-bg-color: #e81e49;
  --el-button-hover-border-color: #e81e49;
  --el-button-active-bg-color: #cf1a41;
}
:deep(.el-checkbox__input.is-checked .el-checkbox__inner) {
  background-color: #fe2c55;
  border-color: #fe2c55;
}
:deep(.el-checkbox__input.is-checked + .el-checkbox__label) {
  color: #fe2c55;
}
:deep(.el-input-number__increase),
:deep(.el-input-number__decrease) {
  --el-border-color: #e5e6eb;
}
</style>
