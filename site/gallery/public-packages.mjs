import { readFile } from 'node:fs/promises';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { DEFAULT_CATALOG, readCatalog, readPublicPackage, latestPackages } from '../../apps/viewer-ui/public-packages.js';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');

export async function fetchPackageResource(url) {
  if (url.startsWith('file:')) return new Response(await readFile(new URL(url)));
  return fetch(url);
}

/** The codes of the site's languages, from i18n/languages.json, or the registry RNL_SITE_LANGUAGES names. */
function siteLanguages() {
  const registry = JSON.parse(readFileSync(process.env.RNL_SITE_LANGUAGES || join(root, 'i18n', 'languages.json'), 'utf8'));
  return registry.languages.map(l => l.code);
}

/**
 * The packages with the translations of their summaries that rnl-packages publishes beside its catalog,
 * summaries/<code>.json, an object keyed by the English of a summary (decision 0015: a record has its English
 * summary, and its translations are outside it). A record's own translation comes first. A language without
 * that file, or whose file cannot be read, shows the English.
 */
export async function withTranslatedSummaries(packages, catalogUrl, codes, fetcher = fetchPackageResource) {
  const norm = text => String(text).split(/\s+/).filter(Boolean).join(' ');
  const files = {};
  await Promise.all(codes.filter(code => code !== 'en').map(async code => {
    const url = new URL(`summaries/${code}.json`, catalogUrl).href;
    try {
      const response = await fetcher(url);
      if (response.ok) files[code] = JSON.parse(new TextDecoder().decode(await response.arrayBuffer()));
      else if (response.status !== 404) console.warn(`${url}: ${response.status}; the summaries are shown in English in ${code}`);
    } catch (error) {
      if (error.code !== 'ENOENT') console.warn(`${url}: ${error.message}; the summaries are shown in English in ${code}`);
    }
  }));
  return packages.map(pkg => {
    const found = Object.entries(files).map(([code, data]) => [code, data?.[norm(pkg.summary.en)]])
      .filter(([, text]) => typeof text === 'string' && text);
    return found.length ? { ...pkg, summary: { ...Object.fromEntries(found), ...pkg.summary } } : pkg;
  });
}

export async function publicGalleryPackages() {
  const setting = process.env.RNL_PACKAGE_CATALOG || DEFAULT_CATALOG;
  const url = /^[a-z]+:/i.test(setting) && !/^[a-z]:[\\/]/i.test(setting) ? setting : pathToFileURL(setting).href;
  const catalog = await readCatalog(url, fetchPackageResource);
  const latest = await withTranslatedSummaries(latestPackages(catalog.packages), url, siteLanguages());
  const packages = await Promise.all(latest.map(async pkg => ({
    ...pkg, text: new TextDecoder().decode(await readPublicPackage(pkg, fetchPackageResource)),
  })));
  return { packages, provenance: { url: setting, sha256: catalog.sha256 } };
}
