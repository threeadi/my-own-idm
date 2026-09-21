const NATIVE_HOST = "com.myownidm.host";

// State
let isInterceptorEnabled = true;
let detectedMediaByTab = {};

// 1. Setup Context Menus on Install
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "download-with-myownidm",
    title: "Unduh dengan IDM Turbo",
    contexts: ["link", "video", "audio", "image"]
  });
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === "download-with-myownidm") {
    const targetUrl = info.linkUrl || info.srcUrl;
    if (targetUrl) {
      sendToMyOwnIdm(targetUrl, tab?.url);
    }
  }
});

function extractFilenameFromUrl(urlStr) {
  try {
    const parsed = new URL(urlStr);
    const pathname = parsed.pathname;
    const segments = pathname.split("/").filter(Boolean);
    const last = segments[segments.length - 1];
    if (last && last.includes(".")) {
      return decodeURIComponent(last);
    }
  } catch (e) {}
  return "";
}

// 2. Intercept Browser Downloads
chrome.downloads.onCreated.addListener(async (item) => {
  if (!isInterceptorEnabled) return;

  // Ignore data URLs and extension internal downloads
  if (!item.url || item.url.startsWith("blob:") || item.url.startsWith("data:")) return;

  // Pause and cancel browser download immediately
  try {
    await chrome.downloads.cancel(item.id);
    await chrome.downloads.erase({ id: item.id });
  } catch (e) {
    // Ignore if already cancelled
  }

  // Forward to desktop client with fallback URL filename
  const safeFilename = item.filename || extractFilenameFromUrl(item.url);
  sendToMyOwnIdm(item.url, item.referrer, safeFilename);
});

// 3. Media Sniffer (Video/Audio/m3u8 stream detection)
const STREAM_EXTENSIONS = [".m3u8", "/pl/", "playlist.m3u8", "master.m3u8", ".urlset", "master.txt", "index.txt"];
const FILE_EXTENSIONS = [".mp4", ".mkv", ".webm", ".flv", ".mp3", ".aac", ".ogg"];
const NON_MEDIA_EXTENSIONS = [
  ".js", ".mjs", ".css", ".json", ".map", ".wasm",
  ".html", ".htm", ".svg", ".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".woff", ".woff2", ".ttf"
];

chrome.webRequest?.onResponseStarted?.addListener(
  (details) => {
    if (details.tabId < 0) return;
    const url = details.url.toLowerCase();

    // STRICTLY IGNORE scripts, stylesheets, fonts, and static non-media assets
    if (NON_MEDIA_EXTENSIONS.some(ext => {
      const idx = url.indexOf(ext);
      if (idx === -1) return false;
      const nextChar = url.charAt(idx + ext.length);
      return nextChar === "" || nextChar === "?" || nextChar === "#" || nextChar === "/";
    })) {
      return;
    }

    // STRICTLY IGNORE fragment chunks (like .m4s, .ts segments, internal SABR videoplayback)
    if (
      url.includes(".m4s") ||
      url.includes(".ts?") ||
      url.endsWith(".ts") ||
      url.includes("videoplayback") ||
      url.includes("/segment") ||
      url.includes("range=") ||
      url.includes("/avc1/") ||
      url.match(/\/\d+\/\d+\/\d+$/)
    ) {
      return;
    }

    const contentTypeHeader = details.responseHeaders?.find(
      h => h.name.toLowerCase() === "content-type"
    );
    const ct = contentTypeHeader?.value?.toLowerCase() || "";
    if (ct.includes("javascript") || ct.includes("json") || ct.includes("css") || ct.includes("text/html")) {
      return;
    }

    const isStream = STREAM_EXTENSIONS.some(ext => url.includes(ext));
    const isFile = FILE_EXTENSIONS.some(ext => url.includes(ext));
    const isStreamMime = ct.includes("mpegurl");

    if (isStream || isStreamMime || isFile) {
      if (!detectedMediaByTab[details.tabId]) {
        detectedMediaByTab[details.tabId] = [];
      }

      // Avoid duplicates
      if (!detectedMediaByTab[details.tabId].some(m => m.url === details.url)) {
        detectedMediaByTab[details.tabId].push({
          url: details.url,
          type: (isStream || isStreamMime) ? "stream" : "media",
          timestamp: Date.now()
        });

        // Update badge
        const count = detectedMediaByTab[details.tabId].length;
        chrome.action.setBadgeText({ tabId: details.tabId, text: String(count) });
        chrome.action.setBadgeBackgroundColor({ tabId: details.tabId, color: "#00e5ff" });
      }
    }
  },
  { urls: ["<all_urls>"] },
  ["responseHeaders"]
);

// Clean up closed tabs
chrome.tabs.onRemoved.addListener((tabId) => {
  delete detectedMediaByTab[tabId];
});

// Helpers for smart filename resolution
function cleanFilename(rawTitle, defaultName = "video", maxLen = 80) {
  if (!rawTitle) return defaultName;
  let clean = rawTitle
    .replace(/\s*-\s*YouTube$/i, "")
    .replace(/\s*\/\s*X$/i, "")
    .replace(/\s*-\s*Twitter$/i, "")
    .replace(/\s*•\s*Instagram.*$/i, "")
    .replace(/\s*on Instagram:.*$/i, "")
    .replace(/\s*\|\s*TikTok$/i, "")
    .replace(/https?:\/\/\S+/gi, "")
    .trim();

  clean = clean.replace(/[\r\n\t]+/g, " ");
  clean = clean.replace(/[^\w\s\-\.\(\)\[\]]/g, " ");
  clean = clean.replace(/[\s_]+/g, "_").trim();
  clean = clean.replace(/^[\._\-]+|[\._\-]+$/g, "");

  if (clean.length > maxLen) {
    clean = clean.substring(0, maxLen).replace(/[\._\-]+$/, "");
  }

  return clean || defaultName;
}

function isGenericTitle(title) {
  if (!title || typeof title !== "string") return true;
  const base = title.replace(/\.[a-zA-Z0-9]+$/, "").trim().toLowerCase();
  const genericList = [
    "embed",
    "video",
    "player",
    "video player",
    "iframe",
    "untitled",
    "stream",
    "media player",
    "watch",
    "download",
    "play",
    "index",
    "master"
  ];
  return genericList.includes(base);
}

function resolveSmartFilename(requestedFilename, tabTitle, quality = "") {
  if (!tabTitle || isGenericTitle(tabTitle)) {
    return requestedFilename || "video.mp4";
  }

  const safeTabTitle = cleanFilename(tabTitle, "video", 80);

  if (!requestedFilename) {
    return quality ? `${safeTabTitle}_${quality}.mp4` : `${safeTabTitle}.mp4`;
  }

  const match = requestedFilename.match(/^(.*?)(_(?:4k|2160p|1080p|720p|480p|360p|audio|sub_[a-z0-9]+))?(\.[a-zA-Z0-9]+)?$/i);
  if (match) {
    const basePrefix = match[1] || "";
    const qualityTag = match[2] || (quality ? `_${quality}` : "");
    const ext = match[3] || ".mp4";

    if (isGenericTitle(basePrefix)) {
      return `${safeTabTitle}${qualityTag}${ext}`;
    }
  }

  return requestedFilename;
}

// 4. Send to Desktop App via Local HTTP Server (Port 18888) or Native Messaging Fallback
async function sendToMyOwnIdm(downloadUrl, refererUrl = "", filename = "", extraMeta = {}) {
  try {
    // Only get cookies for valid http/https URLs
    let cookieHeader = "";
    if (downloadUrl && (downloadUrl.startsWith("http://") || downloadUrl.startsWith("https://"))) {
      try {
        const cookies = await chrome.cookies.getAll({ url: downloadUrl });
        cookieHeader = cookies.map(c => `${c.name}=${c.value}`).join("; ");
      } catch (e) {
        console.warn("Could not read cookies:", e);
      }
    }

    const payload = {
      action: "download",
      url: downloadUrl,
      filename: filename || "",
      quality: extraMeta.quality || "",
      is_audio_only: extraMeta.is_audio_only || false,
      threads: extraMeta.threads || 16,
      headers: {
        "Cookie": cookieHeader,
        "Referer": refererUrl || "",
        "User-Agent": navigator.userAgent
      }
    };

    // Try Local HTTP Server first (Port 18888)
    try {
      const res = await fetch("http://127.0.0.1:18888/download", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload)
      });
      if (res.ok) {
        console.log("Sent to My Own IDM via HTTP successfully!");
        return true;
      }
    } catch (httpErr) {
      console.warn("Local HTTP IPC not reached, trying Native Messaging fallback...", httpErr);
    }

    // Fallback: Native Messaging
    return new Promise((resolve) => {
      chrome.runtime.sendNativeMessage(NATIVE_HOST, payload, (response) => {
        if (chrome.runtime.lastError) {
          console.error("Native messaging error:", chrome.runtime.lastError.message);
          resolve(false);
        } else {
          console.log("Desktop app response via Native Messaging:", response);
          resolve(true);
        }
      });
    });
  } catch (err) {
    console.error("Failed to forward download:", err);
    return false;
  }
}

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === "get-tab-info") {
    sendResponse({
      tabId: sender.tab?.id,
      tabTitle: sender.tab?.title || "",
      tabUrl: sender.tab?.url || ""
    });
    return true;
  } else if (request.action === "get-detected-media") {
    const tabId = request.tabId || sender.tab?.id;
    const media = detectedMediaByTab[tabId] || [];
    sendResponse({ media, isInterceptorEnabled });
  } else if (request.action === "toggle-interceptor") {
    isInterceptorEnabled = request.enabled;
    sendResponse({ isInterceptorEnabled });
  } else if (request.action === "send-download") {
    let filename = request.filename || "";
    const tabTitle = sender.tab?.title;
    if (tabTitle) {
      filename = resolveSmartFilename(filename, tabTitle, request.quality);
    }
    const extraMeta = {
      quality: request.quality,
      is_audio_only: request.is_audio_only,
      threads: request.threads,
      batch: request.batch
    };
    sendToMyOwnIdm(request.url, request.referer, filename, extraMeta)
      .then((success) => {
        sendResponse({ status: success ? "ok" : "error" });
      })
      .catch((err) => {
        sendResponse({ status: "error", error: String(err) });
      });
    return true; // Keep channel open for async response
  } else if (request.action === "report-error") {
    reportExtensionError(request.error_type || "extension_error", request.message, request.details)
      .then((ok) => {
        sendResponse({ status: ok ? "ok" : "fallback" });
      });
    return true;
  }
  return true;
});

const GLITCHTIP_PROJECT_ID = "28025";
const GLITCHTIP_KEY = "2e56fa98dc824baaa843dff62a18b3d2";
const GLITCHTIP_STORE_URL = `https://app.glitchtip.com/api/${GLITCHTIP_PROJECT_ID}/store/`;

// 5. Crash & Diagnostic Error Reporting
async function reportExtensionError(errorType, message, details = {}) {
  const payload = {
    error_type: errorType,
    message: String(message),
    browser: navigator.userAgent,
    details: details || {}
  };

  // Primary: Forward via Desktop App IPC HTTP Server
  try {
    const res = await fetch("http://127.0.0.1:18888/error-report", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    });
    if (res.ok) {
      console.log("[MyOwnIDM] Extension diagnostic reported via Desktop IPC:", errorType);
      return true;
    }
  } catch (ipcErr) {
    // Desktop app may be closed or offline
  }

  // Fallback 1: Try Native Messaging if available
  try {
    chrome.runtime.sendNativeMessage(
      NATIVE_HOST,
      { type: "error_report", ...payload },
      () => {
        if (chrome.runtime.lastError) {
          // Native host offline
        }
      }
    );
  } catch (e) {
    // ignore
  }

  // Fallback 2: Direct HTTP POST to GlitchTip when desktop is closed
  try {
    const eventId = (typeof crypto !== "undefined" && crypto.randomUUID
      ? crypto.randomUUID()
      : Math.random().toString(36).slice(2)).replace(/-/g, "");

    const sentryEvent = {
      event_id: eventId,
      timestamp: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
      platform: "javascript",
      level: "error",
      logger: "browser_extension",
      message: `Extension [${errorType}]: ${message}`,
      tags: {
        component: "browser_extension",
        error_type: errorType,
        browser: navigator.userAgent
      },
      extra: details || {}
    };

    const directRes = await fetch(GLITCHTIP_STORE_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Sentry-Auth": `Sentry sentry_version=7, sentry_client=myownidm-extension/1.0, sentry_key=${GLITCHTIP_KEY}`
      },
      body: JSON.stringify(sentryEvent)
    });
    if (directRes.ok) {
      console.log("[MyOwnIDM] Extension diagnostic reported directly to GlitchTip:", errorType);
      return true;
    }
  } catch (directErr) {
    // network error
  }

  return false;
}


// Global Exception Handlers for Extension Background Service Worker
self.addEventListener("error", (event) => {
  reportExtensionError("uncaught_exception", event.message, {
    filename: event.filename,
    lineno: event.lineno,
    colno: event.colno
  });
});

self.addEventListener("unhandledrejection", (event) => {
  reportExtensionError("unhandled_rejection", String(event.reason));
});

