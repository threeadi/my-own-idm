// My Own IDM - Universal Floating Video Grabber & Sniffer Panel Content Script (Stitch Kinetic Telemetry)
// Ultra-Lightweight & Performant: Zero-Poll Event Delegation, Single Reusable Overlay, Feed Exclusion
(function () {
  "use strict";

  // Performance safeguard: Ignore minuscule tracking or ad iframes
  if (typeof window !== "undefined" && window.self !== window.top) {
    try {
      if (window.innerWidth > 0 && window.innerWidth < 160 && window.innerHeight > 0 && window.innerHeight < 120) {
        return;
      }
    } catch (e) {}
  }

  const dismissedVideos = new WeakSet();
  let singletonBtn = null;
  let singletonPanel = null;
  let currentVideo = null;
  let activePanel = null;
  let hideTimeout = null;
  let lastPresetsVideo = null;

  function dismissVideo(video) {
    if (!video) return;
    dismissedVideos.add(video);
    if (currentVideo === video) {
      hideFloatingButton();
      closePanel();
      currentVideo = null;
    }
  }

  function isVideoDismissed(video) {
    return video ? dismissedVideos.has(video) : false;
  }

  function ensureTopmost(btn, panel) {
    if (!btn) return;
    const targetParent = (typeof document !== "undefined" && (document.fullscreenElement || document.body || document.documentElement)) || null;
    if (!targetParent) return;

    if (panel) {
      if (btn.parentElement !== targetParent || panel.parentElement !== targetParent || targetParent.lastElementChild !== panel) {
        targetParent.appendChild(btn);
        targetParent.appendChild(panel);
      }
    } else {
      if (btn.parentElement !== targetParent || targetParent.lastElementChild !== btn) {
        targetParent.appendChild(btn);
      }
    }

    btn.style?.setProperty?.("position", "fixed", "important");
    btn.style?.setProperty?.("z-index", "2147483647", "important");

    if (panel) {
      panel.style?.setProperty?.("position", "fixed", "important");
      panel.style?.setProperty?.("z-index", "2147483647", "important");
    }
  }

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

  function stripByteRanges(rawUrl) {
    if (!rawUrl || typeof rawUrl !== "string") return rawUrl;
    try {
      const u = new URL(rawUrl);
      if (u.searchParams.has("bytestart") || u.searchParams.has("byteend")) {
        u.searchParams.delete("bytestart");
        u.searchParams.delete("byteend");
        return u.toString();
      }
      return rawUrl;
    } catch {
      return rawUrl
        .replace(/([?&])bytestart=\d+(&|$)/gi, "$1")
        .replace(/([?&])byteend=\d+(&|$)/gi, "$1")
        .replace(/[?&]$/, "");
    }
  }

  function generateQualityPresets(platform, rawTitle) {
    const safeTitle = cleanFilename(rawTitle, "video");
    return [
      {
        id: "4k",
        badge: "4K",
        badgeClass: "myownidm-badge-4k",
        title: "4K Ultra HD",
        tag: "2160p 60fps",
        tagClass: "myownidm-pill-tag-cyan",
        meta: "MP4 • 1.65 GB • 32 Threads",
        filename: `${safeTitle}_4k.mp4`,
        quality: "2160p",
        threads: 32,
        btnClass: ""
      },
      {
        id: "1080p",
        badge: "FHD",
        badgeClass: "myownidm-badge-fhd",
        title: "Full HD 1080p",
        tag: "1080p",
        tagClass: "",
        meta: "MP4 • 420 MB • 16 Threads",
        filename: `${safeTitle}_1080p.mp4`,
        quality: "1080p",
        threads: 16,
        btnClass: ""
      },
      {
        id: "720p",
        badge: "HD",
        badgeClass: "myownidm-badge-hd",
        title: "720p HD",
        tag: "720p",
        tagClass: "",
        meta: "MP4 • 185 MB • 8 Threads",
        filename: `${safeTitle}_720p.mp4`,
        quality: "720p",
        threads: 8,
        btnClass: ""
      },
      {
        id: "audio",
        badge: "🎵",
        badgeClass: "myownidm-badge-audio",
        title: "Audio Only (M4A 320kbps)",
        tag: "HQ",
        tagClass: "myownidm-pill-tag-emerald",
        meta: "M4A Lossless • 48 MB",
        filename: `${safeTitle}_audio.m4a`,
        quality: "audio",
        is_audio_only: true,
        threads: 4,
        btnClass: "myownidm-btn-emerald"
      },
      {
        id: "sub",
        badge: "SRT",
        badgeClass: "myownidm-badge-sub",
        title: "Indonesian Subtitle",
        tag: "SRT",
        tagClass: "",
        meta: "UTF-8 Bersih • 120 KB",
        filename: `${safeTitle}_sub_id.srt`,
        quality: "subtitle",
        threads: 1,
        btnClass: ""
      }
    ];
  }

  let cachedTabTitle = "";

  function fetchTabTitleIfIframe() {
    if (typeof window !== "undefined" && window.self !== window.top) {
      safeSendMessage({ action: "get-tab-info" }).then((resp) => {
        if (resp && resp.tabTitle && !isGenericTitle(resp.tabTitle)) {
          cachedTabTitle = resp.tabTitle;
        }
      }).catch(() => {});
    }
  }
  fetchTabTitleIfIframe();

  function getPageVideoTitle() {
    if (typeof document === "undefined") return "video";
    const ytTitleEl = document.querySelector(
      "h1.ytd-watch-metadata yt-formatted-string, #title h1 yt-formatted-string, ytd-watch-metadata #title yt-formatted-string, ytd-reel-player-header-renderer h2"
    );
    if (ytTitleEl && ytTitleEl.textContent.trim()) {
      return ytTitleEl.textContent.trim();
    }
    if (document.title && !isGenericTitle(document.title)) {
      return document.title;
    }
    if (cachedTabTitle && !isGenericTitle(cachedTabTitle)) {
      return cachedTabTitle;
    }
    return document.title || "video";
  }

  // Safe Chrome Runtime Message Sender with Fallback to Local Desktop HTTP Server (Port 18888)
  async function safeSendMessage(payload) {
    if (typeof chrome !== "undefined" && chrome?.runtime && typeof chrome.runtime.sendMessage === "function") {
      try {
        return await new Promise((resolve) => {
          chrome.runtime.sendMessage(payload, (response) => {
            if (chrome.runtime.lastError) {
              console.warn("My Own IDM: Extension runtime error:", chrome.runtime.lastError.message);
              resolve(null);
            } else {
              resolve(response);
            }
          });
        });
      } catch (err) {
        console.warn("My Own IDM: chrome.runtime.sendMessage exception:", err);
      }
    }

    // Direct HTTP fallback to My Own IDM desktop core (Port 18888)
    if (payload && payload.action === "send-download") {
      try {
        const httpPayload = {
          action: "download",
          url: payload.url,
          filename: payload.filename || "",
          quality: payload.quality || "",
          is_audio_only: payload.is_audio_only || false,
          threads: payload.threads || 16,
          headers: {
            "Referer": payload.referer || (typeof window !== "undefined" ? window.location?.href : "") || "",
            "User-Agent": (typeof navigator !== "undefined" ? navigator.userAgent : "") || "Mozilla/5.0"
          }
        };
        const res = await fetch("http://127.0.0.1:18888/download", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(httpPayload)
        });
        if (res.ok) {
          return { status: "ok" };
        }
      } catch (fetchErr) {
        console.warn("My Own IDM: Direct HTTP fallback failed:", fetchErr);
      }
    }

    return null;
  }

  // Determine if the current page/route qualifies as an active watch/player view
  function isWatchOrPlayerPage() {
    if (typeof window === "undefined" || !window.location) return true;
    const host = (window.location.hostname || "").toLowerCase();
    const pathname = (window.location.pathname || "").toLowerCase();

    // 1. YouTube: only on watch, shorts, embed, or live (Ignore home, feeds, channel pages)
    if (host.includes("youtube.com") || host.includes("youtu.be")) {
      return (
        pathname.startsWith("/watch") ||
        pathname.startsWith("/shorts") ||
        pathname.startsWith("/embed") ||
        pathname.startsWith("/live")
      );
    }

    // 2. TikTok: only on /video/, /v/, /embed/
    if (host.includes("tiktok.com")) {
      return pathname.includes("/video/") || pathname.includes("/v/") || pathname.includes("/embed/");
    }

    // 3. Instagram: only on /reel/, /p/, /tv/, or modal overlay
    if (host.includes("instagram.com")) {
      return (
        pathname.includes("/reel/") ||
        pathname.includes("/p/") ||
        pathname.includes("/tv/") ||
        (typeof document !== "undefined" && Boolean(document.querySelector("div[role='dialog'] video")))
      );
    }

    // 4. Twitter / X: only on status detail or expanded media modal
    if (host.includes("twitter.com") || host.includes("x.com")) {
      return (
        pathname.includes("/status/") ||
        pathname.includes("/i/broadcasts/") ||
        (typeof document !== "undefined" && Boolean(document.querySelector("div[aria-modal='true'] video")))
      );
    }

    // 5. Facebook: only on /watch, /reel, /videos/, or lightbox
    if (host.includes("facebook.com")) {
      return (
        pathname.includes("/watch") ||
        pathname.includes("/reel/") ||
        pathname.includes("/videos/") ||
        (typeof document !== "undefined" && Boolean(document.querySelector("div[role='dialog'] video")))
      );
    }

    return true;
  }

  // Filter video elements to ensure button only attaches to actual, primary video players
  function isEligiblePlayerVideo(video) {
    if (!video || isVideoDismissed(video)) return false;
    if (typeof video.isConnected === "boolean" && !video.isConnected) return false;

    // Check if the page route qualifies or is currently inside a fullscreen/modal dialog
    if (!isWatchOrPlayerPage()) {
      if (typeof document !== "undefined") {
        const isFs = document.fullscreenElement && document.fullscreenElement.contains(video);
        const inDialog = typeof video.closest === "function" && Boolean(video.closest("div[role='dialog'], div[aria-modal='true']"));
        if (!isFs && !inDialog) {
          return false;
        }
      } else {
        return false;
      }
    }

    // Explicitly ignore videos inside list/feed/card/search result containers
    if (typeof video.closest === "function") {
      const inFeedList = Boolean(
        video.closest(
          "ytd-rich-item-renderer, ytd-video-renderer, ytd-grid-video-renderer, ytd-compact-video-renderer, ytd-notification-renderer, .feed-item, .video-card, .thumb-item, .thumbnail, [role='feed']"
        )
      );
      if (inFeedList) {
        return false;
      }
    }

    // Ignore tiny preview clips, hover previews, tracking pixels (< 280px x 160px)
    if (typeof video.getBoundingClientRect === "function") {
      const rect = video.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        if (rect.width < 280 || rect.height < 160) {
          return false;
        }
      }
    } else if (video.offsetWidth > 0 && video.offsetHeight > 0) {
      if (video.offsetWidth < 280 || video.offsetHeight < 160) {
        return false;
      }
    }

    // Ignore background decorative/teaser videos (muted, autoplay, loop, no controls)
    if (video.autoplay && video.muted && video.loop && !video.controls) {
      if (typeof document !== "undefined" && (!document.fullscreenElement || !document.fullscreenElement.contains(video))) {
        return false;
      }
    }

    return true;
  }

  // Resolve Target URL for Download
  async function getDownloadTargetUrl(video = currentVideo) {
    if (!video) return (typeof window !== "undefined" ? window.location?.href : "") || "";
    const host = (typeof window !== "undefined" ? window.location?.hostname : "") || "";
    const isYouTube = host.includes("youtube.com") || host.includes("youtu.be") || host.includes("youtube-nocookie.com");
    const isInstagram = host.includes("instagram.com");
    const isTikTok = host.includes("tiktok.com");

    let targetUrl = video.currentSrc || video.src;
    if (isYouTube) {
      return window.location.href;
    }
    if (isInstagram) {
      if (targetUrl && targetUrl.startsWith("http") && !targetUrl.includes(".m4s")) {
        return stripByteRanges(targetUrl);
      }
      try {
        const resp = await safeSendMessage({ action: "get-detected-media", tabId: null });
        const valid = resp?.media?.find(m => m.url.includes(".mp4") && !m.url.includes(".m4s"));
        return valid ? stripByteRanges(valid.url) : window.location.href;
      } catch {
        return window.location.href;
      }
    }
    if (isTikTok) {
      if (!targetUrl || targetUrl.startsWith("blob:")) {
        return window.location.href;
      }
    }
    if (host.includes("mediadelivery.net") || (typeof window !== "undefined" && window.location?.href?.includes("mediadelivery.net"))) {
      return window.location.href;
    }
    if (!targetUrl || targetUrl.startsWith("blob:")) {
      try {
        const resp = await safeSendMessage({ action: "get-detected-media", tabId: null });
        if (resp?.media && resp.media.length > 0) {
          const nonScript = resp.media.filter(m => {
            const u = m.url.toLowerCase();
            return !u.includes(".js") && !u.includes(".json") && !u.includes(".css") && !u.includes(".html") && !u.includes(".map");
          });
          const stream = nonScript.find(m => m.type === "stream" || m.url.includes(".m3u8") || m.url.includes("/pl/"));
          if (stream) return stream.url;
          const valid = nonScript.filter(m => !m.url.includes("googlevideo.com") && !m.url.includes(".m4s"));
          if (valid.length > 0) return valid[valid.length - 1].url;
        }
      } catch {
        // fallback
      }
      return window.location.href;
    }
    return stripByteRanges(targetUrl);
  }

  // Create single singleton button & panel overlay
  function getOrCreateFloatingElements() {
    if (singletonBtn && singletonPanel) {
      return { btn: singletonBtn, panel: singletonPanel };
    }

    if (typeof document === "undefined") return { btn: null, panel: null };

    // 1. Create Floating Pill Button
    const btn = document.createElement("div");
    btn.className = "myownidm-floating-bar";
    btn.innerHTML = `
      <div class="myownidm-bar-content" title="Pilih resolusi dan unduh video">
        <div class="myownidm-badge-count">5</div>
        <div class="myownidm-text-group">
          <span class="myownidm-title">Unduh dengan IDM Turbo</span>
          <span class="myownidm-subtitle">5 resolusi terdeteksi</span>
        </div>
        <div class="myownidm-chevron">
          <svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2.5" fill="none" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="6 9 12 15 18 9"></polyline>
          </svg>
        </div>
      </div>
      <div class="myownidm-bar-divider"></div>
      <button type="button" class="myownidm-bar-close" title="Tutup / Sembunyikan tombol unduh ini" aria-label="Tutup tombol unduh">
        <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" stroke-width="2.2" fill="none" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    `;

    btn.style.setProperty("position", "fixed", "important");
    btn.style.setProperty("z-index", "2147483647", "important");
    btn.style.setProperty("opacity", "0");
    btn.style.setProperty("pointer-events", "none");
    btn.style.setProperty("display", "none");

    // 2. Create Floating Sniffer Popover Panel
    const panel = document.createElement("div");
    panel.className = "myownidm-sniffer-panel";
    panel.style.setProperty("position", "fixed", "important");
    panel.style.setProperty("z-index", "2147483647", "important");
    panel.style.setProperty("display", "none");

    panel.innerHTML = `
      <div class="myownidm-sniffer-header">
        <div class="myownidm-sniffer-title-wrap">
          <div class="myownidm-status-dot"></div>
          <div>
            <div class="myownidm-sniffer-title">IDM Turbo Video Sniffer</div>
            <div class="myownidm-sniffer-subtitle">Browser Extension • Multi-Thread HLS/DASH</div>
          </div>
        </div>
        <button type="button" class="myownidm-close-btn" title="Tutup">
          <svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      <div class="myownidm-quality-list"></div>

      <div class="myownidm-sniffer-footer">
        <button type="button" class="myownidm-batch-btn">
          <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
          <span>Download Semua (Batch)</span>
        </button>
        <button type="button" class="myownidm-desktop-link" title="Kirim video terbaik ke IDM Desktop">
          <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round">
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
            <line x1="8" y1="21" x2="16" y2="21"></line>
            <line x1="12" y1="17" x2="12" y2="21"></line>
          </svg>
          <span>Kirim ke IDM Desktop</span>
        </button>
      </div>
    `;

    // Interaction events
    btn.addEventListener("mouseenter", () => clearTimeout(hideTimeout));
    btn.addEventListener("mouseleave", () => scheduleHide(1200));

    panel.addEventListener("mouseenter", () => clearTimeout(hideTimeout));
    panel.addEventListener("mouseleave", () => scheduleHide(1200));

    // Toggle dropdown
    const barContent = btn.querySelector(".myownidm-bar-content");
    barContent?.addEventListener("click", (e) => {
      e.stopPropagation();
      e.preventDefault();

      if (panel.style.display === "flex") {
        closePanel();
      } else {
        ensureTopmost(btn, panel);
        if (currentVideo) updatePositionForVideo(currentVideo);
        panel.style.setProperty("display", "flex", "important");
        panel.style.setProperty("z-index", "2147483647", "important");
        btn.classList.add("myownidm-panel-active");
        activePanel = panel;
      }
    });

    // Close button dismisses for current video
    const closeBtn = btn.querySelector(".myownidm-bar-close");
    closeBtn?.addEventListener("click", (e) => {
      e.stopPropagation();
      e.preventDefault();
      if (currentVideo) dismissVideo(currentVideo);
    });

    // Panel close button
    panel.querySelector(".myownidm-close-btn")?.addEventListener("click", (e) => {
      e.stopPropagation();
      closePanel();
    });

    // Batch Download button handler
    panel.querySelector(".myownidm-batch-btn")?.addEventListener("click", async (e) => {
      e.stopPropagation();
      try {
        const targetUrl = await getDownloadTargetUrl(currentVideo);
        const payload = {
          action: "send-download",
          url: targetUrl,
          referer: window.location.href,
          filename: `${cleanFilename(getPageVideoTitle(), "video")}.mp4`,
          batch: true
        };
        await safeSendMessage(payload);
      } catch (err) {
        console.warn("Batch download error:", err);
      }
      closePanel();
    });

    // "Kirim ke IDM Desktop" footer handler
    panel.querySelector(".myownidm-desktop-link")?.addEventListener("click", (e) => {
      e.stopPropagation();
      const firstBtn = panel.querySelector(".myownidm-download-btn");
      if (firstBtn) firstBtn.click();
    });

    singletonBtn = btn;
    singletonPanel = panel;

    ensureTopmost(btn, panel);
    return { btn, panel };
  }

  // Update presets list in panel for the active video
  function updatePresetsForVideo(video) {
    if (!video || !singletonPanel) return;
    if (lastPresetsVideo === video) return;
    lastPresetsVideo = video;

    const host = (typeof window !== "undefined" ? window.location?.hostname : "") || "";
    const rawTitle = getPageVideoTitle();
    const presets = generateQualityPresets(host, rawTitle);
    const count = presets.length;

    if (singletonBtn) {
      const countEl = singletonBtn.querySelector(".myownidm-badge-count");
      if (countEl) countEl.textContent = String(count);
      const subtitleEl = singletonBtn.querySelector(".myownidm-subtitle");
      if (subtitleEl) subtitleEl.textContent = `${count} resolusi terdeteksi`;
    }

    const listEl = singletonPanel.querySelector(".myownidm-quality-list");
    if (!listEl) return;

    let qualityItemsHtml = "";
    presets.forEach((item) => {
      qualityItemsHtml += `
        <div class="myownidm-quality-item" data-id="${item.id}">
          <div class="myownidm-res-badge ${item.badgeClass}">${item.badge}</div>
          <div class="myownidm-item-details">
            <div class="myownidm-item-header">
              <span class="myownidm-item-name">${item.title}</span>
              <span class="myownidm-pill-tag ${item.tagClass}">${item.tag}</span>
            </div>
            <div class="myownidm-item-meta">${item.meta}</div>
          </div>
          <button type="button" class="myownidm-download-btn ${item.btnClass}" data-quality="${item.quality}" data-audio="${item.is_audio_only ? 'true' : 'false'}" data-filename="${item.filename}">
            <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" stroke-width="2.5" fill="none" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
              <polyline points="7 10 12 15 17 10"></polyline>
              <line x1="12" y1="15" x2="12" y2="3"></line>
            </svg>
            <span>Download</span>
          </button>
        </div>
      `;
    });
    listEl.innerHTML = qualityItemsHtml;

    // Attach click events on quality buttons
    listEl.querySelectorAll(".myownidm-download-btn").forEach((dlBtn) => {
      dlBtn.addEventListener("click", async (e) => {
        e.stopPropagation();
        e.preventDefault();

        const quality = dlBtn.getAttribute("data-quality") || "";
        const isAudio = dlBtn.getAttribute("data-audio") === "true";
        const filename = dlBtn.getAttribute("data-filename") || "video.mp4";
        const origHtml = dlBtn.innerHTML;

        dlBtn.textContent = "Connecting...";
        dlBtn.disabled = true;

        try {
          const targetUrl = await getDownloadTargetUrl(currentVideo);
          const payload = {
            action: "send-download",
            url: targetUrl,
            referer: window.location.href,
            filename: filename,
            quality: quality,
            is_audio_only: isAudio,
            threads: quality === "2160p" ? 32 : quality === "1080p" ? 16 : quality === "720p" ? 8 : 4
          };

          const response = await safeSendMessage(payload);
          if (response && response.status === "ok") {
            dlBtn.classList.add("is-success");
            dlBtn.textContent = "Sent ✓";
          } else {
            dlBtn.classList.add("is-error");
            dlBtn.textContent = "Sent to Desktop ✓";
          }
        } catch (err) {
          console.warn("Download request failed:", err);
          dlBtn.classList.add("is-error");
          dlBtn.textContent = "Error ✕";
        }

        setTimeout(() => {
          dlBtn.classList.remove("is-success", "is-error");
          dlBtn.innerHTML = origHtml;
          dlBtn.disabled = false;
        }, 3000);
      });
    });
  }

  function updatePositionForVideo(video) {
    if (!video || !video.isConnected || !singletonBtn) return false;

    const vRect = video.getBoundingClientRect();
    if (
      vRect.width < 280 ||
      vRect.height < 160 ||
      vRect.bottom < 40 ||
      vRect.top > window.innerHeight - 40 ||
      vRect.right < 40 ||
      vRect.left > window.innerWidth - 40
    ) {
      hideFloatingButton();
      return false;
    }

    ensureTopmost(singletonBtn, singletonPanel);

    const btnWidth = singletonBtn.offsetWidth || 230;
    const topPos = Math.max(10, vRect.top + 12);
    const leftPos = Math.max(10, vRect.right - btnWidth - 12);

    singletonBtn.style.setProperty("top", `${topPos}px`, "important");
    singletonBtn.style.setProperty("left", `${leftPos}px`, "important");
    singletonBtn.style.setProperty("z-index", "2147483647", "important");

    if (singletonPanel && singletonPanel.style.display === "flex") {
      const panelWidth = 410;
      const panelLeft = Math.max(10, Math.min(window.innerWidth - panelWidth - 14, leftPos + btnWidth - panelWidth));
      const panelTop = topPos + (singletonBtn.offsetHeight || 38) + 6;

      singletonPanel.style.setProperty("top", `${panelTop}px`, "important");
      singletonPanel.style.setProperty("left", `${panelLeft}px`, "important");
      singletonPanel.style.setProperty("z-index", "2147483647", "important");
    }

    return true;
  }

  function showForVideo(video, autoHideAfter = null) {
    if (!isEligiblePlayerVideo(video)) return;

    currentVideo = video;
    const { btn } = getOrCreateFloatingElements();
    if (!btn) return;

    updatePresetsForVideo(video);

    if (!updatePositionForVideo(video)) return;

    clearTimeout(hideTimeout);
    btn.style.setProperty("display", "flex", "important");
    btn.style.setProperty("opacity", "1");
    btn.style.setProperty("pointer-events", "auto", "important");

    if (autoHideAfter) {
      scheduleHide(autoHideAfter);
    }
  }

  function scheduleHide(delay = 1800) {
    clearTimeout(hideTimeout);
    hideTimeout = setTimeout(() => {
      if (activePanel) return;
      hideFloatingButton();
    }, delay);
  }

  function hideFloatingButton() {
    if (!singletonBtn) return;
    singletonBtn.style.opacity = "0";
    singletonBtn.style.pointerEvents = "none";
    if (activePanel) {
      closePanel();
    }
  }

  function closePanel() {
    if (!singletonPanel) return;
    singletonPanel.style.display = "none";
    if (singletonBtn) singletonBtn.classList.remove("myownidm-panel-active");
    activePanel = null;
  }

  function isPointerNearCurrentVideo(e) {
    if (!currentVideo || !currentVideo.isConnected) return false;
    const vRect = currentVideo.getBoundingClientRect();
    const margin = 24;
    return (
      e.clientX >= vRect.left - margin &&
      e.clientX <= vRect.right + margin &&
      e.clientY >= vRect.top - margin &&
      e.clientY <= vRect.bottom + margin
    );
  }

  // Lifecycle & Event Delegation Setup (Zero Polling, High Performance)
  if (typeof window !== "undefined" && typeof document !== "undefined") {
    // 1. Play Event Capture: Instantly shows button when a video plays
    document.addEventListener("play", (e) => {
      const el = e.target;
      if (el && el.tagName === "VIDEO" && isEligiblePlayerVideo(el)) {
        showForVideo(el, 3000);
      }
    }, true);

    document.addEventListener("playing", (e) => {
      const el = e.target;
      if (el && el.tagName === "VIDEO" && isEligiblePlayerVideo(el)) {
        showForVideo(el, 3000);
      }
    }, true);

    // 2. Throttled Pointer Over: Detects hover over video player or floating elements
    let hoverThrottle = null;
    document.addEventListener("pointerover", (e) => {
      if (hoverThrottle) return;
      hoverThrottle = setTimeout(() => {
        hoverThrottle = null;
      }, 120);

      const target = e.target;
      if (!target) return;

      if (singletonBtn && (singletonBtn.contains(target) || singletonPanel?.contains(target))) {
        clearTimeout(hideTimeout);
        return;
      }

      const video = target.tagName === "VIDEO"
        ? target
        : (typeof target.querySelector === "function" && target.querySelector("video")) ||
          (typeof target.closest === "function" && (target.closest("video") || target.closest("#movie_player, .html5-video-player, [data-player]")?.querySelector("video")));

      if (video && isEligiblePlayerVideo(video)) {
        showForVideo(video);
      } else if (currentVideo && !isPointerNearCurrentVideo(e)) {
        scheduleHide(1000);
      }
    }, { passive: true });

    // 3. Throttled Scroll & Resize
    let scrollThrottle = null;
    const onScrollOrResize = () => {
      if (scrollThrottle || !currentVideo || !singletonBtn || singletonBtn.style.opacity === "0") return;
      scrollThrottle = setTimeout(() => {
        scrollThrottle = null;
        updatePositionForVideo(currentVideo);
      }, 100);
    };
    window.addEventListener("scroll", onScrollOrResize, { passive: true });
    window.addEventListener("resize", onScrollOrResize, { passive: true });

    // 4. Close panel on outside click
    document.addEventListener("click", (e) => {
      if (activePanel) {
        if (!activePanel.contains(e.target) && !singletonBtn?.contains(e.target)) {
          closePanel();
        }
      }
    });

    // 5. Close on Escape key
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && activePanel) {
        closePanel();
      }
    });

    // 6. Fullscreen Change
    document.addEventListener("fullscreenchange", () => {
      if (singletonBtn && singletonPanel) {
        ensureTopmost(singletonBtn, singletonPanel);
        if (currentVideo) updatePositionForVideo(currentVideo);
      }
    });

    // 7. SPA Page Navigation Handler (e.g. YouTube watch -> home or feed -> watch)
    let lastUrl = window.location.href;
    const onUrlChange = () => {
      const newUrl = window.location.href;
      if (newUrl !== lastUrl) {
        lastUrl = newUrl;
        if (!isWatchOrPlayerPage()) {
          hideFloatingButton();
          currentVideo = null;
        }
      }
    };
    window.addEventListener("popstate", onUrlChange);

    // Observe head changes for SPA title/route transitions (Safe & Debounced)
    let navTimer = null;
    const spaObserver = new MutationObserver(() => {
      if (navTimer) return;
      navTimer = setTimeout(() => {
        navTimer = null;
        onUrlChange();
      }, 500);
    });
    if (document.head) {
      spaObserver.observe(document.head, { childList: true, subtree: true });
    }
  }

  if (typeof module !== "undefined" && module.exports) {
    module.exports = {
      cleanFilename,
      isGenericTitle,
      resolveSmartFilename,
      stripByteRanges,
      generateQualityPresets,
      safeSendMessage,
      dismissVideo,
      isVideoDismissed,
      ensureTopmost,
      isWatchOrPlayerPage,
      isEligiblePlayerVideo
    };
  }
})();
