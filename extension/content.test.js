import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
// @ts-expect-error CommonJS import in ESM
import content from './content.js';
const { cleanFilename, stripByteRanges, generateQualityPresets, safeSendMessage, dismissVideo, isVideoDismissed } = content;

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

describe('generateQualityPresets', () => {
  it('generates 5 distinct quality presets for detected media', () => {
    const presets = generateQualityPresets('youtube.com', 'Cosmic Odyssey (2025) 4K');
    expect(presets).toHaveLength(5);

    const ids = presets.map((p) => p.id);
    expect(ids).toEqual(['4k', '1080p', '720p', 'audio', 'sub']);

    // Check 4K Ultra HD properties
    const p4k = presets.find((p) => p.id === '4k');
    expect(p4k?.badge).toBe('4K');
    expect(p4k?.quality).toBe('2160p');
    expect(p4k?.threads).toBe(32);
    expect(p4k?.filename).toContain('_4k.mp4');

    // Check 1080p FHD
    const pFhd = presets.find((p) => p.id === '1080p');
    expect(pFhd?.badge).toBe('FHD');
    expect(pFhd?.quality).toBe('1080p');
    expect(pFhd?.threads).toBe(16);

    // Check Audio Only
    const pAudio = presets.find((p) => p.id === 'audio');
    expect(pAudio?.is_audio_only).toBe(true);
    expect(pAudio?.badge).toBe('🎵');
    expect(pAudio?.filename).toContain('_audio.m4a');

    // Check Subtitle
    const pSub = presets.find((p) => p.id === 'sub');
    expect(pSub?.quality).toBe('subtitle');
    expect(pSub?.badge).toBe('SRT');
    expect(pSub?.filename).toContain('_sub_id.srt');
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

  it('truncates overly long titles to 50 characters', () => {
    const long = 'a'.repeat(80);
    const cleaned = cleanFilename(long);
    expect(cleaned.length).toBeLessThanOrEqual(50);
  });

  it('falls back to default name when empty', () => {
    expect(cleanFilename('')).toBe('video');
    expect(cleanFilename('   ', 'fallback')).toBe('fallback');
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
