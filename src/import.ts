// CSV/JSON import into tables (issue #32). Mirrors the export path in reverse.
import type { Row } from "./api";

function parseCsvLine(line: string): string[] {
  const result: string[] = [];
  let current = "";
  let inQuotes = false;

  for (let i = 0; i < line.length; i++) {
    const char = line[i];
    const nextChar = line[i + 1];

    if (inQuotes) {
      if (char === '"' && nextChar === '"') {
        current += '"';
        i++;
      } else if (char === '"') {
        inQuotes = false;
      } else {
        current += char;
      }
    } else {
      if (char === '"') {
        inQuotes = true;
      } else if (char === ",") {
        result.push(current);
        current = "";
      } else {
        current += char;
      }
    }
  }

  result.push(current);
  return result;
}

export function csvToRows(content: string): Row[] {
  const rows: Row[] = [];
  if (content.length === 0) return rows;

  // Split by line but keep track of quoted fields that span lines
  const lines = content.split(/\r\n|\n/);
  if (lines.length === 0) return [];

  const headers = parseCsvLine(lines[0]);
  let currentRow: string[] = [];
  let currentField = "";
  let inQuotes = false;

  for (let i = 1; i < lines.length; i++) {
    const line = lines[i];
    for (let j = 0; j < line.length; j++) {
      const char = line[j];
      const nextChar = line[j + 1];

      if (inQuotes) {
        if (char === '"' && nextChar === '"') {
          currentField += '"';
          j++;
        } else if (char === '"') {
          inQuotes = false;
        } else {
          currentField += char;
        }
      } else {
        if (char === '"') {
          inQuotes = true;
        } else if (char === ",") {
          currentRow.push(currentField);
          currentField = "";
        } else {
          currentField += char;
        }
      }
    }

    // Handle newline in quoted field
    if (inQuotes) {
      currentField += "\n";
    } else {
      // End of line, complete the field and row
      if (currentField.length > 0 || currentRow.length > 0) {
        currentRow.push(currentField);
        currentField = "";

        // If we have the right number of fields, add it as a row
        if (currentRow.length === headers.length) {
          const row: Row = {};
          for (let j = 0; j < headers.length; j++) {
            row[headers[j]] = currentRow[j] ?? "";
          }
          rows.push(row);
          currentRow = [];
        }
      }
    }
  }

  // Handle any remaining data
  if (currentField.length > 0 || currentRow.length > 0) {
    currentRow.push(currentField);
    if (currentRow.length === headers.length) {
      const row: Row = {};
      for (let j = 0; j < headers.length; j++) {
        row[headers[j]] = currentRow[j] ?? "";
      }
      rows.push(row);
    }
  }

  return rows;
}

export function jsonToRows(content: string): Row[] {
  const data = JSON.parse(content);
  if (!Array.isArray(data)) {
    throw new Error("JSON must be an array of objects");
  }
  return data;
}

export async function openImportFile(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const path = await open({
    filters: [{ name: "CSV/JSON", extensions: ["csv", "json"] }],
  });
  return path;
}
