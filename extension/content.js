// My Own IDM - Universal Floating Video Grabber Content Script
(function () {
  "use strict";

  const attachedButtons = new Map(); // video element -> floating button element

  function scanAndAttach() {
    const videos = document.querySelectorAll("video");
    videos.forEach((video) => {
      if (attachedButtons.has(video)) {
        const btn = attachedButtons.get(video);
        if (!document.body.contains(btn)) {
          document.body.appendChild(btn);
        }
        return;
      }

      attachFloatingButton(video);
    });
  }

  function cleanFilename(rawTitle, defaultName = "video") {
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

    if (clean.length > 50) {
      clean = clean.substring(0, 50).replace(/[\._\-]+$/, "");
    }

    return clean || defaultName;
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

  function attachFloatingButton(video) {
    const btn = document.createElement("div");
    btn.className = "myownidm-floating-bar";
    btn.innerHTML = `
      <div class="myownidm-icon">⚡</div>
      <span class="myownidm-text">Download this video</span>
      <span class="myownidm-badge">TURBO</span>
    `;

    // Fixed positioning anchored to the viewport for 100% overlay and overflow immunity
    btn.style.position = "fixed";
    btn.style.zIndex = "2147483647";
    btn.style.opacity = "0";
    btn.style.pointerEvents = "none";
    btn.style.display = "none";

    document.body.appendChild(btn);
    attachedButtons.set(video, btn);

    const updatePosition = () => {
      if (!video || !video.isConnected) {
        btn.remove();
        attachedButtons.delete(video);
        return;
      }

      const vRect = video.getBoundingClientRect();

      // Check if video is visible and has a meaningful size
      if (
        vRect.width < 120 ||
        vRect.height < 80 ||
        vRect.bottom < 40 ||
        vRect.top > window.innerHeight - 40 ||
        vRect.right < 40 ||
        vRect.left > window.innerWidth - 40
      ) {
        btn.style.display = "none";
        return;
      }

      btn.style.display = "flex";

      const btnWidth = btn.offsetWidth || 165;
      const topPos = Math.max(10, vRect.top + 12);
      const leftPos = Math.max(10, vRect.right - btnWidth - 12);

      btn.style.top = `${topPos}px`;
      btn.style.left = `${leftPos}px`;
    };

    let hideTimeout;
    const showBtn = () => {
      clearTimeout(hideTimeout);
      updatePosition();
      btn.style.opacity = "1";
      btn.style.pointerEvents = "auto";
    };

    const hideBtn = (delay = 1800) => {
      clearTimeout(hideTimeout);
      hideTimeout = setTimeout(() => {
        btn.style.opacity = "0";
        btn.style.pointerEvents = "none";
      }, delay);
    };

    // 1. Video events
    video.addEventListener("mouseenter", showBtn);
    video.addEventListener("mousemove", showBtn);
    video.addEventListener("mouseleave", () => hideBtn(1200));

    // When Reel, Short, or Video starts playing, show the button for 3 seconds so the user knows it's ready!
    video.addEventListener("play", () => {
      showBtn();
      hideBtn(3000);
    });
    video.addEventListener("playing", () => {
      showBtn();
      hideBtn(3000);
    });

    // 2. Ancestor container events (catches hover over overlay layers on Instagram, TikTok, Shorts)
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

    // 3. Keep button positioned on scroll and window resize
    window.addEventListener("scroll", updatePosition, { passive: true });
    window.addEventListener("resize", updatePosition, { passive: true });

    // 4. Hovering on button keeps it visible
    btn.addEventListener("mouseenter", () => clearTimeout(hideTimeout));
    btn.addEventListener("mouseleave", () => hideBtn(1200));

    // 5. Click handler
    btn.addEventListener("click", async (e) => {
      e.stopPropagation();
      e.preventDefault();

      const originalText = "Download this video";
      const textElem = btn.querySelector(".myownidm-text");

      let targetUrl = video.currentSrc || video.src;

      // Detect Platform
      const host = window.location.hostname;
      const isYouTube = host.includes("youtube.com") || host.includes("youtu.be");
      const isInstagram = host.includes("instagram.com");
      const isTikTok = host.includes("tiktok.com");

      // Extract title
      let rawTitle = "";
      const ytTitleEl = document.querySelector(
        "h1.ytd-watch-metadata yt-formatted-string, #title h1 yt-formatted-string, ytd-watch-metadata #title yt-formatted-string, ytd-reel-player-header-renderer h2"
      );
      if (ytTitleEl && ytTitleEl.textContent.trim()) {
        rawTitle = ytTitleEl.textContent.trim();
      } else if (document.title) {
        rawTitle = document.title;
      }
      const safeTitle = cleanFilename(rawTitle, isInstagram ? "instagram_video" : isTikTok ? "tiktok_video" : "video");

      if (isYouTube) {
        targetUrl = window.location.href;
      } else if (isInstagram) {
        // On Instagram: strip bytestart/byteend from direct CDN URL to download full video
        if (targetUrl && targetUrl.startsWith("http") && !targetUrl.includes(".m4s")) {
          targetUrl = stripByteRanges(targetUrl);
        } else {
          try {
            const resp = await new Promise((resolve) => {
              chrome.runtime.sendMessage({ action: "get-detected-media", tabId: null }, resolve);
            });
            const valid = resp?.media?.find(m => m.url.includes(".mp4") && !m.url.includes(".m4s"));
            if (valid) {
              targetUrl = stripByteRanges(valid.url);
            } else {
              targetUrl = window.location.href;
            }
          } catch (err) {
            targetUrl = window.location.href;
          }
        }
      } else if (isTikTok) {
        if (!targetUrl || targetUrl.startsWith("blob:")) {
          targetUrl = window.location.href;
        }
      } else if (!targetUrl || targetUrl.startsWith("blob:")) {
        try {
          const resp = await new Promise((resolve) => {
            chrome.runtime.sendMessage({ action: "get-detected-media", tabId: null }, resolve);
          });
          if (resp?.media && resp.media.length > 0) {
            const stream = resp.media.find(m => m.type === "stream" || m.url.includes(".m3u8") || m.url.includes("/pl/"));
            if (stream) {
              targetUrl = stream.url;
            } else {
              const valid = resp.media.filter(m => !m.url.includes("googlevideo.com") && !m.url.includes(".m4s"));
              if (valid.length > 0) {
                targetUrl = valid[valid.length - 1].url;
              } else {
                targetUrl = window.location.href;
              }
            }
          } else {
            targetUrl = window.location.href;
          }
        } catch (err) {
          targetUrl = window.location.href;
        }
      }

      // Universal byte-range parameter cleanup
      if (targetUrl) {
        targetUrl = stripByteRanges(targetUrl);
      }

      if (targetUrl) {
        btn.classList.remove("myownidm-success", "myownidm-error");
        textElem.textContent = "Connecting to IDM...";

        const payload = {
          action: "send-download",
          url: targetUrl,
          referer: window.location.href,
          filename: `${safeTitle}.mp4`
        };

        chrome.runtime.sendMessage(payload, (response) => {
          if (response && response.status === "ok") {
            btn.classList.add("myownidm-success");
            textElem.textContent = "Sent to My Own IDM ✓";
          } else {
            btn.classList.add("myownidm-error");
            textElem.textContent = "Cannot connect to IDM Desktop ✕";
          }

          setTimeout(() => {
            btn.classList.remove("myownidm-success", "myownidm-error");
            textElem.textContent = originalText;
          }, 3000);
        });
      }
    });
  }

  if (typeof window !== "undefined" && typeof document !== "undefined") {
    // Global mousemove check: if cursor is over ANY visible video's rect, trigger showBtn!
    let lastGlobalCheck = 0;
    window.addEventListener("mousemove", (e) => {
      const now = Date.now();
      if (now - lastGlobalCheck < 150) return;
      lastGlobalCheck = now;

      for (const [video, btn] of attachedButtons.entries()) {
        if (!video.isConnected) continue;
        const vRect = video.getBoundingClientRect();
        if (
          e.clientX >= vRect.left &&
          e.clientX <= vRect.right &&
          e.clientY >= vRect.top &&
          e.clientY <= vRect.bottom
        ) {
          btn.style.display = "flex";
          btn.style.opacity = "1";
          btn.style.pointerEvents = "auto";
          const btnWidth = btn.offsetWidth || 165;
          btn.style.top = `${Math.max(10, vRect.top + 12)}px`;
          btn.style.left = `${Math.max(10, vRect.right - btnWidth - 12)}px`;
        }
      }
    }, { passive: true });

    // Periodic scan & MutationObserver for single page applications (Instagram, TikTok, YouTube Shorts, etc.)
    const observer = new MutationObserver(() => {
      scanAndAttach();
    });

    observer.observe(document.body || document.documentElement, {
      childList: true,
      subtree: true
    });

    setInterval(scanAndAttach, 1500);

    // Initial scan
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", scanAndAttach);
    } else {
      scanAndAttach();
    }
  }

  if (typeof module !== "undefined" && module.exports) {
    module.exports = { cleanFilename, stripByteRanges };
  }
})();

