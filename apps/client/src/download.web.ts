import { exportWorkbook } from './api';

export async function downloadWorkbook(): Promise<void> {
  const file = await exportWorkbook();
  const url = URL.createObjectURL(file);
  const link = document.createElement('a');
  link.href = url;
  link.download = `skarma-${new Date().toISOString().slice(0, 10)}.xlsx`;
  link.style.display = 'none';
  document.body.appendChild(link);
  try { link.click(); } finally {
    link.remove();
    // Give mobile browsers time to hand the Blob to their download manager.
    setTimeout(() => URL.revokeObjectURL(url), 60_000);
  }
}
