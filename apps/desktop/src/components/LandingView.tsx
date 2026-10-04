import React from 'react';
import {
  Sparkles,
  Download,
  PlayCircle,
  Tv,
  Network,
  Clock,
  ShieldCheck,
  Bot,
  ArrowRight,
  HardDrive,
  CheckCircle2,
  Github,
} from 'lucide-react';

interface LandingViewProps {
  onEnterDemo: () => void;
}

export const LandingView: React.FC<LandingViewProps> = ({ onEnterDemo }) => {
  const GITHUB_REPO = 'https://github.com/kirigayakazima/Looma';
  const GITHUB_RELEASES = 'https://github.com/kirigayakazima/Looma/releases';

  return (
    <div className="min-h-screen bg-neutral-950 text-neutral-100 selection:bg-emerald-500/30 selection:text-emerald-200">
      {/* Background ambient glow */}
      <div className="fixed inset-0 pointer-events-none overflow-hidden z-0">
        <div className="absolute -top-40 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-gradient-to-b from-emerald-500/10 via-sky-500/5 to-transparent blur-3xl rounded-full" />
        <div className="absolute top-1/3 -left-48 w-96 h-96 bg-purple-500/10 blur-3xl rounded-full" />
        <div className="absolute top-2/3 -right-48 w-96 h-96 bg-emerald-500/10 blur-3xl rounded-full" />
      </div>

      {/* Navigation Header */}
      <header className="sticky top-0 z-50 backdrop-blur-md bg-neutral-950/80 border-b border-neutral-800/80">
        <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-emerald-400 to-teal-600 flex items-center justify-center shadow-lg shadow-emerald-500/20">
              <Sparkles className="w-4 h-4 text-neutral-950" />
            </div>
            <div className="flex items-baseline gap-2">
              <span className="font-bold text-lg tracking-tight text-white">Looma</span>
              <span className="text-xs px-2 py-0.5 rounded-full bg-neutral-800 text-neutral-400 font-mono">
                v0.2.0
              </span>
            </div>
          </div>

          <nav className="hidden md:flex items-center gap-8 text-sm text-neutral-400">
            <a href="#features" className="hover:text-emerald-400 transition-colors">
              核心特性
            </a>
            <a href="#architecture" className="hover:text-emerald-400 transition-colors">
              架构设计
            </a>
            <a href="#preview" className="hover:text-emerald-400 transition-colors">
              界面预览
            </a>
            <a
              href={GITHUB_REPO}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1.5 hover:text-emerald-400 transition-colors"
            >
              <Github className="w-4 h-4" />
              <span>GitHub</span>
            </a>
          </nav>

          <div className="flex items-center gap-3">
            <button
              onClick={onEnterDemo}
              className="px-3.5 py-1.5 rounded-lg text-xs font-medium bg-neutral-800 hover:bg-neutral-700 text-neutral-200 border border-neutral-700 flex items-center gap-1.5 transition-all shadow-sm cursor-pointer"
            >
              <PlayCircle className="w-3.5 h-3.5 text-emerald-400" />
              <span>在线 Demo</span>
            </button>
            <a
              href={GITHUB_RELEASES}
              target="_blank"
              rel="noopener noreferrer"
              className="px-3.5 py-1.5 rounded-lg text-xs font-medium bg-emerald-500 hover:bg-emerald-400 text-neutral-950 flex items-center gap-1.5 transition-all shadow-md shadow-emerald-500/20 font-semibold cursor-pointer"
            >
              <Download className="w-3.5 h-3.5" />
              <span className="hidden sm:inline">下载</span> Windows 版
            </a>
          </div>
        </div>
      </header>

      {/* Hero Section */}
      <section className="relative z-10 pt-20 pb-16 px-6 max-w-5xl mx-auto text-center">
        <div className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-full bg-neutral-900/90 border border-neutral-800 text-xs text-neutral-300 mb-8 backdrop-blur shadow-inner">
          <span className="flex h-2 w-2 rounded-full bg-emerald-400 animate-pulse" />
          <span>本地优先 · 个人数字资产库 · 作品档案与记忆织机</span>
        </div>

        <h1 className="text-4xl sm:text-6xl font-extrabold tracking-tight text-white mb-6 leading-tight">
          让离散的数字足迹，
          <br />
          <span className="bg-gradient-to-r from-emerald-400 via-teal-300 to-sky-400 bg-clip-text text-transparent">
            编织成可漫游的生命之网
          </span>
        </h1>

        <p className="max-w-2xl mx-auto text-base sm:text-lg text-neutral-400 mb-10 leading-relaxed">
          Looma 是一款采用 Rust + Tauri 2 驱动的本地优先桌面系统。你的动画番剧追更、游戏通关记录、本地大文件引用与灵感手记，全部安全沉淀于本地 SQLite 数据库，并天然接入标准 MCP (Model Context Protocol) 赋能外部 AI。
        </p>

        <div className="flex flex-wrap items-center justify-center gap-4">
          <button
            onClick={onEnterDemo}
            className="px-6 py-3 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-neutral-950 font-bold text-sm flex items-center gap-2 transition-all shadow-lg shadow-emerald-500/25 hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
          >
            <PlayCircle className="w-4 h-4" />
            <span>进入在线 Demo 体验</span>
            <ArrowRight className="w-4 h-4 ml-1" />
          </button>

          <a
            href={GITHUB_RELEASES}
            target="_blank"
            rel="noopener noreferrer"
            className="px-6 py-3 rounded-xl bg-neutral-900 hover:bg-neutral-800 text-neutral-100 font-medium text-sm border border-neutral-700/80 flex items-center gap-2 transition-all hover:scale-[1.02] active:scale-[0.98]"
          >
            <Download className="w-4 h-4 text-emerald-400" />
            <span>下载 Windows 客户端 (.exe)</span>
          </a>

          <a
            href={GITHUB_REPO}
            target="_blank"
            rel="noopener noreferrer"
            className="px-4 py-3 rounded-xl bg-neutral-900/60 hover:bg-neutral-800/80 text-neutral-400 hover:text-neutral-200 text-sm border border-neutral-800 flex items-center gap-1.5 transition-all"
          >
            <Github className="w-4 h-4" />
            <span>GitHub Star</span>
          </a>
        </div>

        {/* Feature Highlights Pills */}
        <div className="mt-12 flex flex-wrap justify-center items-center gap-6 text-xs text-neutral-400">
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-emerald-400" />
            <span>零搬移引用扫描</span>
          </div>
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-emerald-400" />
            <span>作品进度微调 (+ / -)</span>
          </div>
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-emerald-400" />
            <span>原生 SQLite 存储</span>
          </div>
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-emerald-400" />
            <span>离线可用 · 隐私绝对自主</span>
          </div>
        </div>
      </section>

      {/* Interactive Showcase Preview */}
      <section id="preview" className="relative z-10 max-w-6xl mx-auto px-6 pb-20">
        <div className="relative rounded-2xl border border-neutral-800 bg-neutral-900/70 backdrop-blur-xl shadow-2xl overflow-hidden p-2 sm:p-4 group">
          {/* Top Bar of Mock App */}
          <div className="flex items-center justify-between px-3 py-2 border-b border-neutral-800/60 text-xs text-neutral-400">
            <div className="flex items-center gap-2">
              <div className="flex gap-1.5">
                <div className="w-2.5 h-2.5 rounded-full bg-red-500/80" />
                <div className="w-2.5 h-2.5 rounded-full bg-yellow-500/80" />
                <div className="w-2.5 h-2.5 rounded-full bg-green-500/80" />
              </div>
              <span className="font-mono text-neutral-500 ml-2">Looma Desktop Preview</span>
            </div>
            <button
              onClick={onEnterDemo}
              className="px-2.5 py-1 rounded bg-emerald-500/20 text-emerald-300 hover:bg-emerald-500/30 font-medium text-[11px] flex items-center gap-1 transition-colors"
            >
              <span>点此直接在浏览器试用</span>
              <ArrowRight className="w-3 h-3" />
            </button>
          </div>

          {/* Simulated App Dashboard Snippet */}
          <div
            onClick={onEnterDemo}
            className="p-6 grid grid-cols-1 md:grid-cols-3 gap-4 cursor-pointer hover:opacity-95 transition-opacity"
            title="点击进入全功能交互 Demo"
          >
            {/* Card 1: Anime Tracking */}
            <div className="bg-neutral-950/80 rounded-xl p-4 border border-neutral-800 hover:border-emerald-500/40 transition-colors">
              <div className="flex items-center justify-between mb-3">
                <span className="px-2 py-0.5 rounded text-[11px] font-semibold bg-emerald-500/20 text-emerald-400">
                  番剧 / Anime
                </span>
                <span className="text-xs text-neutral-500">追更中</span>
              </div>
              <h3 className="font-semibold text-neutral-100 text-sm mb-1">BLEACH 千年血战篇 相克谭</h3>
              <p className="text-xs text-neutral-500 mb-3 line-clamp-1">TV动画第三季，灭却师灵王宫死斗</p>
              <div className="space-y-1.5">
                <div className="flex justify-between text-xs">
                  <span className="text-neutral-400">追更进度</span>
                  <span className="text-emerald-400 font-mono font-medium">第 32 / 39 话</span>
                </div>
                <div className="w-full bg-neutral-800 h-1.5 rounded-full overflow-hidden">
                  <div className="bg-emerald-500 h-full rounded-full" style={{ width: '82%' }} />
                </div>
              </div>
            </div>

            {/* Card 2: Game Library Tracking */}
            <div className="bg-neutral-950/80 rounded-xl p-4 border border-neutral-800 hover:border-emerald-500/40 transition-colors">
              <div className="flex items-center justify-between mb-3">
                <span className="px-2 py-0.5 rounded text-[11px] font-semibold bg-emerald-500/20 text-emerald-400 font-mono">
                  游戏库 / Game
                </span>
                <span className="text-xs text-neutral-500">游玩中</span>
              </div>
              <h3 className="font-semibold text-neutral-100 text-sm mb-1">尼尔：机械纪元 (NieR:Automata)</h3>
              <p className="text-xs text-neutral-400 mb-2 font-mono truncate">
                📁 E:\Games\NieR_Automata
              </p>
              <div className="space-y-1.5">
                <div className="flex justify-between text-xs">
                  <span className="text-neutral-400">通关进度</span>
                  <span className="text-emerald-400 font-mono font-medium">第 3 / 5 结局周目</span>
                </div>
                <div className="w-full bg-neutral-800 h-1.5 rounded-full overflow-hidden">
                  <div className="bg-emerald-500 h-full rounded-full" style={{ width: '60%' }} />
                </div>
              </div>
            </div>

            {/* Card 3: Memory / Timeline */}
            <div className="bg-neutral-950/80 rounded-xl p-4 border border-neutral-800 hover:border-emerald-500/40 transition-colors">
              <div className="flex items-center justify-between mb-3">
                <span className="px-2 py-0.5 rounded text-[11px] font-semibold bg-purple-500/20 text-purple-400">
                  记忆手记 / Note
                </span>
                <span className="text-xs text-neutral-500">2024-10-02</span>
              </div>
              <h3 className="font-semibold text-neutral-100 text-sm mb-1">《葬送的芙莉莲》补番完结评测</h3>
              <p className="text-xs text-neutral-400 line-clamp-2 leading-relaxed">
                “所谓微小的旅行回忆，正是漫长精灵生命中最灿烂的星火...” 评分 9.8/10。
              </p>
            </div>
          </div>

          <div className="bg-neutral-950/60 p-3 text-center border-t border-neutral-800/60 text-xs text-neutral-400 flex items-center justify-center gap-2">
            <Sparkles className="w-3.5 h-3.5 text-emerald-400" />
            <span>无需后端服务器支持，纯静态网页版现已内置完整模拟数据与 LocalStorage 本地持久化</span>
          </div>
        </div>
      </section>

      {/* Core Features Grid */}
      <section id="features" className="py-20 px-6 max-w-7xl mx-auto border-t border-neutral-900">
        <div className="text-center max-w-3xl mx-auto mb-16">
          <h2 className="text-xs font-semibold text-emerald-400 uppercase tracking-widest mb-3">
            Core Architecture & Capabilities
          </h2>
          <p className="text-3xl sm:text-4xl font-bold text-white tracking-tight">
            六大核心子系统，构筑终身数字资产基座
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {/* Feature 1 */}
          <div className="bg-neutral-900/40 border border-neutral-800/80 hover:border-neutral-700 rounded-2xl p-6 transition-all">
            <div className="w-10 h-10 rounded-xl bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-400 mb-4">
              <Tv className="w-5 h-5" />
            </div>
            <h3 className="text-lg font-semibold text-white mb-2">作品档案与追更微调</h3>
            <p className="text-sm text-neutral-400 leading-relaxed">
              动画、漫画、游戏、书籍分门别类。支持多维度进度指标（话/集/卷/百分比/时长），支持列表内 <code className="text-emerald-400 bg-neutral-950 px-1 py-0.5 rounded text-xs">+1 / -1</code> 瞬时微调，并直达 Bangumi、Steam 等外部站。
            </p>
          </div>

          {/* Feature 2 */}
          <div className="bg-neutral-900/40 border border-neutral-800/80 hover:border-neutral-700 rounded-2xl p-6 transition-all">
            <div className="w-10 h-10 rounded-xl bg-sky-500/10 border border-sky-500/20 flex items-center justify-center text-sky-400 mb-4">
              <HardDrive className="w-5 h-5" />
            </div>
            <h3 className="text-lg font-semibold text-white mb-2">零搬移·原地引用存储</h3>
            <p className="text-sm text-neutral-400 leading-relaxed">
              无需把海量动漫画、壁纸、影视复制搬迁到特定仓库。通过快速增量扫描器索引文件元数据与 SHA256 指纹，保持原始目录结构不变，无冗余磁盘占用。
            </p>
          </div>

          {/* Feature 3 */}
          <div className="bg-neutral-900/40 border border-neutral-800/80 hover:border-neutral-700 rounded-2xl p-6 transition-all">
            <div className="w-10 h-10 rounded-xl bg-purple-500/10 border border-purple-500/20 flex items-center justify-center text-purple-400 mb-4">
              <Network className="w-5 h-5" />
            </div>
            <h3 className="text-lg font-semibold text-white mb-2">通用多态实体与图谱</h3>
            <p className="text-sm text-neutral-400 leading-relaxed">
              创作者、角色、制作公司、IP宇宙自由建模。灵活建立多对多双向关系网络，让作品与本地截图、素材、官方手记紧密交织。
            </p>
          </div>

          {/* Feature 4 */}
          <div className="bg-neutral-900/40 border border-neutral-800/80 hover:border-neutral-700 rounded-2xl p-6 transition-all">
            <div className="w-10 h-10 rounded-xl bg-amber-500/10 border border-amber-500/20 flex items-center justify-center text-amber-400 mb-4">
              <Clock className="w-5 h-5" />
            </div>
            <h3 className="text-lg font-semibold text-white mb-2">记忆手记与时光流动</h3>
            <p className="text-sm text-neutral-400 leading-relaxed">
              内置纯粹的本地 Markdown 手记编辑器。所有打卡、完结评测、感悟手记统一汇总于时序时光轴，按年月穿梭回顾你的数字生命切片。
            </p>
          </div>

          {/* Feature 5 */}
          <div className="bg-neutral-900/40 border border-neutral-800/80 hover:border-neutral-700 rounded-2xl p-6 transition-all">
            <div className="w-10 h-10 rounded-xl bg-rose-500/10 border border-rose-500/20 flex items-center justify-center text-rose-400 mb-4">
              <ShieldCheck className="w-5 h-5" />
            </div>
            <h3 className="text-lg font-semibold text-white mb-2">资产医生与安全体检</h3>
            <p className="text-sm text-neutral-400 leading-relaxed">
              文件被外部误删或改名？一键启动 Vault Doctor 深度体检，自动标记失效资产，保证 SQLite 数据库完整性与资产状态绝对一致。
            </p>
          </div>

          {/* Feature 6 */}
          <div className="bg-neutral-900/40 border border-neutral-800/80 hover:border-neutral-700 rounded-2xl p-6 transition-all">
            <div className="w-10 h-10 rounded-xl bg-teal-500/10 border border-teal-500/20 flex items-center justify-center text-teal-400 mb-4">
              <Bot className="w-5 h-5" />
            </div>
            <h3 className="text-lg font-semibold text-white mb-2">标准 MCP 与 AI 互通</h3>
            <p className="text-sm text-neutral-400 leading-relaxed">
              内置标准 Model Context Protocol (MCP) 服务。Cursor、Claude、Antigravity 等外部 AI 助手可直接检索你的追更进度与本地知识，完成智能梳理。
            </p>
          </div>
        </div>
      </section>

      {/* Architecture Deep Dive */}
      <section id="architecture" className="py-20 px-6 max-w-6xl mx-auto border-t border-neutral-900">
        <div className="text-center max-w-2xl mx-auto mb-14">
          <h2 className="text-xs font-semibold text-emerald-400 uppercase tracking-widest mb-2">
            Rust Core Architecture
          </h2>
          <p className="text-3xl font-bold text-white">坚实、现代、完全可控的技术栈</p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-4 gap-4 text-center">
          <div className="bg-neutral-900/50 border border-neutral-800 rounded-xl p-5">
            <div className="font-mono text-emerald-400 text-base font-bold mb-1">Tauri v2</div>
            <div className="text-xs text-neutral-400">轻量原生系统容器</div>
            <div className="text-[11px] text-neutral-500 mt-2">极致内存占用，无庞大 Chromium 负担</div>
          </div>

          <div className="bg-neutral-900/50 border border-neutral-800 rounded-xl p-5">
            <div className="font-mono text-sky-400 text-base font-bold mb-1">Rust 2021</div>
            <div className="text-xs text-neutral-400">核心性能与文件引擎</div>
            <div className="text-[11px] text-neutral-500 mt-2">高并发增量扫描，零悬挂指针</div>
          </div>

          <div className="bg-neutral-900/50 border border-neutral-800 rounded-xl p-5">
            <div className="font-mono text-amber-400 text-base font-bold mb-1">SQLite 3</div>
            <div className="text-xs text-neutral-400">本地事务型知识库</div>
            <div className="text-[11px] text-neutral-500 mt-2">单文件携带，数十万条资产秒级检索</div>
          </div>

          <div className="bg-neutral-900/50 border border-neutral-800 rounded-xl p-5">
            <div className="font-mono text-purple-400 text-base font-bold mb-1">React 18 + Vite</div>
            <div className="text-xs text-neutral-400">现代桌面与网页共用</div>
            <div className="text-[11px] text-neutral-500 mt-2">双端架构，无缝部署 GitHub Pages</div>
          </div>
        </div>
      </section>

      {/* Download & Demo CTA */}
      <section className="py-20 px-6 max-w-4xl mx-auto text-center border-t border-neutral-900">
        <div className="p-10 rounded-3xl bg-gradient-to-b from-neutral-900 to-neutral-950 border border-neutral-800 relative overflow-hidden">
          <div className="absolute top-0 right-0 -mt-10 -mr-10 w-40 h-40 bg-emerald-500/10 rounded-full blur-2xl pointer-events-none" />

          <h2 className="text-2xl sm:text-3xl font-bold text-white mb-4">
            开始梳理你的数字世界
          </h2>
          <p className="text-neutral-400 text-sm max-w-xl mx-auto mb-8">
            你可以先在浏览器里体验完整的在线演练 Demo，或者直接下载 Windows 桌面端接管你本地的实体文件与番剧库。
          </p>

          <div className="flex flex-wrap items-center justify-center gap-4">
            <button
              onClick={onEnterDemo}
              className="px-6 py-3 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-neutral-950 font-bold text-sm flex items-center gap-2 transition-all shadow-lg shadow-emerald-500/20 cursor-pointer"
            >
              <PlayCircle className="w-4 h-4" />
              <span>在线体验 Web Demo</span>
            </button>
            <a
              href={GITHUB_RELEASES}
              target="_blank"
              rel="noopener noreferrer"
              className="px-6 py-3 rounded-xl bg-neutral-800 hover:bg-neutral-700 text-white font-medium text-sm flex items-center gap-2 transition-all border border-neutral-700"
            >
              <Download className="w-4 h-4 text-emerald-400" />
              <span>下载最新发行版 (.exe)</span>
            </a>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="py-8 px-6 border-t border-neutral-900 text-neutral-500 text-xs">
        <div className="max-w-7xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-4">
          <div className="flex items-center gap-2">
            <span className="font-semibold text-neutral-400">Looma</span>
            <span>— 本地优先数字资产与记忆织机</span>
          </div>
          <div className="flex items-center gap-6">
            <button
              onClick={onEnterDemo}
              className="hover:text-emerald-400 transition-colors"
            >
              在线体验
            </button>
            <a
              href={GITHUB_RELEASES}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-emerald-400 transition-colors"
            >
              发行版下载
            </a>
            <a
              href={GITHUB_REPO}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-emerald-400 transition-colors"
            >
              GitHub 仓库
            </a>
          </div>
          <div>MIT License · Crafted with Rust & React</div>
        </div>
      </footer>
    </div>
  );
};
