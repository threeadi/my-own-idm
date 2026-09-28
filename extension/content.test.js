import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
// @ts-expect-error CommonJS import in ESM
import content from './content.js';
const {
  cleanFilename,
  isGenericTitle,
  resolveSmartFilename,
  stripByteRanges,
  classifyResolution,
  formatDuration,
  generateRealMediaItems,
  generateQualityPresets,
  safeSendMessage,
  dismissVideo,
  isVideoDismissed,
  ensureTopmost,
  isWatchOrPlayerPage,
  isEligiblePlayerVideo
} = content;

describe('safeSendMessage', () => {
  const originalChrome = globalThis.chrome;
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.chrome = originalChrome;
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it('safely handles undefined chrome without throwing', async () => {
    globalThis.chrome = undefined;
    globalThis.fetch = vi.fn().mockResolvedValue({ ok: true });

    const result = await safeSendMessage({
      action: 'send-download',
      url: 'https://example.com/video.mp4',
      filename: 'video.mp4'
    });

    expect(result).toEqual({ status: 'ok' });
    expect(globalThis.fetch).toHaveBeenCalledWith(
      'http://127.0.0.1:18888/download',
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('sends via chrome.runtime.sendMessage when available', async () => {
    const mockSendMessage = vi.fn((_payload, callback) => {
      callback({ status: 'ok' });
    });
    globalThis.chrome = {
      runtime: {
        sendMessage: mockSendMessage
      }
    };

    const result = await safeSendMessage({ action: 'test' });
    expect(result).toEqual({ status: 'ok' });
    expect(mockSendMessage).toHaveBeenCalled();
  });

  it('handles chrome.runtime.lastError gracefully', async () => {
    const mockSendMessage = vi.fn((_payload, callback) => {
      globalThis.chrome.runtime.lastError = {
        message: 'Could not establish connection. Receiving end does not exist.'
      };
      callback(undefined);
    });

    globalThis.chrome = {
      runtime: {
        sendMessage: mockSendMessage,
        lastError: undefined
      }
    };

    const result = await safeSendMessage({ action: 'get-detected-media' });
    expect(result).toBeNull();
  });
});

describe('classifyResolution', () => {
  it('correctly classifies standard resolutions from height and width', () => {
    expect(classifyResolution(2160, 3840).quality).toBe('2160p');
    expect(classifyResolution(2160, 3840).badge).toBe('4K');
    expect(classifyResolution(1440, 2560).quality).toBe('1440p');
    expect(classifyResolution(1080, 1920).quality).toBe('1080p');
    expect(classifyResolution(1080, 1920).badge).toBe('FHD');
    expect(classifyResolution(720, 1280).quality).toBe('720p');
    expect(classifyResolution(720, 1280).badge).toBe('HD');
    expect(classifyResolution(480, 854).quality).toBe('480p');
    expect(classifyResolution(480, 854).badge).toBe('SD');
    expect(classifyResolution(360, 640).quality).toBe('360p');
  });

  it('infers resolution from URL or text hints when height is zero', () => {
    expect(classifyResolution(0, 0, 'https://example.com/video_1080p.mp4').quality).toBe('1080p');
    expect(classifyResolution(0, 0, 'Breaking News 720p HD').quality).toBe('720p');
    expect(classifyResolution(0, 0, 'trailer_4k.webm').quality).toBe('2160p');
    expect(classifyResolution(0, 0, 'random_video.mp4').quality).toBe('original');
  });
});

describe('formatDuration', () => {
  it('formats seconds into MM:SS string', () => {
    expect(formatDuration(65)).toBe('1:05');
    expect(formatDuration(180)).toBe('3:00');
    expect(formatDuration(0)).toBe('');
    expect(formatDuration(null)).toBe('');
  });
});

describe('generateRealMediaItems', () => {
  it('returns truthful items for a generic web video without fake resolutions or subtitles', () => {
    const mockVideo = {
      videoHeight: 720,
      videoWidth: 1280,
      currentSrc: 'https://twitter.com/vid.mp4',
      duration: 45
    };
    const items = generateRealMediaItems(mockVideo, [], 'x.com', 'Funny Cat Clip');
    
    // Exactly 2 items: The 720p video itself, and audio extraction
    expect(items).toHaveLength(2);
    expect(items[0].id).toBe('main_video');
    expect(items[0].badge).toBe('HD');
    expect(items[0].quality).toBe('720p');
    expect(items[0].title).toContain('720p HD');
    expect(items[0].filename).toBe('Funny_Cat_Clip_720p.mp4');

    expect(items[1].id).toBe('audio');
    expect(items[1].is_audio_only).toBe(true);

    // Verify absolutely NO fake 4K or fake subtitles
    expect(items.some((item) => item.quality === '2160p')).toBe(false);
    expect(items.some((item) => item.quality === 'subtitle')).toBe(false);
  });

  it('includes subtitle only when track element actually exists', () => {
    const mockVideo = {
      videoHeight: 1080,
      videoWidth: 1920,
      currentSrc: 'https://example.com/movie.mp4',
      querySelectorAll: (sel) => {
        if (sel.includes('subtitles')) {
          return [{ srclang: 'en', label: 'English', src: 'https://example.com/en.vtt' }];
        }
        return [];
      }
    };
    const items = generateRealMediaItems(mockVideo, [], 'movie-site.com', 'Feature Film');
    expect(items.some((item) => item.quality === 'subtitle')).toBe(true);
    const sub = items.find((item) => item.quality === 'subtitle');
    expect(sub?.title).toContain('English');
    expect(sub?.url).toBe('https://example.com/en.vtt');
  });

  it('generates real YouTube qualities capped at actual playing resolution', () => {
    // 720p video on YouTube should NOT offer fake 4K or 1080p
    const mockYt720 = {
      videoHeight: 720,
      videoWidth: 1280
    };
    const items720 = generateRealMediaItems(mockYt720, [], 'youtube.com', 'Indie Vlog');
    const qualities720 = items720.map((item) => item.quality);
    expect(qualities720).toEqual(['720p', '480p', 'audio']);
    expect(qualities720).not.toContain('2160p');
    expect(qualities720).not.toContain('1080p');

    // 4K video on YouTube
    const mockYt4k = {
      videoHeight: 2160,
      videoWidth: 3840
    };
    const items4k = generateRealMediaItems(mockYt4k, [], 'youtube.com', 'Nature in 4K');
    const qualities4k = items4k.map((item) => item.quality);
    expect(qualities4k).toContain('2160p');
    expect(qualities4k).toContain('1080p');
    expect(qualities4k).toContain('720p');
    expect(qualities4k).toContain('audio');
  });

  it('handles multiple sniffed media streams accurately', () => {
    const sniffed = [
      { url: 'https://cdn.example.com/stream_1080p.m3u8', type: 'stream', format: 'M3U8', sizeFormatted: '' },
      { url: 'https://cdn.example.com/stream_720p.m3u8', type: 'stream', format: 'M3U8', sizeFormatted: '' }
    ];
    const items = generateRealMediaItems(null, sniffed, 'streamer.com', 'Live Show');
    expect(items.length).toBe(3); // 2 streams + 1 audio option
    expect(items[0].url).toBe('https://cdn.example.com/stream_1080p.m3u8');
    expect(items[1].url).toBe('https://cdn.example.com/stream_720p.m3u8');
    expect(items[2].id).toBe('audio');
  });
});

describe('stripByteRanges', () => {
  it('strips bytestart and byteend from Instagram CDN URL', () => {
    const url = 'https://instagram.fsub15-1.fna.fbcdn.net/v/t2/m367/file.mp4?bytestart=0&byteend=817&efg=123';
    const cleaned = stripByteRanges(url);
    expect(cleaned).not.toContain('bytestart');
    expect(cleaned).not.toContain('byteend');
    expect(cleaned).toContain('efg=123');
  });

  it('handles URL with no byte parameters', () => {
    const url = 'https://example.com/video.mp4?token=abc';
    expect(stripByteRanges(url)).toBe(url);
  });

  it('handles null and invalid inputs gracefully', () => {
    expect(stripByteRanges(null)).toBeNull();
    expect(stripByteRanges(undefined)).toBeUndefined();
    expect(stripByteRanges('')).toBe('');
  });
});

describe('cleanFilename', () => {
  it('removes platform suffixes', () => {
    expect(cleanFilename('Awesome Video - YouTube')).toBe('Awesome_Video');
    expect(cleanFilename('Breaking News / X')).toBe('Breaking_News');
    expect(cleanFilename('Funny Reel • Instagram')).toBe('Funny_Reel');
  });

  it('removes URLs inside titles', () => {
    expect(cleanFilename('Check this out https://example.com/now cool')).toBe('Check_this_out_cool');
  });

  it('replaces forbidden characters with underscores', () => {
    expect(cleanFilename('test:file*name?with"bad/chars')).toBe('test_file_name_with_bad_chars');
  });

  it('truncates overly long titles to specified max length (default 80)', () => {
    const long = 'a'.repeat(120);
    const cleaned = cleanFilename(long);
    expect(cleaned.length).toBeLessThanOrEqual(80);

    const cleaned50 = cleanFilename(long, 'video', 50);
    expect(cleaned50.length).toBeLessThanOrEqual(50);
  });

  it('falls back to default name when empty', () => {
    expect(cleanFilename('')).toBe('video');
    expect(cleanFilename('   ', 'fallback')).toBe('fallback');
  });
});

describe('isGenericTitle', () => {
  it('detects generic embed and player titles', () => {
    expect(isGenericTitle('Embed')).toBe(true);
    expect(isGenericTitle('embed.mp4')).toBe(true);
    expect(isGenericTitle('video')).toBe(true);
    expect(isGenericTitle('Player')).toBe(true);
    expect(isGenericTitle('Video Player')).toBe(true);
    expect(isGenericTitle('untitled')).toBe(true);
    expect(isGenericTitle('')).toBe(true);
    expect(isGenericTitle(null)).toBe(true);
  });

  it('recognizes non-generic real titles', () => {
    expect(isGenericTitle('Msbreewc Update Lingerie Hijau Ajak Omek Bareng Squirt - Bokephub')).toBe(false);
    expect(isGenericTitle('My Vacation Vlog')).toBe(false);
  });
});

describe('resolveSmartFilename', () => {
  it('replaces generic Embed prefix with parent tab title while preserving quality and extension', () => {
    const resolved = resolveSmartFilename(
      'Embed_720p.mp4',
      'Msbreewc Update Lingerie Hijau Ajak Omek Bareng Squirt - Bokephub',
      '720p'
    );
    expect(resolved).toBe('Msbreewc_Update_Lingerie_Hijau_Ajak_Omek_Bareng_Squirt_-_Bokephub_720p.mp4');
  });

  it('handles empty requestedFilename by generating from tab title and quality', () => {
    const resolved = resolveSmartFilename(
      '',
      'Awesome Documentary',
      '1080p'
    );
    expect(resolved).toBe('Awesome_Documentary_1080p.mp4');
  });

  it('preserves non-generic specific filename', () => {
    const resolved = resolveSmartFilename(
      'Documentary_Full_720p.mp4',
      'Generic Web Site - Video Watcher',
      '720p'
    );
    expect(resolved).toBe('Documentary_Full_720p.mp4');
  });

  it('falls back to requestedFilename when tabTitle is generic or missing', () => {
    expect(resolveSmartFilename('Embed_720p.mp4', 'Embed', '720p')).toBe('Embed_720p.mp4');
    expect(resolveSmartFilename('Embed_720p.mp4', '', '720p')).toBe('Embed_720p.mp4');
    expect(resolveSmartFilename('', '', '720p')).toBe('video.mp4');
  });
});

describe('dismissVideo and isVideoDismissed', () => {
  it('correctly tracks and checks dismissed video elements', () => {
    const mockVideo1 = { id: 'v1' };
    const mockVideo2 = { id: 'v2' };

    expect(isVideoDismissed(mockVideo1)).toBe(false);
    expect(isVideoDismissed(mockVideo2)).toBe(false);

    dismissVideo(mockVideo1);

    expect(isVideoDismissed(mockVideo1)).toBe(true);
    expect(isVideoDismissed(mockVideo2)).toBe(false);
  });

  it('handles null/undefined gracefully without throwing', () => {
    expect(isVideoDismissed(null)).toBe(false);
    expect(isVideoDismissed(undefined)).toBe(false);
    expect(() => dismissVideo(null)).not.toThrow();
  });
});

describe('ensureTopmost', () => {
  it('re-appends btn and panel to be the last children over any ads', () => {
    const originalDoc = globalThis.document;
    const body = {
      children: [],
      lastElementChild: null,
      appendChild(child) {
        const idx = this.children.indexOf(child);
        if (idx !== -1) this.children.splice(idx, 1);
        this.children.push(child);
        child.parentElement = this;
        this.lastElementChild = child;
      }
    };

    const mockDoc = {
      fullscreenElement: null,
      body,
      documentElement: body
    };

    globalThis.document = mockDoc;

    const mockBtn = {
      parentElement: null,
      style: {
        setProperty: vi.fn()
      }
    };
    const mockPanel = {
      parentElement: null,
      style: {
        setProperty: vi.fn()
      }
    };

    // Initial attachment
    ensureTopmost(mockBtn, mockPanel);

    expect(body.children).toEqual([mockBtn, mockPanel]);
    expect(body.lastElementChild).toBe(mockPanel);
    expect(mockBtn.style.setProperty).toHaveBeenCalledWith('z-index', '2147483647', 'important');
    expect(mockPanel.style.setProperty).toHaveBeenCalledWith('z-index', '2147483647', 'important');

    // Simulate an ad element appended after IDM floating bar
    const adOverlay = { parentElement: null, style: {} };
    body.appendChild(adOverlay);
    expect(body.lastElementChild).toBe(adOverlay);

    // Call ensureTopmost again - IDM must re-append itself to be on top of the ad
    ensureTopmost(mockBtn, mockPanel);
    expect(body.children).toEqual([adOverlay, mockBtn, mockPanel]);
    expect(body.lastElementChild).toBe(mockPanel);

    globalThis.document = originalDoc;
  });

  it('prioritizes fullscreenElement when in fullscreen mode', () => {
    const originalDoc = globalThis.document;
    const fsContainer = {
      children: [],
      lastElementChild: null,
      appendChild(child) {
        this.children.push(child);
        child.parentElement = this;
        this.lastElementChild = child;
      }
    };

    const mockDoc = {
      fullscreenElement: fsContainer,
      body: { children: [], appendChild: vi.fn() },
      documentElement: {}
    };

    globalThis.document = mockDoc;

    const mockBtn = { parentElement: null, style: { setProperty: vi.fn() } };
    const mockPanel = { parentElement: null, style: { setProperty: vi.fn() } };

    ensureTopmost(mockBtn, mockPanel);

    expect(fsContainer.children).toEqual([mockBtn, mockPanel]);
    expect(fsContainer.lastElementChild).toBe(mockPanel);

    globalThis.document = originalDoc;
  });

  it('handles null/undefined btn without throwing', () => {
    expect(() => ensureTopmost(null, null)).not.toThrow();
    expect(() => ensureTopmost(undefined, undefined)).not.toThrow();
  });
});

describe('isWatchOrPlayerPage', () => {
  const originalLocation = globalThis.window?.location;

  afterEach(() => {
    if (globalThis.window) {
      // @ts-expect-error test mock
      globalThis.window.location = originalLocation;
    }
  });

  it('correctly identifies YouTube watch and shorts pages while excluding feeds', () => {
    // @ts-expect-error test mock
    globalThis.window = {
      location: { hostname: 'www.youtube.com', pathname: '/watch' }
    };
    expect(isWatchOrPlayerPage()).toBe(true);

    // @ts-expect-error test mock
    globalThis.window.location = { hostname: 'www.youtube.com', pathname: '/shorts/abc123' };
    expect(isWatchOrPlayerPage()).toBe(true);

    // @ts-expect-error test mock
    globalThis.window.location = { hostname: 'www.youtube.com', pathname: '/embed/abc123' };
    expect(isWatchOrPlayerPage()).toBe(true);

    // Excluded YouTube pages: Home, feed, subscriptions, channel
    // @ts-expect-error test mock
    globalThis.window.location = { hostname: 'www.youtube.com', pathname: '/' };
    expect(isWatchOrPlayerPage()).toBe(false);

    // @ts-expect-error test mock
    globalThis.window.location = { hostname: 'www.youtube.com', pathname: '/feed/subscriptions' };
    expect(isWatchOrPlayerPage()).toBe(false);

    // @ts-expect-error test mock
    globalThis.window.location = { hostname: 'www.youtube.com', pathname: '/results' };
    expect(isWatchOrPlayerPage()).toBe(false);
  });

  it('correctly identifies TikTok video pages and excludes feeds', () => {
    // @ts-expect-error test mock
    globalThis.window = {
      location: { hostname: 'www.tiktok.com', pathname: '/@user/video/12345' }
    };
    expect(isWatchOrPlayerPage()).toBe(true);

    // @ts-expect-error test mock
    globalThis.window.location = { hostname: 'www.tiktok.com', pathname: '/foryou' };
    expect(isWatchOrPlayerPage()).toBe(false);
  });
});

describe('isEligiblePlayerVideo', () => {
  it('rejects dismissed videos or disconnected videos', () => {
    const video = { isConnected: false, getBoundingClientRect: () => ({ width: 640, height: 360 }) };
    expect(isEligiblePlayerVideo(video)).toBe(false);

    const video2 = { isConnected: true, getBoundingClientRect: () => ({ width: 640, height: 360 }) };
    dismissVideo(video2);
    expect(isEligiblePlayerVideo(video2)).toBe(false);
  });

  it('rejects videos inside feed listing containers', () => {
    const video = {
      isConnected: true,
      getBoundingClientRect: () => ({ width: 640, height: 360 }),
      closest: (sel) => sel.includes('ytd-rich-item-renderer')
    };
    expect(isEligiblePlayerVideo(video)).toBe(false);
  });

  it('rejects tiny hover thumbnails and preview clips', () => {
    const tinyVideo = {
      isConnected: true,
      getBoundingClientRect: () => ({ width: 120, height: 80 }),
      closest: () => null
    };
    expect(isEligiblePlayerVideo(tinyVideo)).toBe(false);
  });

  it('accepts valid full-sized player videos', () => {
    // @ts-expect-error test mock
    globalThis.window = {
      location: { hostname: 'example.com', pathname: '/video' }
    };
    const playerVideo = {
      isConnected: true,
      getBoundingClientRect: () => ({ width: 854, height: 480 }),
      closest: () => null
    };
    expect(isEligiblePlayerVideo(playerVideo)).toBe(true);
  });
});

