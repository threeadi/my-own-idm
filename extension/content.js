// My Own IDM - Universal Floating Video Grabber & Sniffer Panel Content Script (Stitch Kinetic Telemetry)
(function () {
  "use strict";

  const attachedButtons = new Map(); // video element -> { btn, panel }
  const dismissedVideos = new WeakSet();
  let activePanel = null;

  function dismissVideo(video) {
    if (!video) return;
    dismissedVideos.add(video);
    if (attachedButtons.has(video)) {
      const { btn, panel } = attachedButtons.get(video);
      if (panel) {
        panel.style.display = "none";
        panel.remove();
      }
      if (btn) {
        btn.style.opacity = "0";
        btn.style.pointerEvents = "none";
        btn.style.display = "none";
        btn.remove();
      }
      attachedButtons.delete(video);
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

    btn.style.setProperty("position", "fixed", "important");
    btn.style.setProperty("z-index", "2147483647", "important");

    if (panel) {
      panel.style.setProperty("position", "fixed", "important");
      panel.style.setProperty("z-index", "2147483647", "important");
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
    // Works even if extension context was invalidated upon reload or in third-party contexts
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

  function attachFloatingButton(video) {
    if (dismissedVideos.has(video)) return;
    const host = window.location.hostname;
    const isYouTube = host.includes("youtube.com") || host.includes("youtu.be") || host.includes("youtube-nocookie.com");
    const isInstagram = host.includes("instagram.com");
    const isTikTok = host.includes("tiktok.com");

    const rawTitle = getPageVideoTitle();
    const presets = generateQualityPresets(host, rawTitle);
    const count = presets.length;

    // 1. Create Floating Pill Button
    const btn = document.createElement("div");
    btn.className = "myownidm-floating-bar";
    btn.innerHTML = `
      <div class="myownidm-bar-content" title="Pilih resolusi dan unduh video">
        <div class="myownidm-badge-count">${count}</div>
        <div class="myownidm-text-group">
          <span class="myownidm-title">Unduh dengan IDM Turbo</span>
          <span class="myownidm-subtitle">${count} resolusi terdeteksi</span>
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

      <div class="myownidm-quality-list">
        ${qualityItemsHtml}
      </div>

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

    ensureTopmost(btn, panel);
    attachedButtons.set(video, { btn, panel });

    const closePanel = () => {
      panel.style.display = "none";
      btn.classList.remove("myownidm-panel-active");
      if (activePanel === panel) activePanel = null;
    };

    panel.querySelector(".myownidm-close-btn")?.addEventListener("click", (e) => {
      e.stopPropagation();
      closePanel();
    });

    // Resolve Target URL for Download
    async function getDownloadTargetUrl() {
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
      if (host.includes("mediadelivery.net") || (typeof window !== "undefined" && window.location.href.includes("mediadelivery.net"))) {
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

    // Attach Download Click Handler to each quality item button
    panel.querySelectorAll(".myownidm-download-btn").forEach((dlBtn) => {
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
          const targetUrl = await getDownloadTargetUrl();
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

    // Batch Download button handler
    panel.querySelector(".myownidm-batch-btn")?.addEventListener("click", async (e) => {
      e.stopPropagation();
      try {
        const targetUrl = await getDownloadTargetUrl();
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
      if (firstBtn) {
        firstBtn.click();
      }
    });

    // Position updates
    const updatePosition = () => {
      if (!video || !video.isConnected) {
        btn.remove();
        panel.remove();
        attachedButtons.delete(video);
        return;
      }

      const vRect = video.getBoundingClientRect();

      if (
        vRect.width < 120 ||
        vRect.height < 80 ||
        vRect.bottom < 40 ||
        vRect.top > window.innerHeight - 40 ||
        vRect.right < 40 ||
        vRect.left > window.innerWidth - 40
      ) {
        btn.style.display = "none";
        closePanel();
        return;
      }

      ensureTopmost(btn, panel);
      btn.style.setProperty("display", "flex", "important");

      const btnWidth = btn.offsetWidth || 230;
      const topPos = Math.max(10, vRect.top + 12);
      const leftPos = Math.max(10, vRect.right - btnWidth - 12);

      btn.style.setProperty("top", `${topPos}px`, "important");
      btn.style.setProperty("left", `${leftPos}px`, "important");
      btn.style.setProperty("z-index", "2147483647", "important");

      // Position dropdown panel directly below floating pill
      const panelWidth = 410;
      const panelLeft = Math.max(10, Math.min(window.innerWidth - panelWidth - 14, leftPos + btnWidth - panelWidth));
      const panelTop = topPos + (btn.offsetHeight || 38) + 6;

      panel.style.setProperty("top", `${panelTop}px`, "important");
      panel.style.setProperty("left", `${panelLeft}px`, "important");
      panel.style.setProperty("z-index", "2147483647", "important");
    };

    let hideTimeout;
    const showBtn = () => {
      if (dismissedVideos.has(video)) return;
      clearTimeout(hideTimeout);
      ensureTopmost(btn, panel);
      updatePosition();
      btn.style.setProperty("opacity", "1");
      btn.style.setProperty("pointer-events", "auto", "important");
    };

    const hideBtn = (delay = 1800) => {
      clearTimeout(hideTimeout);
      hideTimeout = setTimeout(() => {
        if (activePanel !== panel) {
          btn.style.opacity = "0";
          btn.style.pointerEvents = "none";
        }
      }, delay);
    };

    // Video events
    video.addEventListener("mouseenter", showBtn);
    video.addEventListener("mousemove", showBtn);
    video.addEventListener("mouseleave", () => hideBtn(1200));

    video.addEventListener("play", () => {
      showBtn();
      hideBtn(3000);
    });
    video.addEventListener("playing", () => {
      showBtn();
      hideBtn(3000);
    });

    // Ancestor container events
    const container =
      video.closest("article") ||
      video.closest("section") ||
      video.closest("div[role='dialog']") ||
      video.closest("#movie_player") ||
      video.closest(".html5-video-player") ||
      video.parentElement?.parentElement ||
      video.parentElement;

    if (container && container !== video && container !== document.body) {
      container.addEventListener("mouseenter", showBtn);
      container.addEventListener("mousemove", showBtn);
      container.addEventListener("mouseleave", () => hideBtn(1200));
    }

    window.addEventListener("scroll", updatePosition, { passive: true });
    window.addEventListener("resize", updatePosition, { passive: true });

    btn.addEventListener("mouseenter", () => clearTimeout(hideTimeout));
    btn.addEventListener("mouseleave", () => hideBtn(1200));

    panel.addEventListener("mouseenter", () => clearTimeout(hideTimeout));
    panel.addEventListener("mouseleave", () => hideBtn(1200));

    // Pill Click Handler: Click on content toggles Dropdown
    const barContent = btn.querySelector(".myownidm-bar-content");
    barContent?.addEventListener("click", (e) => {
      e.stopPropagation();
      e.preventDefault();

      if (panel.style.display === "flex") {
        closePanel();
      } else {
        if (activePanel && activePanel !== panel) {
          activePanel.style.display = "none";
        }
        ensureTopmost(btn, panel);
        updatePosition();
        panel.style.setProperty("display", "flex", "important");
        panel.style.setProperty("z-index", "2147483647", "important");
        btn.classList.add("myownidm-panel-active");
        activePanel = panel;
      }
    });

    // Close Button Click Handler: Dismisses floating bar for this video
    const closeBtn = btn.querySelector(".myownidm-bar-close");
    closeBtn?.addEventListener("click", (e) => {
      e.stopPropagation();
      e.preventDefault();
      dismissVideo(video);
    });
  }

  function scanAndAttach() {
    if (typeof window !== "undefined") {
      const host = window.location.hostname || "";
      if (
        host.includes("doubleclick.net") ||
        host.includes("googleads") ||
        host.includes("googlesyndication") ||
        host.includes("adnxs.com")
      ) {
        return;
      }
    }

    const videos = document.querySelectorAll("video");
    videos.forEach((video) => {
      if (dismissedVideos.has(video)) return;
      // Ignore tiny video elements (thumbnails, tracking pixels)
      if (video.offsetWidth > 0 && video.offsetWidth < 180 && video.offsetHeight > 0 && video.offsetHeight < 120) {
        return;
      }

      if (attachedButtons.has(video)) {
        const item = attachedButtons.get(video);
        ensureTopmost(item.btn, item.panel);
        return;
      }

      attachFloatingButton(video);
    });
  }

  if (typeof window !== "undefined" && typeof document !== "undefined") {
    // Close active panel on outside click
    document.addEventListener("click", (e) => {
      if (activePanel) {
        if (!activePanel.contains(e.target) && !e.target.closest(".myownidm-floating-bar")) {
          activePanel.style.display = "none";
          document.querySelectorAll(".myownidm-floating-bar").forEach(b => b.classList.remove("myownidm-panel-active"));
          activePanel = null;
        }
      }
    });

    // Close on Escape key
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && activePanel) {
        activePanel.style.display = "none";
        document.querySelectorAll(".myownidm-floating-bar").forEach(b => b.classList.remove("myownidm-panel-active"));
        activePanel = null;
      }
    });

    let lastGlobalCheck = 0;
    window.addEventListener("mousemove", (e) => {
      const now = Date.now();
      if (now - lastGlobalCheck < 150) return;
      lastGlobalCheck = now;

      for (const [video, item] of attachedButtons.entries()) {
        if (!video.isConnected || dismissedVideos.has(video)) continue;
        const vRect = video.getBoundingClientRect();
        if (
          e.clientX >= vRect.left &&
          e.clientX <= vRect.right &&
          e.clientY >= vRect.top &&
          e.clientY <= vRect.bottom
        ) {
          ensureTopmost(item.btn, item.panel);
          item.btn.style.setProperty("display", "flex", "important");
          item.btn.style.setProperty("opacity", "1");
          item.btn.style.setProperty("pointer-events", "auto", "important");
          const btnWidth = item.btn.offsetWidth || 230;
          item.btn.style.setProperty("top", `${Math.max(10, vRect.top + 12)}px`, "important");
          item.btn.style.setProperty("left", `${Math.max(10, vRect.right - btnWidth - 12)}px`, "important");
        }
      }
    }, { passive: true });

    const observer = new MutationObserver(() => {
      scanAndAttach();
      for (const [video, item] of attachedButtons.entries()) {
        if (item.btn && item.panel && item.btn.style.display !== "none") {
          ensureTopmost(item.btn, item.panel);
        }
      }
    });

    document.addEventListener("fullscreenchange", () => {
      for (const [video, item] of attachedButtons.entries()) {
        if (item.btn && item.panel) {
          ensureTopmost(item.btn, item.panel);
        }
      }
    });

    observer.observe(document.body || document.documentElement, {
      childList: true,
      subtree: true
    });

    setInterval(scanAndAttach, 1500);

    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", scanAndAttach);
    } else {
      scanAndAttach();
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
      ensureTopmost
    };
  }
})();
