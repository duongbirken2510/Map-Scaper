import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import ZaloTab from "./ZaloTab";
import "./App.css";

// ============================================================
// Types
// ============================================================
interface Lead {
  stt: number;
  name: string;
  phone: string;
  address: string;
  region: string;
}

interface DbLead {
  id: number;
  name: string;
  phone: string;
  address: string;
  region: string;
  keyword: string;
  scraped_at: string;
}

interface DbStats {
  total_leads: number;
  unique_phones: number;
  regions: { region: string; count: number }[];
  keywords: { keyword: string; count: number }[];
}

interface ScrapeProgress {
  message: string;
  found: number;
  skipped: number;
  done: boolean;
  error?: string;
}

type Tab = "logs" | "results" | "history" | "zalo";

// ============================================================
// App
// ============================================================
function App() {
  // Config
  const [keyword, setKeyword] = useState("giày dép");
  const [region, setRegion] = useState("Hồ Chí Minh");
  const [limit, setLimit] = useState(50);
  const [deduplicate, setDeduplicate] = useState(true);

  // Scrape state
  const [isScraping, setIsScraping] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const [results, setResults] = useState<Lead[]>([]);
  const [found, setFound] = useState(0);
  const [skipped, setSkipped] = useState(0);

  // DB / History state
  const [activeTab, setActiveTab] = useState<Tab>("logs");
  const [history, setHistory] = useState<DbLead[]>([]);
  const [stats, setStats] = useState<DbStats | null>(null);
  const [historyFilter, setHistoryFilter] = useState("");
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  const logsEndRef = useRef<HTMLDivElement>(null);

  const addLog = (msg: string) => {
    const time = new Date().toLocaleTimeString("vi-VN");
    setLogs((prev) => [...prev, `[${time}] ${msg}`]);
  };

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  // Real-time progress from Rust
  useEffect(() => {
    const unlisten = listen<ScrapeProgress>("scrape_progress", (e) => {
      const p = e.payload;
      addLog(p.message);
      setFound(p.found);
      setSkipped(p.skipped);
      if (p.done) {
        setIsScraping(false);
        setActiveTab("results");
        loadStats(); // refresh stats after scrape
      }
      if (p.error) {
        addLog(`❌ Lỗi: ${p.error}`);
        setIsScraping(false);
      }
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  // Load stats on mount
  useEffect(() => { loadStats(); }, []);

  // ── Actions ────────────────────────────────────────────────

  const startScraping = async () => {
    if (!keyword.trim() || !region.trim()) {
      addLog("⚠️ Vui lòng nhập đầy đủ Từ khóa và Khu vực.");
      return;
    }
    setIsScraping(true);
    setResults([]);
    setFound(0);
    setSkipped(0);
    setLogs([]);
    setActiveTab("logs");
    try {
      const leads = await invoke<Lead[]>("start_scraping", {
        config: { keyword, region, limit, deduplicate },
      });
      setResults(leads);
    } catch (err: any) {
      addLog(`❌ Lỗi hệ thống: ${err}`);
      setIsScraping(false);
    }
  };

  const exportSessionExcel = async () => {
    if (results.length === 0) return;
    try {
      const filePath = await save({
        title: "Lưu file Excel (phiên hiện tại)",
        defaultPath: `khachhang_${keyword}_${region}.xlsx`,
        filters: [{ name: "Excel File", extensions: ["xlsx"] }],
      });
      if (!filePath) return;
      const msg = await invoke<string>("export_excel", { leads: results, filePath });
      addLog(`✅ ${msg}`);
    } catch (err: any) {
      addLog(`❌ Lỗi xuất file: ${err}`);
    }
  };

  const exportAllExcel = async () => {
    try {
      const filePath = await save({
        title: "Xuất toàn bộ dữ liệu từ Database",
        defaultPath: `toan_bo_khachhang.xlsx`,
        filters: [{ name: "Excel File", extensions: ["xlsx"] }],
      });
      if (!filePath) return;
      const msg = await invoke<string>("export_all_to_excel", { filePath });
      addLog(`✅ ${msg}`);
      setActiveTab("logs");
    } catch (err: any) {
      addLog(`❌ Lỗi xuất file: ${err}`);
    }
  };

  const loadHistory = async () => {
    setIsLoadingHistory(true);
    try {
      const data = historyFilter.trim()
        ? await invoke<DbLead[]>("get_history_by_region", { region: historyFilter })
        : await invoke<DbLead[]>("get_history");
      setHistory(data);
    } catch (err) {
      console.error(err);
    } finally {
      setIsLoadingHistory(false);
    }
  };

  const loadStats = async () => {
    try {
      const s = await invoke<DbStats>("get_db_stats");
      setStats(s);
    } catch { }
  };

  const clearAll = async () => {
    try {
      const msg = await invoke<string>("clear_all_leads");
      addLog(`🗑 ${msg}`);
      setHistory([]);
      loadStats();
      setConfirmClear(false);
    } catch (err: any) {
      addLog(`❌ ${err}`);
    }
  };

  useEffect(() => {
    if (activeTab === "history") loadHistory();
  }, [activeTab]);

  const progress = limit > 0 ? Math.min((found / limit) * 100, 100) : 0;

  return (
    <div className="h-screen flex flex-col bg-slate-950 text-slate-100 font-sans overflow-hidden">
      {/* ── Header ── */}
      <header className="flex items-center justify-between px-6 py-3 bg-slate-900 border-b border-slate-700/60 shrink-0">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-blue-600 flex items-center justify-center text-lg">📍</div>
          <div>
            <h1 className="text-base font-bold text-white leading-none">Tidu Map Scraper</h1>
            <p className="text-xs text-slate-400 mt-0.5">Thu thập khách hàng tiềm năng từ Google Maps</p>
          </div>
        </div>
        <div className="flex items-center gap-5 text-sm">
          {stats && (
            <div className="flex items-center gap-4 text-xs">
              <span className="text-slate-400">DB tổng: <strong className="text-white">{stats.total_leads.toLocaleString()}</strong></span>
              <span className="text-slate-600">|</span>
              <span className="text-slate-400">SĐT unique: <strong className="text-emerald-400">{stats.unique_phones.toLocaleString()}</strong></span>
            </div>
          )}
          <span className="text-slate-600">|</span>
          <span className="text-emerald-400 text-xs flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-emerald-400 inline-block animate-pulse"></span>
            Phiên này: <strong>{found}</strong>
          </span>
          <span className="text-amber-400 text-xs">Bỏ qua: <strong>{skipped}</strong></span>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        {/* ── Sidebar ── */}
        <aside className="w-72 shrink-0 bg-slate-900 border-r border-slate-700/60 flex flex-col p-5 gap-4 overflow-y-auto">
          <div>
            <h2 className="text-xs font-semibold text-slate-400 uppercase tracking-widest mb-4">Cấu hình cào</h2>
            <div className="space-y-3">
              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1.5">🔍 Từ khóa</label>
                <input type="text" value={keyword} onChange={(e) => setKeyword(e.target.value)}
                  disabled={isScraping}
                  className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
                  placeholder="VD: giày dép" />
              </div>
              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1.5">📍 Khu vực</label>
                <input type="text" value={region} onChange={(e) => setRegion(e.target.value)}
                  disabled={isScraping}
                  className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
                  placeholder="VD: Hồ Chí Minh" />
              </div>
              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1.5">🎯 Số lượng tối đa</label>
                <input type="number" value={limit} min={1} max={500}
                  onChange={(e) => setLimit(Number(e.target.value))}
                  disabled={isScraping}
                  className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50" />
              </div>
              <div className="flex items-center gap-3 pt-1">
                <button role="switch" aria-checked={deduplicate}
                  onClick={() => !isScraping && setDeduplicate((v) => !v)}
                  className={`relative w-10 h-5 rounded-full transition-colors shrink-0 ${deduplicate ? "bg-blue-600" : "bg-slate-600"} ${isScraping ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}`}>
                  <span className={`absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full shadow transition-transform ${deduplicate ? "translate-x-5" : "translate-x-0"}`} />
                </button>
                <label className="text-xs text-slate-300 select-none">Lọc trùng (toàn DB)</label>
              </div>
            </div>
          </div>

          {/* Progress */}
          <div>
            <div className="flex justify-between text-xs text-slate-400 mb-1.5">
              <span>Tiến trình</span><span>{found}/{limit}</span>
            </div>
            <div className="h-2 bg-slate-700 rounded-full overflow-hidden">
              <div className="h-full bg-blue-500 rounded-full transition-all duration-500" style={{ width: `${progress}%` }} />
            </div>
          </div>

          {/* DB Info box */}
          {stats && (
            <div className="bg-slate-800/60 rounded-lg p-3 border border-slate-700/40 space-y-1">
              <p className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">Trạng thái Database</p>
              <div className="text-xs text-slate-300 space-y-1">
                <div className="flex justify-between"><span>Tổng leads:</span><strong className="text-white">{stats.total_leads.toLocaleString()}</strong></div>
                <div className="flex justify-between"><span>SĐT unique:</span><strong className="text-emerald-400">{stats.unique_phones.toLocaleString()}</strong></div>
                {stats.regions.slice(0, 3).map(r => (
                  <div key={r.region} className="flex justify-between text-slate-400">
                    <span className="truncate max-w-[140px]">{r.region}:</span><span>{r.count}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Action buttons */}
          <div className="space-y-2 mt-auto">
            <button id="btn-start-scraping" onClick={startScraping} disabled={isScraping}
              className={`w-full py-2.5 rounded-lg font-semibold text-sm transition-all ${isScraping ? "bg-slate-700 text-slate-400 cursor-not-allowed" : "bg-blue-600 hover:bg-blue-500 text-white shadow-lg"}`}>
              {isScraping ? (
                <span className="flex items-center justify-center gap-2">
                  <span className="w-4 h-4 border-2 border-slate-400 border-t-transparent rounded-full animate-spin" />Đang cào...
                </span>
              ) : "▶ Bắt đầu cào"}
            </button>
            <button id="btn-export-session" onClick={exportSessionExcel} disabled={isScraping || results.length === 0}
              className="w-full py-2 rounded-lg font-semibold text-sm bg-emerald-600 hover:bg-emerald-500 text-white shadow transition-all disabled:opacity-40 disabled:cursor-not-allowed">
              📊 Xuất phiên này ({results.length})
            </button>
            <button id="btn-export-all" onClick={exportAllExcel} disabled={isScraping || !stats || stats.total_leads === 0}
              className="w-full py-2 rounded-lg font-semibold text-sm bg-violet-600 hover:bg-violet-500 text-white shadow transition-all disabled:opacity-40 disabled:cursor-not-allowed">
              🗄 Xuất toàn bộ DB ({stats?.total_leads ?? 0})
            </button>
          </div>
        </aside>

        {/* ── Main Content ── */}
        <main className="flex-1 flex flex-col overflow-hidden">
          {/* Tabs */}
          <div className="flex border-b border-slate-700/60 shrink-0 bg-slate-900/50">
            {(["logs", "results", "history", "zalo"] as Tab[]).map((tab) => (
              <button key={tab} onClick={() => setActiveTab(tab)}
                className={`px-5 py-3 text-sm font-medium transition-colors border-b-2 ${activeTab === tab ? "border-blue-500 text-blue-400" : "border-transparent text-slate-400 hover:text-slate-200"}`}>
                {tab === "logs" && "📋 Logs"}
                {tab === "results" && <>📊 Kết quả {results.length > 0 && <span className="ml-1.5 px-1.5 py-0.5 text-xs bg-blue-600 text-white rounded-full">{results.length}</span>}</>}
                {tab === "history" && <>🗄 Lịch sử DB {stats && stats.total_leads > 0 && <span className="ml-1.5 px-1.5 py-0.5 text-xs bg-violet-600 text-white rounded-full">{stats.total_leads.toLocaleString()}</span>}</>}
                {tab === "zalo" && "🤝 Zalo CRM"}
              </button>
            ))}
          </div>

          {/* ── Logs Panel ── */}
          {activeTab === "logs" && (
            <div className="flex-1 overflow-y-auto p-4 font-mono text-xs bg-slate-950 space-y-1">
              {logs.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-slate-600 gap-3">
                  <div className="text-4xl">🗺️</div>
                  <p>Nhấn "Bắt đầu cào" để bắt đầu...</p>
                </div>
              ) : (
                <>
                  {logs.map((log, i) => (
                    <div key={i} className={`leading-relaxed ${log.includes("❌") ? "text-red-400" : log.includes("✅") || log.includes("🎉") ? "text-emerald-400" : log.includes("⚠") ? "text-amber-400" : log.includes("⊘") ? "text-slate-500" : "text-green-400"}`}>
                      {log}
                    </div>
                  ))}
                  <div ref={logsEndRef} />
                </>
              )}
            </div>
          )}

          {/* ── Results Panel ── */}
          {activeTab === "results" && (
            <div className="flex-1 overflow-auto">
              {results.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-slate-600 gap-3">
                  <div className="text-4xl">📭</div><p>Chưa có dữ liệu phiên hiện tại.</p>
                </div>
              ) : (
                <table className="w-full text-sm border-collapse">
                  <thead className="sticky top-0 bg-slate-800 text-slate-300 z-10">
                    <tr>
                      {["STT", "Tên khách hàng", "Số điện thoại", "Địa chỉ", "Khu vực"].map(h => (
                        <th key={h} className="px-4 py-3 text-left text-xs font-semibold uppercase tracking-wider">{h}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {results.map((lead, i) => (
                      <tr key={lead.stt} className={`border-t border-slate-800 hover:bg-slate-800/50 ${i % 2 === 0 ? "bg-slate-900/30" : "bg-slate-900/60"}`}>
                        <td className="px-4 py-2.5 text-slate-400 text-center w-12">{lead.stt}</td>
                        <td className="px-4 py-2.5 font-medium text-white">{lead.name || "—"}</td>
                        <td className="px-4 py-2.5 text-blue-400 font-mono w-36">{lead.phone || "—"}</td>
                        <td className="px-4 py-2.5 text-slate-300 text-xs">{lead.address || "—"}</td>
                        <td className="px-4 py-2.5 text-slate-300 text-xs w-40">{lead.region || "—"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          )}

          {/* ── History Panel ── */}
          {activeTab === "history" && (
            <div className="flex-1 flex flex-col overflow-hidden">
              {/* History toolbar */}
              <div className="flex items-center gap-3 px-4 py-3 bg-slate-900/50 border-b border-slate-700/40 shrink-0">
                <input type="text" value={historyFilter}
                  onChange={(e) => setHistoryFilter(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && loadHistory()}
                  placeholder="Lọc theo khu vực (Enter để tìm)..."
                  className="flex-1 px-3 py-1.5 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-violet-500" />
                <button onClick={loadHistory} disabled={isLoadingHistory}
                  className="px-3 py-1.5 text-sm bg-violet-600 hover:bg-violet-500 text-white rounded-lg transition-all disabled:opacity-50">
                  {isLoadingHistory ? "⟳" : "Tìm"}
                </button>
                <div className="w-px h-5 bg-slate-700" />
                {!confirmClear ? (
                  <button onClick={() => setConfirmClear(true)}
                    className="px-3 py-1.5 text-xs text-red-400 hover:text-red-300 hover:bg-red-900/20 rounded-lg transition-all border border-red-800/40">
                    🗑 Xóa tất cả
                  </button>
                ) : (
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-red-400">Xác nhận xóa hết DB?</span>
                    <button onClick={clearAll} className="px-2 py-1 text-xs bg-red-600 hover:bg-red-500 text-white rounded">Xóa</button>
                    <button onClick={() => setConfirmClear(false)} className="px-2 py-1 text-xs bg-slate-700 text-slate-300 rounded">Hủy</button>
                  </div>
                )}
              </div>

              {/* History table */}
              <div className="flex-1 overflow-auto">
                {isLoadingHistory ? (
                  <div className="h-full flex items-center justify-center text-slate-500">Đang tải...</div>
                ) : history.length === 0 ? (
                  <div className="h-full flex flex-col items-center justify-center text-slate-600 gap-3">
                    <div className="text-4xl">🗄️</div>
                    <p>Chưa có dữ liệu trong database. Hãy cào dữ liệu trước.</p>
                  </div>
                ) : (
                  <table className="w-full text-sm border-collapse">
                    <thead className="sticky top-0 bg-slate-800 text-slate-300 z-10">
                      <tr>
                        {["#", "Tên khách hàng", "Số điện thoại", "Khu vực", "Từ khóa", "Thời gian cào"].map(h => (
                          <th key={h} className="px-4 py-3 text-left text-xs font-semibold uppercase tracking-wider">{h}</th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {history.map((lead, i) => (
                        <tr key={lead.id} className={`border-t border-slate-800 hover:bg-slate-800/50 ${i % 2 === 0 ? "bg-slate-900/30" : "bg-slate-900/60"}`}>
                          <td className="px-4 py-2 text-slate-500 text-xs w-12">{lead.id}</td>
                          <td className="px-4 py-2 font-medium text-white text-sm">{lead.name || "—"}</td>
                          <td className="px-4 py-2 text-blue-400 font-mono text-xs w-32">{lead.phone || "—"}</td>
                          <td className="px-4 py-2 text-slate-300 text-xs">{lead.region || "—"}</td>
                          <td className="px-4 py-2 text-violet-400 text-xs">{lead.keyword || "—"}</td>
                          <td className="px-4 py-2 text-slate-500 text-xs whitespace-nowrap">{lead.scraped_at}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            </div>
          )}
          {/* ── Zalo CRM ── */}
          {activeTab === "zalo" && <ZaloTab />}
        </main>
      </div>
    </div>
  );
}

export default App;
