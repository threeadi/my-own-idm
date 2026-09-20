document.addEventListener("DOMContentLoaded", async () => {
  const toggle = document.getElementById("interceptor-toggle");
  const mediaContainer = document.getElementById("media-container");

  // Get active tab
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });

  if (tab?.id) {
    chrome.runtime.sendMessage(
      { action: "get-detected-media", tabId: tab.id },
      (response) => {
        if (response) {
          toggle.checked = response.isInterceptorEnabled;
          renderMedia(response.media, tab.url);
        }
      }
    );
  }

  toggle.addEventListener("change", () => {
    chrome.runtime.sendMessage({
      action: "toggle-interceptor",
      enabled: toggle.checked
    });
  });

  function renderMedia(mediaList, pageUrl) {
    if (!mediaList || mediaList.length === 0) {
      mediaContainer.innerHTML = '<div class="empty-state">No video or audio stream detected on this page.</div>';
      return;
    }

    mediaContainer.innerHTML = "";
    mediaList.forEach((m, idx) => {
      const item = document.createElement("div");
      item.className = "media-item";

      const info = document.createElement("div");
      info.className = "media-info";

      const urlDiv = document.createElement("div");
      urlDiv.className = "media-url";
      urlDiv.textContent = m.url;
      urlDiv.title = m.url;

      const tag = document.createElement("div");
      tag.className = "media-tag";
      tag.textContent = m.url.includes(".m3u8") ? "HLS STREAM" : "MEDIA";

      info.appendChild(urlDiv);
      info.appendChild(tag);

      const btn = document.createElement("button");
      btn.className = "btn-dl";
      btn.textContent = "Download";
      btn.addEventListener("click", () => {
        btn.textContent = "Sent ✓";
        btn.style.background = "#10b981";
        chrome.runtime.sendMessage({
          action: "send-download",
          url: m.url,
          referer: pageUrl
        });
      });

      item.appendChild(info);
      item.appendChild(btn);
      mediaContainer.appendChild(item);
    });
  }
});
