const NATIVE_HOST = "com.myownidm.host";

// State
let isInterceptorEnabled = true;
let detectedMediaByTab = {};

// 1. Setup Context Menus on Install
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "download-with-myownidm",
    title: "Download with My Own IDM",
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

  // Forward to desktop client
  sendToMyOwnIdm(item.url, item.referrer, item.filename);
});

// 3. Media Sniffer (Video/Audio/m3u8 stream detection)
const STREAM_EXTENSIONS = [".m3u8", "/pl/", "playlist"];
const FILE_EXTENSIONS = [".mp4", ".mkv", ".webm", ".flv", ".mp3", ".aac", ".ogg"];

chrome.webRequest?.onResponseStarted?.addListener(
  (details) => {
    if (details.tabId < 0) return;
    const url = details.url.toLowerCase();

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

    const isStream = STREAM_EXTENSIONS.some(ext => url.includes(ext));
    const isFile = FILE_EXTENSIONS.some(ext => url.includes(ext));
    const contentTypeHeader = details.responseHeaders?.find(
      h => h.name.toLowerCase() === "content-type"
    );
    const ct = contentTypeHeader?.value?.toLowerCase() || "";
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
        chrome.action.setBadgeBackgroundColor({ tabId: details.tabId, color: "#06b6d4" });
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
  if (request.action === "get-detected-media") {
    const tabId = request.tabId || sender.tab?.id;
    const media = detectedMediaByTab[tabId] || [];
    sendResponse({ media, isInterceptorEnabled });
  } else if (request.action === "toggle-interceptor") {
    isInterceptorEnabled = request.enabled;
    sendResponse({ isInterceptorEnabled });
  } else if (request.action === "send-download") {
    const extraMeta = {
      quality: request.quality,
      is_audio_only: request.is_audio_only,
      threads: request.threads,
      batch: request.batch
    };
    sendToMyOwnIdm(request.url, request.referer, request.filename, extraMeta)
      .then((success) => {
        sendResponse({ status: success ? "ok" : "error" });
      })
      .catch((err) => {
        sendResponse({ status: "error", error: String(err) });
      });
    return true; // Keep channel open for async response
  }
  return true;
});
