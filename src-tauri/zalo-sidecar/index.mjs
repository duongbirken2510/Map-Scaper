/**
 * Zalo Sidecar — Node.js process using zca-js
 * 
 * Communication: JSON lines over stdin/stdout
 * Each line = one JSON command/response
 * 
 * Commands:
 *   { cmd: "login", cookie, imei, userAgent }
 *   { cmd: "login_saved" }
 *   { cmd: "login_qr" }
 *   { cmd: "check" }
 *   { cmd: "find_user", phone }
 *   { cmd: "send_friend", uid, message }
 *   { cmd: "send_message", uid, message, threadId }
 *   { cmd: "get_account" }
 *   { cmd: "exit" }
 */

import { Zalo } from "zca-js";
import { createInterface } from "readline";
import { readFileSync, writeFileSync, existsSync, watchFile, unwatchFile } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));

let api = null;
let zaloInstance = null;
let isLoggedIn = false;

// Session file path (passed as CLI arg or default)
const sessionDir = process.argv[2] || __dirname;
const credentialsPath = join(sessionDir, "zalo_credentials.json");
const qrPath = join(sessionDir, "zalo_qr.png");

function respond(id, data) {
  const msg = JSON.stringify({ id, ...data });
  process.stdout.write(msg + "\n");
}

function respondError(id, error) {
  respond(id, { ok: false, error: String(error) });
}

async function handleCommand(line) {
  let cmd;
  try {
    cmd = JSON.parse(line);
  } catch {
    return;
  }

  const { id } = cmd;

  try {
    switch (cmd.cmd) {
      case "login": {
        // Save credentials for reuse
        const creds = {
          cookie: cmd.cookie,
          imei: cmd.imei,
          userAgent: cmd.userAgent,
        };
        writeFileSync(credentialsPath, JSON.stringify(creds, null, 2));

        zaloInstance = new Zalo({
          selfListen: false,
          checkUpdate: false,
          logging: false,
        });

        api = await zaloInstance.login({
          cookie: typeof cmd.cookie === "string" ? JSON.parse(cmd.cookie) : cmd.cookie,
          imei: cmd.imei,
          userAgent: cmd.userAgent,
        });

        isLoggedIn = true;

        // Get account info
        let name = "Đã đăng nhập";
        try {
          const info = await api.fetchAccountInfo();
          if (info?.profile?.displayName) name = info.profile.displayName;
          else if (info?.profile?.zaloName) name = info.profile.zaloName;
        } catch {}

        respond(id, { ok: true, loggedIn: true, name });
        break;
      }

      case "login_saved": {
        // Try to login with saved credentials
        if (!existsSync(credentialsPath)) {
          respond(id, { ok: false, error: "Chưa có thông tin đăng nhập" });
          break;
        }

        const saved = JSON.parse(readFileSync(credentialsPath, "utf-8"));
        zaloInstance = new Zalo({
          selfListen: false,
          checkUpdate: false,
          logging: false,
        });

        api = await zaloInstance.login({
          cookie: typeof saved.cookie === "string" ? JSON.parse(saved.cookie) : saved.cookie,
          imei: saved.imei,
          userAgent: saved.userAgent,
        });

        isLoggedIn = true;

        let name = "Đã đăng nhập";
        try {
          const info = await api.fetchAccountInfo();
          if (info?.profile?.displayName) name = info.profile.displayName;
          else if (info?.profile?.zaloName) name = info.profile.zaloName;
        } catch {}

        respond(id, { ok: true, loggedIn: true, name });
        break;
      }

      case "login_qr": {
        // QR Code login
        // Step 1: Create Zalo instance
        zaloInstance = new Zalo({
          selfListen: false,
          checkUpdate: false,
          logging: false,
        });

        // Step 2: Send "qr_generating" status so frontend knows it started
        respond(id + "_status", { ok: true, status: "generating_qr" });

        // Step 3: loginQR will save QR image to qrPath
        // We watch the file to send it to frontend when ready
        let qrSent = false;
        const checkQR = () => {
          if (qrSent) return;
          try {
            if (existsSync(qrPath)) {
              const qrData = readFileSync(qrPath);
              const qrBase64 = "data:image/png;base64," + qrData.toString("base64");
              respond(id + "_qr", { ok: true, qrImage: qrBase64 });
              qrSent = true;
            }
          } catch {}
        };

        // Watch for QR file creation
        const watchInterval = setInterval(checkQR, 500);

        try {
          api = await zaloInstance.loginQR({
            qrPath: qrPath,
            userAgent: cmd.userAgent || "",
          });

          clearInterval(watchInterval);
          isLoggedIn = true;

          // After successful QR login, save session for future reuse
          // Get cookie from api
          let name = "Đã đăng nhập";
          try {
            const info = await api.fetchAccountInfo();
            if (info?.profile?.displayName) name = info.profile.displayName;
            else if (info?.profile?.zaloName) name = info.profile.zaloName;
          } catch {}

          // Try to save cookie for later reuse
          try {
            const cookie = await api.getCookie();
            const context = api.getContext();
            if (cookie || context) {
              writeFileSync(credentialsPath, JSON.stringify({
                cookie: cookie || [],
                imei: context?.imei || "",
                userAgent: context?.userAgent || cmd.userAgent || "",
              }, null, 2));
            }
          } catch {}

          respond(id, { ok: true, loggedIn: true, name });
        } catch (err) {
          clearInterval(watchInterval);
          respondError(id, err.message || String(err));
        }
        break;
      }

      case "check": {
        if (!isLoggedIn || !api) {
          respond(id, { ok: true, loggedIn: false });
          break;
        }
        try {
          const ownId = await api.getOwnId();
          respond(id, { ok: true, loggedIn: true, ownId });
        } catch {
          isLoggedIn = false;
          respond(id, { ok: true, loggedIn: false });
        }
        break;
      }

      case "get_account": {
        if (!api) {
          respondError(id, "Chưa đăng nhập");
          break;
        }
        const info = await api.fetchAccountInfo();
        respond(id, { ok: true, info });
        break;
      }

      case "find_user": {
        if (!api) {
          respondError(id, "Chưa đăng nhập");
          break;
        }
        // Normalize phone: remove leading 84, add 0
        let phone = cmd.phone.replace(/\s/g, "");
        if (phone.startsWith("84") && phone.length > 9) {
          phone = "0" + phone.slice(2);
        }
        if (!phone.startsWith("0") && phone.length === 9) {
          phone = "0" + phone;
        }

        const user = await api.findUser(phone);
        if (user && user.uid) {
          respond(id, {
            ok: true,
            found: true,
            uid: user.uid,
            displayName: user.display_name || user.zalo_name || "",
            avatar: user.avatar || "",
          });
        } else {
          respond(id, { ok: true, found: false });
        }
        break;
      }

      case "send_friend": {
        if (!api) {
          respondError(id, "Chưa đăng nhập");
          break;
        }
        // API: api.sendFriendRequest(msg, userId) — msg FIRST, userId SECOND
        await api.sendFriendRequest(cmd.message || "Xin chào!", cmd.uid);
        respond(id, { ok: true, result: "Đã gửi lời mời kết bạn" });
        break;
      }

      case "send_message": {
        if (!api) {
          respondError(id, "Chưa đăng nhập");
          break;
        }
        // API: api.sendMessage(message, threadId, type)
        // message can be string or {msg: string}
        // type: ThreadType.User = 0 (default), ThreadType.Group = 1
        await api.sendMessage(
          cmd.message,
          cmd.threadId || cmd.uid,
          0 // ThreadType.User (direct message)
        );
        respond(id, { ok: true, result: "Đã gửi tin nhắn" });
        break;
      }

      case "exit": {
        respond(id, { ok: true });
        process.exit(0);
        break;
      }

      default:
        respondError(id, `Unknown command: ${cmd.cmd}`);
    }
  } catch (err) {
    respondError(id, err.message || String(err));
  }
}

// Read commands from stdin
const rl = createInterface({ input: process.stdin });
rl.on("line", handleCommand);

// Signal ready
respond("_ready", { ok: true, ready: true });
