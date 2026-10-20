import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { DEFAULT_CATALOG, readCatalog, readPublicPackage, latestPackages } from '../../apps/viewer-ui/public-packages.js';

export async function fetchPackageResource(url) {
  if (url.startsWith('file:')) return new Response(await readFile(new URL(url)));
  return fetch(url);
}

export async function publicGalleryPackages() {
  const setting = process.env.RNL_PACKAGE_CATALOG || DEFAULT_CATALOG;
  const url = /^[a-z]+:/i.test(setting) && !/^[a-z]:[\\/]/i.test(setting) ? setting : pathToFileURL(setting).href;
  const catalog = await readCatalog(url, fetchPackageResource);
  const packages = await Promise.all(latestPackages(catalog.packages).map(async pkg => ({
    ...pkg, text: new TextDecoder().decode(await readPublicPackage(pkg, fetchPackageResource)),
  })));
  return { packages, provenance: { url: setting, sha256: catalog.sha256 } };
}
