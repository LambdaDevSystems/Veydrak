const UNITS = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '0 B';
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i += 1;
  }
  return `${i === 0 ? v : v.toFixed(1)} ${UNITS[i]}`;
}

/** Device names come from the network (mDNS) and are untrusted: strip control chars, cap length. */
export function sanitizeDeviceName(raw: string, max = 48): string {
  // eslint-disable-next-line no-control-regex
  const cleaned = raw.replace(/[\u0000-\u001f\u007f-\u009f‪-‮⁦-⁩]/g, '').trim();
  if (!cleaned) return 'unknown device';
  return cleaned.length > max ? `${cleaned.slice(0, max - 1)}…` : cleaned;
}
