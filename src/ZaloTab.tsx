import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "./i18n";

interface ZaloUser { name: string; avatar: string; }
interface ZaloStatus { logged_in: boolean; user?: ZaloUser; message: string; session_file: string; }
interface OutreachProgress {
  lead_id: number; lead_name: string; phone: string; email?: string;
  action: string; result: string; done: number; total: number; is_finished: boolean;
}
interface CrmLead {
  id: number; name: string; phone: string; address: string;
  region: string; keyword: string; website: string; email: string; status: string; zalo_found: boolean;
  note: string; scraped_at: string;
  friend_sent_at?: string; friend_accepted_at?: string; first_msg_sent_at?: string;
  last_reply_at?: string;  // FIX: was missing, now matches backend CrmLead struct
  converted_at?: string;   // FIX: was missing, now matches backend CrmLead struct
}
interface CrmStats {
  new: number; sent_friend: number; sent_msg: number;
  friend_accepted: number; replied: number; customer: number;
  rejected: number; today_sent: number; // rejected was already present - OK
}

export default function ZaloTab() {
  const { t } = useI18n();

  const STATUS_CONFIG: Record<string, { label: string; color: string }> = {
    NEW:             { label: t.statusNew,            color: "bg-slate-600 text-slate-200" },
    SENT_EMAIL:      { label: t.statusSentEmail,      color: "bg-indigo-600 text-indigo-100" },
    SENT_FRIEND:     { label: t.statusSentFriend,     color: "bg-yellow-600 text-yellow-100" },
    SENT_MSG:        { label: t.statusSentMsg,        color: "bg-blue-600 text-blue-100" },
    FRIEND_ACCEPTED: { label: t.statusFriendAccepted, color: "bg-emerald-600 text-emerald-100" },
    REPLIED:         { label: t.statusReplied,        color: "bg-violet-600 text-violet-100" },
    CUSTOMER:        { label: t.statusCustomer,       color: "bg-orange-500 text-orange-100" },
    REJECTED:        { label: t.statusRejected,       color: "bg-red-700 text-red-100" },
  };

  const [status, setStatus] = useState<ZaloStatus | null>(null);
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [isChecking, setIsChecking] = useState(true);
  const [qrImage, setQrImage] = useState("");
  const [loginMode, setLoginMode] = useState<"qr"|"cookie">("qr");
  const [crmLeads, setCrmLeads] = useState<CrmLead[]>([]);
  const [crmStats, setCrmStats] = useState<CrmStats | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [statusFilter, setStatusFilter] = useState<string>("ALL");
  const [editingNote, setEditingNote] = useState<{id:number;text:string}|null>(null);
  const [outreachLogs, setOutreachLogs] = useState<string[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [settings, setSettings] = useState<Record<string,string>>({});
  const [activeSection, setActiveSection] = useState<"zalo"|"crm"|"settings"|"email">(() => {
    const urlSec = new URLSearchParams(window.location.search).get("section");
    if (urlSec === "zalo" || urlSec === "crm" || urlSec === "settings" || urlSec === "email") return urlSec;
    return "zalo";
  });
  const [previewKey, setPreviewKey] = useState(0);

  // Login form fields
  const [loginCookie, setLoginCookie] = useState("");
  const [loginImei, setLoginImei] = useState("");
  const [loginUserAgent, setLoginUserAgent] = useState("");
  const [loginError, setLoginError] = useState("");

  useEffect(() => {
    checkSession();
    loadCrm();
    loadSettings();
    const u1 = listen<OutreachProgress>("zalo_outreach_progress", e => {
      const p = e.payload;
      const icon = p.result === "SUCCESS" ? "✓" : "⚠";
      setOutreachLogs(prev => [...prev,
        `[${new Date().toLocaleTimeString("vi-VN")}] ${icon} [${p.done}/${p.total}] ${p.lead_name} - ${p.result}`
      ]);
      if (p.is_finished) { setIsSending(false); loadCrm(); }
    });
    const u3 = listen<OutreachProgress>("email_outreach_progress", e => {
      const p = e.payload;
      const icon = p.result === "Thành công" ? "✓" : "⚠";
      setOutreachLogs(prev => [...prev,
        `[${new Date().toLocaleTimeString("vi-VN")}] ${icon} [${p.done}/${p.total}] ${t.emailToLog} ${p.lead_name} (${p.email}) - ${p.result}`
      ]);
      if (p.is_finished) { setIsSending(false); loadCrm(); }
    });
    const u2 = listen<string>("zalo_qr", e => setQrImage(e.payload));

    // FIX: QR login result comes via event (not invoke response)
    // because the backend runs it in a background thread to keep UI responsive
    const u4 = listen<ZaloStatus>("zalo_login_result", e => {
      setStatus(e.payload);
      if (e.payload.logged_in) setQrImage("");
      setIsLoggingIn(false);
    });
    const u5 = listen<string>("zalo_login_error", e => {
      setLoginError(e.payload);
      setStatus({ logged_in: false, message: `Lỗi: ${e.payload}`, session_file: "" });
      setIsLoggingIn(false);
    });

    return () => { u1.then(f=>f()); u2.then(f=>f()); u3.then(f=>f()); u4.then(f=>f()); u5.then(f=>f()); };
  }, []);

  const checkSession = async () => {
    setIsChecking(true);
    try {
      const s = await invoke<ZaloStatus>("zalo_check_session");
      setStatus(s);
    } catch { } finally { setIsChecking(false); }
  };

  const startQrLogin = async () => {
    setIsLoggingIn(true); setLoginError(""); setQrImage("");
    try {
      // Fire-and-forget: backend spawns background thread, returns immediately.
      // Actual result (ZaloStatus) comes via 'zalo_login_result' or 'zalo_login_error' events.
      await invoke("zalo_start_qr_login");
      // isLoggingIn stays true — will be reset when event arrives
    } catch (e: any) {
      setLoginError(String(e));
      setIsLoggingIn(false);
    }
  };

  const doLogin = async () => {
    if (!loginCookie.trim() || !loginImei.trim() || !loginUserAgent.trim()) {
      setLoginError(t.errMissingCookieFields);
      return;
    }
    setIsLoggingIn(true); setLoginError("");
    try {
      const s = await invoke<ZaloStatus>("zalo_login", {
        cookie: loginCookie.trim(),
        imei: loginImei.trim(),
        userAgent: loginUserAgent.trim(),
      });
      setStatus(s);
      if (s.logged_in) {
        setLoginCookie(""); setLoginImei(""); setLoginUserAgent("");
      }
    } catch (e: any) {
      setLoginError(String(e));
      setStatus({ logged_in: false, message: `Lỗi: ${e}`, session_file: "" });
    } finally { setIsLoggingIn(false); }
  };

  const loadCrm = async () => {
    try {
      const [leads, stats] = await Promise.all([
        invoke<CrmLead[]>("get_crm_leads", { statusFilter: null }),
        invoke<CrmStats>("get_crm_stats"),
      ]);
      setCrmLeads(leads); setCrmStats(stats);
    } catch { }
  };

  const loadSettings = async () => {
    try { setSettings(await invoke<Record<string,string>>("get_settings")); } catch { }
  };

  const updateStatus = async (id: number, newStatus: string) => {
    await invoke("update_lead_status", { id, status: newStatus });
    loadCrm();
  };

  const saveNote = async () => {
    if (!editingNote) return;
    await invoke("update_lead_note", { id: editingNote.id, note: editingNote.text });
    setEditingNote(null); loadCrm();
  };

  const startOutreach = async (action: string) => {
    if (selectedIds.size === 0) return;
    setIsSending(true);
    // FIX: Append new session logs with a header separator instead of resetting.
    // This prevents losing email logs when Zalo action starts immediately after.
    setOutreachLogs(prev => [...prev,
      prev.length > 0 ? "───────────────────────────" : "",
      `[${new Date().toLocaleTimeString("vi-VN")}] ${t.outreachStartLog} ${action} (${selectedIds.size} leads)`
    ].filter(Boolean));
    await invoke("zalo_start_outreach", { leadIds: Array.from(selectedIds), action });
  };

  const startEmailOutreach = async () => {
    if (selectedIds.size === 0) return;
    setIsSending(true);
    // FIX: Append new session logs with a header separator instead of resetting.
    setOutreachLogs(prev => [...prev,
      prev.length > 0 ? "───────────────────────────" : "",
      `[${new Date().toLocaleTimeString("vi-VN")}] ${t.outreachEmailStartLog} (${selectedIds.size} leads)`
    ].filter(Boolean));
    try {
      await invoke("email_start_outreach", { leadIds: Array.from(selectedIds) });
    } catch (e: any) {
      alert(`Error: ${e}`);
      setIsSending(false);
    }
  };

  const doSaveSettings = async () => {
    await invoke("save_settings", { settings });
    alert(t.settingsSavedAlert);
  };

  const getPreviewHtml = () => {
    let text = settings["email_body"] || "";
    let preview = text.replace(/\{([^{}]+)\}/g, (_match, p1) => {
      const options = p1.split('|');
      return options[Math.floor(Math.random() * options.length)];
    });
    preview = preview.replace(/\[Tên\]/g, t.previewSampleName);
    preview = preview.replace(/\[Website\]/g, "https://vidu.com");
    return preview;
  };

  const filteredLeads = statusFilter === "ALL" ? crmLeads : crmLeads.filter(l => l.status === statusFilter);

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Sub-nav */}
      <div className="flex gap-1 px-4 py-2 bg-slate-900/70 border-b border-slate-700/40 shrink-0">
        {([
          ["zalo", t.subNavZalo],
          ["crm", t.subNavCrm],
          ["email", t.subNavEmail],
          ["settings", t.subNavSettings],
        ] as const).map(([k,l]) => (
          <button key={k} onClick={() => setActiveSection(k)}
            className={`px-4 py-1.5 rounded-lg text-sm font-medium transition-all ${activeSection === k ? "bg-blue-600 text-white" : "text-slate-400 hover:text-white hover:bg-slate-700"}`}>
            {l}
          </button>
        ))}
        {/* Live status pill */}
        <div className="ml-auto flex items-center gap-2 text-xs">
          {isChecking ? (
            <span className="text-slate-500">{t.checkingStatus}</span>
          ) : status?.logged_in ? (
            <span className="flex items-center gap-1.5 text-emerald-400">
              <span className="w-2 h-2 bg-emerald-400 rounded-full animate-pulse"/>
              {status.user?.name || t.loggedInDefault}
            </span>
          ) : (
            <span className="flex items-center gap-1.5 text-red-400">
              <span className="w-2 h-2 bg-red-400 rounded-full"/>
              {t.notConnected}
            </span>
          )}
          <button onClick={checkSession} className="px-2 py-1 bg-slate-700 hover:bg-slate-600 rounded text-slate-300 transition-all">↻</button>
        </div>
      </div>

      {/* ── ZALO LOGIN SECTION ── */}
      {activeSection === "zalo" && (
        <div className="flex-1 overflow-y-auto p-6">
          <div className="max-w-xl mx-auto space-y-5">
            <h2 className="text-lg font-semibold text-white">{t.zaloConnectTitle}</h2>

            {/* Status card */}
            <div className={`rounded-xl p-5 border ${status?.logged_in ? "bg-emerald-900/20 border-emerald-700/40" : "bg-slate-800 border-slate-700"}`}>
              {status?.logged_in ? (
                <div className="flex items-center gap-4">
                  <div className="w-14 h-14 rounded-full bg-emerald-700 flex items-center justify-center text-2xl shrink-0">👤</div>
                  <div>
                    <p className="font-bold text-white text-lg">{status.user?.name || "Zalo User"}</p>
                    <p className="text-emerald-400 text-sm">{t.zaloConnectedSuccess}</p>
                    <p className="text-slate-500 text-xs mt-1">Session: {status.session_file}</p>
                  </div>
                </div>
              ) : (
                <div className="space-y-1">
                  <p className="text-slate-300 text-sm">{status?.message || t.zaloNotLoggedIn}</p>
                </div>
              )}
            </div>

            {/* Login Form — only shown when not logged in */}
            {!status?.logged_in && (
              <div className="bg-slate-800 rounded-xl p-5 border border-slate-700 space-y-4">
                {/* Login mode tabs */}
                <div className="flex gap-2">
                  <button onClick={() => setLoginMode("qr")}
                    className={`flex-1 py-2 rounded-lg text-sm font-medium transition-all ${
                      loginMode === "qr" ? "bg-blue-600 text-white" : "bg-slate-700 text-slate-400 hover:text-white"
                    }`}>
                    {t.qrLoginTab}
                  </button>
                  <button onClick={() => setLoginMode("cookie")}
                    className={`flex-1 py-2 rounded-lg text-sm font-medium transition-all ${
                      loginMode === "cookie" ? "bg-blue-600 text-white" : "bg-slate-700 text-slate-400 hover:text-white"
                    }`}>
                    {t.cookieLoginTab}
                  </button>
                </div>

                {/* QR Login Mode */}
                {loginMode === "qr" && (
                  <div className="space-y-4">
                    {qrImage ? (
                      <div className="text-center space-y-3">
                        <p className="text-slate-300 font-medium">{t.scanQrInstruction}</p>
                        <img src={qrImage} alt="Zalo QR" className="mx-auto rounded-lg max-w-[250px] bg-white p-3"/>
                        <p className="text-xs text-slate-500">{t.scanQrSubInstruction}</p>
                      </div>
                    ) : (
                      <div className="text-center space-y-3">
                        {isLoggingIn ? (
                          <div className="py-8">
                            <div className="w-[200px] h-[200px] mx-auto bg-slate-700 rounded-lg flex items-center justify-center">
                              <span className="text-4xl animate-spin">⟳</span>
                            </div>
                            <p className="text-slate-400 text-sm mt-3">{t.generatingQr}</p>
                          </div>
                        ) : (
                          <>
                            <p className="text-slate-400 text-sm">{t.clickToGenerateQr}</p>
                            <button onClick={startQrLogin} disabled={isLoggingIn}
                              className="px-8 py-3 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white rounded-lg font-semibold transition-all shadow-lg">
                              {t.generateQrBtn}
                            </button>
                          </>
                        )}
                      </div>
                    )}

                    {loginError && (
                      <div className="bg-red-900/30 border border-red-700/40 rounded-lg p-3 text-sm text-red-400">
                        ❌ {loginError}
                      </div>
                    )}
                  </div>
                )}

                {/* Cookie Login Mode */}
                {loginMode === "cookie" && (
                  <div className="space-y-4">
                    <h3 className="text-base font-semibold text-white">{t.cookieLoginTitle}</h3>

                <div className="bg-blue-900/20 border border-blue-700/40 rounded-lg p-3 text-xs text-blue-300 space-y-1">
                  <p><strong>{t.howToGetCredentials}</strong></p>
                  <p>{t.cookieStep1Prefix} <a href="https://chat.zalo.me" target="_blank" className="underline text-blue-400">chat.zalo.me</a> {t.cookieStep1Suffix}</p>
                  <p>{t.cookieStep2}</p>
                  <p>{t.cookieStep3Prefix} <code className="bg-slate-700 px-1 rounded">localStorage.getItem('z_uuid')</code></p>
                  <p>{t.cookieStep4Prefix} <code className="bg-slate-700 px-1 rounded">navigator.userAgent</code></p>
                  <p>{t.cookieStep5Prefix} <a href="https://chromewebstore.google.com/detail/j2team-cookies/okpidcojinmlaakglciglbpcpajaibco" target="_blank" className="underline text-blue-400">J2TEAM Cookies</a> {t.orText} <a href="https://github.com/JustKemForFun/ZaloDataExtractor" target="_blank" className="underline text-blue-400">ZaloDataExtractor</a></p>
                </div>

                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-1">IMEI (z_uuid)</label>
                  <input value={loginImei} onChange={e => setLoginImei(e.target.value)}
                    className="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                    placeholder={t.imeiPlaceholder}/>
                </div>

                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-1">User Agent</label>
                  <input value={loginUserAgent} onChange={e => setLoginUserAgent(e.target.value)}
                    className="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                    placeholder={t.userAgentPlaceholder}/>
                </div>

                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-1">Cookie (JSON)</label>
                  <textarea rows={4} value={loginCookie} onChange={e => setLoginCookie(e.target.value)}
                    className="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none font-mono"
                    placeholder={t.cookieJsonPlaceholder}/>
                </div>

                {loginError && (
                  <div className="bg-red-900/30 border border-red-700/40 rounded-lg p-3 text-sm text-red-400">
                    ❌ {loginError}
                  </div>
                )}

                    <button onClick={doLogin} disabled={isLoggingIn}
                      className="w-full py-2.5 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white rounded-lg font-semibold transition-all shadow-lg">
                      {isLoggingIn ? t.loggingInBtn : t.loginZaloBtn}
                    </button>
                  </div>
                )}
              </div>
            )}

            {/* Info box */}
            <div className="bg-slate-800/50 rounded-lg p-4 border border-slate-700/40 text-sm text-slate-400 space-y-1">
              <p>💾 <strong className="text-slate-200">{t.zaloInfoAutoSave}</strong> {t.zaloInfoAutoSaveDesc}</p>
              <p>⚡ <strong className="text-slate-200">{t.zaloInfoNoBrowser}</strong> {t.zaloInfoNoBrowserDesc}</p>
              <p>{t.zaloInfoSessionExpire}</p>
            </div>
          </div>
        </div>
      )}

      {/* ── CRM SECTION ── */}
      {activeSection === "crm" && (
        <div className="flex-1 flex flex-col overflow-hidden">
          {/* Stats funnel */}
          {crmStats && (
            <div className="flex items-center gap-3 px-4 py-3 bg-slate-900/50 border-b border-slate-700/40 shrink-0 overflow-x-auto">
              {[
                [t.statusNew, crmStats.new, "text-slate-300"],
                [t.statusSentEmail, crmLeads.filter(l => l.status === "SENT_EMAIL").length, "text-indigo-400"],
                [t.statusSentFriendFunnel, crmStats.sent_friend, "text-yellow-400"],
                [t.statusSentMsgFunnel, crmStats.sent_msg, "text-blue-400"],
                [t.statusFriendAccepted, crmStats.friend_accepted, "text-emerald-400"],
                [t.statusReplied, crmStats.replied, "text-violet-400"],
                [t.statusCustomer, crmStats.customer, "text-orange-400"],
              ].map(([label, count, cls]) => (
                <div key={label as string} className="flex flex-col items-center shrink-0">
                  <span className={`text-xl font-bold ${cls}`}>{count as number}</span>
                  <span className="text-xs text-slate-500">{label as string}</span>
                </div>
              ))}
              <div className="ml-auto text-xs text-amber-400 shrink-0">
                {t.todaySentLabel} <strong>{crmStats.today_sent}</strong>{t.perDayUnit}
              </div>
            </div>
          )}

          {/* Toolbar */}
          <div className="flex items-center gap-2 px-4 py-2 bg-slate-900/30 border-b border-slate-700/30 shrink-0 flex-wrap">
            <select value={statusFilter} onChange={e => setStatusFilter(e.target.value)}
              className="px-3 py-1.5 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white focus:outline-none">
              <option value="ALL">{t.filterAll}</option>
              {Object.entries(STATUS_CONFIG).map(([k,v]) => <option key={k} value={k}>{v.label}</option>)}
            </select>
            <span className="text-xs text-slate-500">{selectedIds.size} {t.selectedCountSuffix}</span>
            <button onClick={() => {
              const visible = new Set(filteredLeads.map(l => l.id));
              setSelectedIds(prev => prev.size === visible.size ? new Set() : visible);
            }} className="px-3 py-1.5 text-xs bg-slate-700 hover:bg-slate-600 text-slate-300 rounded-lg transition-all">
              {t.selectAllBtn}
            </button>
            <div className="flex gap-2 ml-auto">
              <button onClick={startEmailOutreach} disabled={isSending || selectedIds.size === 0}
                className="px-3 py-1.5 text-xs bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg disabled:opacity-40 transition-all">
                {t.sendEmailBtn} ({selectedIds.size})
              </button>
              <button onClick={() => startOutreach("FRIEND")} disabled={!status?.logged_in || isSending || selectedIds.size === 0}
                className="px-3 py-1.5 text-xs bg-yellow-600 hover:bg-yellow-500 text-white rounded-lg disabled:opacity-40 transition-all">
                {t.sendFriendBtn} ({selectedIds.size})
              </button>
              <button onClick={() => startOutreach("MESSAGE")} disabled={!status?.logged_in || isSending || selectedIds.size === 0}
                className="px-3 py-1.5 text-xs bg-blue-600 hover:bg-blue-500 text-white rounded-lg disabled:opacity-40 transition-all">
                {t.sendMsgBtn} ({selectedIds.size})
              </button>
              <button onClick={loadCrm} className="px-3 py-1.5 text-xs bg-slate-700 hover:bg-slate-600 text-slate-300 rounded-lg">↻</button>
            </div>
          </div>

          {/* Outreach log */}
          {(isSending || outreachLogs.length > 0) && (
            <div className="px-4 py-2 bg-slate-900 border-b border-slate-700/40 shrink-0 max-h-24 overflow-y-auto font-mono text-xs">
              {isSending && <div className="text-yellow-400 flex items-center gap-2"><span className="animate-spin">⟳</span> {t.sendingProgress}</div>}
              {outreachLogs.slice(-5).map((l,i) => (
                <div key={i} className={l.includes("✓") ? "text-emerald-400" : "text-amber-400"}>{l}</div>
              ))}
            </div>
          )}

          {/* Leads table */}
          <div className="flex-1 overflow-x-auto">
            <table className="w-full text-sm border-collapse min-w-[1200px]">
              <thead className="sticky top-0 bg-slate-800 z-10 shadow-sm">
                <tr>
                  <th className="px-3 py-2.5 w-8"/>
                  <th className="px-3 py-2.5 text-left text-xs font-semibold text-slate-300 uppercase tracking-wider">{t.colShortName}</th>
                  <th className="px-3 py-2.5 text-left text-xs font-semibold text-slate-300 uppercase tracking-wider w-32 shrink-0">{t.colShortPhone}</th>
                  <th className="px-3 py-2.5 text-left text-xs font-semibold text-slate-300 uppercase tracking-wider">{t.colEmail}</th>
                  <th className="px-3 py-2.5 text-left text-xs font-semibold text-slate-300 uppercase tracking-wider">{t.colWebsite}</th>
                  <th className="px-3 py-2.5 text-left text-xs font-semibold text-slate-300 uppercase tracking-wider w-32 shrink-0">{t.colRegion}</th>
                  <th className="px-3 py-2.5 text-left text-xs font-semibold text-slate-300 uppercase tracking-wider w-36 shrink-0">{t.colStatus}</th>
                  <th className="px-3 py-2.5 text-left text-xs font-semibold text-slate-300 uppercase tracking-wider w-48 shrink-0">{t.colNote}</th>
                </tr>
              </thead>
              <tbody>
                {filteredLeads.map((lead, i) => {
                  const cfg = STATUS_CONFIG[lead.status] || STATUS_CONFIG.NEW;
                  const isSelected = selectedIds.has(lead.id);
                  return (
                    <tr key={lead.id} className={`border-t border-slate-800 hover:bg-slate-800/40 transition-colors ${i%2===0?"bg-slate-900/20":"bg-slate-900/50"} ${isSelected?"ring-1 ring-inset ring-blue-500":""}`}>
                      <td className="px-3 py-2 text-center shrink-0">
                        <input type="checkbox" checked={isSelected} onChange={e => {
                          const next = new Set(selectedIds);
                          e.target.checked ? next.add(lead.id) : next.delete(lead.id);
                          setSelectedIds(next);
                        }} className="w-3.5 h-3.5"/>
                      </td>
                      <td className="px-3 py-2 font-medium text-white text-sm whitespace-nowrap overflow-hidden text-ellipsis max-w-[200px]" title={lead.name}>{lead.name || "—"}</td>
                      <td className="px-3 py-2 text-blue-400 font-mono text-xs shrink-0">{lead.phone || "—"}</td>
                      <td className="px-3 py-2 text-amber-400 text-xs whitespace-nowrap overflow-hidden text-ellipsis max-w-[200px]" title={lead.email}>{lead.email || "—"}</td>
                      <td className="px-3 py-2 text-slate-300 text-xs whitespace-nowrap overflow-hidden text-ellipsis max-w-[150px]">
                        {lead.website ? <a href={lead.website} target="_blank" className="text-blue-400 hover:underline">Link</a> : "—"}
                      </td>
                      <td className="px-3 py-2 text-slate-300 text-xs whitespace-nowrap overflow-hidden text-ellipsis max-w-[150px]" title={lead.region}>{lead.region || "—"}</td>
                      <td className="px-3 py-2 shrink-0">
                        <select value={lead.status} onChange={e => updateStatus(lead.id, e.target.value)}
                          className={`text-xs px-2 py-1 rounded-full border-0 cursor-pointer ${cfg.color} focus:outline-none`}>
                          {Object.entries(STATUS_CONFIG).map(([k,v]) => <option key={k} value={k}>{v.label}</option>)}
                        </select>
                      </td>
                      <td className="px-3 py-2 shrink-0">
                        {editingNote?.id === lead.id ? (
                          <div className="flex gap-1">
                            <input value={editingNote.text} onChange={e => setEditingNote({id:lead.id,text:e.target.value})}
                              className="flex-1 px-2 py-1 bg-slate-700 rounded text-xs text-white focus:outline-none focus:ring-1 focus:ring-blue-500"/>
                            <button onClick={saveNote} className="px-2 py-1 bg-emerald-600 text-white text-xs rounded">✓</button>
                            <button onClick={() => setEditingNote(null)} className="px-2 py-1 bg-slate-600 text-white text-xs rounded">✕</button>
                          </div>
                        ) : (
                          <span onClick={() => setEditingNote({id:lead.id, text:lead.note})}
                            className="text-xs text-slate-400 hover:text-white cursor-text block truncate max-w-[180px]">
                            {lead.note || <span className="text-slate-600 italic">{t.addNotePlaceholder}</span>}
                          </span>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
            {filteredLeads.length === 0 && (
              <div className="h-40 flex items-center justify-center text-slate-600">
                {t.noLeadsWithFilter}
              </div>
            )}
          </div>
        </div>
      )}

      {/* ── EMAIL OUTREACH SECTION ── */}
      {activeSection === "email" && (
        <div className="flex-1 flex flex-col overflow-hidden bg-slate-900/30 p-6">
          <div className="max-w-5xl mx-auto w-full space-y-6">
            <h2 className="text-lg font-semibold text-white">{t.emailOutreachTitle}</h2>
            <div className="grid grid-cols-2 gap-6">
              <div className="bg-slate-800 border border-slate-700 rounded-xl p-5 space-y-4">
                <h3 className="font-medium text-slate-200">{t.smtpConfigTitle}</h3>
                <div>
                  <label className="block text-xs font-medium text-slate-400 mb-1">{t.smtpEmailLabel}</label>
                  <input type="text" value={settings["smtp_email"] || ""}
                    onChange={e => setSettings(s => ({...s, smtp_email: e.target.value}))}
                    placeholder="vidu@gmail.com"
                    className="w-full px-3 py-2 bg-slate-900 border border-slate-600 rounded-lg text-sm text-white focus:ring-2 focus:ring-blue-500" />
                </div>
                <div>
                  <label className="block text-xs font-medium text-slate-400 mb-1">{t.smtpPassLabel}</label>
                  <input type="password" value={settings["smtp_password"] || ""}
                    onChange={e => setSettings(s => ({...s, smtp_password: e.target.value}))}
                    placeholder="xxxx xxxx xxxx xxxx"
                    className="w-full px-3 py-2 bg-slate-900 border border-slate-600 rounded-lg text-sm text-white focus:ring-2 focus:ring-blue-500" />
                  <div className="mt-3 p-3 bg-slate-900/50 rounded-lg border border-amber-900/30">
                    <p className="text-[11px] font-medium text-amber-400 mb-1">{t.appPassGuideTitle}</p>
                    <ol className="text-[10px] text-slate-400 list-decimal pl-4 space-y-1">
                      <li>{t.appPassStep1}</li>
                      <li>{t.appPassStep2}</li>
                      <li>{t.appPassStep3}</li>
                      <li>{t.appPassStep4}</li>
                    </ol>
                  </div>
                </div>
                <div className="pt-2">
                  <button onClick={doSaveSettings} className="px-4 py-2 bg-slate-700 hover:bg-slate-600 text-white rounded-lg text-sm transition-all">{t.saveConfigBtn}</button>
                </div>
              </div>

              <div className="bg-slate-800 border border-slate-700 rounded-xl p-5 space-y-4">
                <h3 className="font-medium text-slate-200">{t.composeEmailTitle}</h3>
                <div>
                  <label className="block text-xs font-medium text-slate-400 mb-1">{t.emailSubjectLabel}</label>
                  <input type="text" value={settings["email_subject"] || ""}
                    onChange={e => setSettings(s => ({...s, email_subject: e.target.value}))}
                    placeholder={t.emailSubjectPlaceholder}
                    className="w-full px-3 py-2 bg-slate-900 border border-slate-600 rounded-lg text-sm text-white focus:ring-2 focus:ring-blue-500" />
                </div>
                <div>
                  <label className="block text-xs font-medium text-slate-400 mb-1">{t.emailBodyLabel}</label>
                  <textarea rows={6} value={settings["email_body"] || ""}
                    onChange={e => setSettings(s => ({...s, email_body: e.target.value}))}
                    className="w-full px-3 py-2 bg-slate-900 border border-slate-600 rounded-lg text-sm text-white focus:ring-2 focus:ring-blue-500 resize-none font-mono"
                    placeholder={t.emailBodyPlaceholder}></textarea>
                </div>
                <div className="flex justify-between items-center">
                  <p className="text-[10px] text-slate-500">{t.supportedVarsLabel} <code className="text-blue-400">[Tên]</code>, <code className="text-blue-400">[Website]</code></p>
                  <button onClick={() => setPreviewKey(k => k + 1)} className="px-3 py-1.5 bg-slate-700 hover:bg-slate-600 rounded text-xs text-white">{t.spinPreviewBtn}</button>
                </div>

                <div className="mt-4 pt-4 border-t border-slate-700/50">
                  <label className="block text-xs font-medium text-amber-400 mb-2">{t.previewLabel}</label>
                  {/* FIX: Sanitize HTML to prevent XSS before rendering.
                      Note: DOMPurify should be added: npm install dompurify @types/dompurify */}
                  <div key={previewKey}
                    className="bg-slate-950 p-4 rounded-lg text-sm text-slate-300 border border-slate-800 prose prose-invert max-w-none"
                    dangerouslySetInnerHTML={{__html: (() => {
                      const html = getPreviewHtml();
                      // Basic sanitization: strip <script> tags and event handlers
                      // For production, use: DOMPurify.sanitize(html)
                      return html
                        .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '')
                        .replace(/\son\w+\s*=\s*["'][^"']*["']/gi, '');
                    })()}}>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* ── SETTINGS SECTION ── */}
      {activeSection === "settings" && (
        <div className="flex-1 overflow-y-auto p-6">
          <div className="max-w-xl mx-auto space-y-6">
            <h2 className="text-lg font-semibold text-white">{t.zaloSettingsTitle}</h2>

            <div className="bg-slate-800 rounded-xl p-5 border border-slate-700 space-y-4">
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">{t.firstMsgTemplateLabel}</label>
                <textarea rows={4} value={settings["zalo_msg_template"] || ""}
                  onChange={e => setSettings(s => ({...s, zalo_msg_template: e.target.value}))}
                  className="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
                  placeholder={t.firstMsgTemplatePlaceholder}/>
                <p className="text-xs text-slate-500 mt-1">{t.useVarPrefix} <code className="text-blue-400">[Tên]</code> {t.firstMsgTemplateHint}</p>
              </div>

              <div className="grid grid-cols-3 gap-4">
                {[
                  [t.delayMinLabel, "zalo_delay_min", "60"],
                  [t.delayMaxLabel, "zalo_delay_max", "120"],
                  [t.dailyLimitLabel, "zalo_daily_limit", "20"],
                ].map(([label, key, placeholder]) => (
                  <div key={key}>
                    <label className="block text-xs font-medium text-slate-300 mb-1">{label}</label>
                    <input type="number" value={settings[key] || ""}
                      onChange={e => setSettings(s => ({...s, [key]: e.target.value}))}
                      className="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                      placeholder={placeholder}/>
                  </div>
                ))}
              </div>
            </div>

            <div className="bg-amber-900/20 border border-amber-700/40 rounded-lg p-4 text-sm text-amber-300 space-y-1">
              <p><strong>{t.zaloLimitWarnTitle}</strong></p>
              <p>{t.zaloLimitWarn1}</p>
              <p>{t.zaloLimitWarn2}</p>
              <p>{t.zaloLimitWarn3}</p>
            </div>

            <button onClick={doSaveSettings}
              className="w-full py-2.5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg font-semibold transition-all shadow-lg">
              {t.saveSettingsBtn}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
