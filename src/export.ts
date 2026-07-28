// CSV/JSON export of grid rows (issue #12). Exports exactly what the grid is
// currently showing — the current page for paginated grids, not all rows.
import type { Row } from "./api";

function csvField(v: unknown): string {
  let s: string;
  if (v === null || v === undefined) s = "";
  else if (typeof v === "object") s = JSON.stringify(v);
  else s = String(v);
  if (/["\r\n,]/.test(s)) s = `"${s.replace(/"/g, '""')}"`;
  return s;
}

export function rowsToCsv(rows: Row[]): string {
  if (rows.length === 0) return "";
  const columns = Object.keys(rows[0]);
  const lines = [columns.map(csvField).join(",")];
  for (const r of rows) lines.push(columns.map((c) => csvField(r[c])).join(","));
  return lines.join("\r\n");
}

export function rowsToJson(rows: Row[]): string {
  return JSON.stringify(rows, null, 2);
}

export function downloadFile(filename: string, content: string, mimeType: string): void {
  const url = URL.createObjectURL(new Blob([content], { type: mimeType }));
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
