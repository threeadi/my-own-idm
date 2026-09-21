import { describe, it, expect } from 'vitest';
import { getTranslation, translations, AVAILABLE_LANGUAGES, type TranslationKey } from './i18n';

describe('i18n Localization Engine', () => {
  it('includes both Indonesian and English in AVAILABLE_LANGUAGES', () => {
    const codes = AVAILABLE_LANGUAGES.map((l) => l.code);
    expect(codes).toContain('id');
    expect(codes).toContain('en');
    expect(AVAILABLE_LANGUAGES.length).toBe(2);
  });

  it('guarantees complete key parity between Indonesian and English dictionaries', () => {
    const idKeys = Object.keys(translations.id).sort();
    const enKeys = Object.keys(translations.en).sort();

    expect(idKeys).toEqual(enKeys);
    expect(idKeys.length).toBeGreaterThan(50);
  });

  it('translates keys properly in Indonesian and English', () => {
    expect(getTranslation('id', 'toolbar.addUrl')).toBe('Tambah URL');
    expect(getTranslation('en', 'toolbar.addUrl')).toBe('Add URL');

    expect(getTranslation('id', 'common.save')).toBe('Simpan');
    expect(getTranslation('en', 'common.save')).toBe('Save');

    expect(getTranslation('id', 'table.sortLabel')).toBe('Urut:');
    expect(getTranslation('en', 'table.sortLabel')).toBe('Sort:');

    expect(getTranslation('id', 'settings.tabGeneral')).toBe('Umum');
    expect(getTranslation('en', 'settings.tabGeneral')).toBe('General');
  });

  it('interpolates parameters into translated strings', () => {
    expect(getTranslation('id', 'table.filesCount', { count: 42 })).toBe('42 Berkas');
    expect(getTranslation('en', 'table.filesCount', { count: 42 })).toBe('42 Files');

    expect(getTranslation('id', 'toolbar.speedLimited', { value: '2 MB/s' })).toBe('Dibatasi: 2 MB/s');
    expect(getTranslation('en', 'toolbar.speedLimited', { value: '2 MB/s' })).toBe('Limited: 2 MB/s');
  });

  it('falls back to default language (id) when unknown language is specified', () => {
    expect(getTranslation('fr' as any, 'common.save')).toBe('Simpan');
  });

  it('falls back to key string when translation key is unknown', () => {
    expect(getTranslation('id', 'nonexistent.key' as any)).toBe('nonexistent.key');
  });
});
