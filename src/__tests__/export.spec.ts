import { describe, expect, it } from "vitest";
import { rowsToCsv, rowsToJson } from "../export";

describe("rowsToCsv", () => {
  it("returns an empty string for no rows", () => {
    expect(rowsToCsv([])).toBe("");
  });

  it("emits a header row from the column list", () => {
    expect(rowsToCsv([{ id: 1, name: "a" }])).toBe("id,name\r\n1,a");
  });

  it("quotes fields containing a comma", () => {
    expect(rowsToCsv([{ v: "a,b" }])).toBe('v\r\n"a,b"');
  });

  it("quotes fields containing a double quote, doubling it", () => {
    expect(rowsToCsv([{ v: 'say "hi"' }])).toBe('v\r\n"say ""hi"""');
  });

  it("quotes fields containing a newline", () => {
    expect(rowsToCsv([{ v: "line1\nline2" }])).toBe('v\r\n"line1\nline2"');
  });

  it("renders null/undefined as an empty field", () => {
    expect(rowsToCsv([{ a: null, b: undefined }])).toBe("a,b\r\n,");
  });

  it("renders nested JSON values as a compact string", () => {
    expect(rowsToCsv([{ tags: ["x", "y"] }])).toBe('tags\r\n"[""x"",""y""]"');
  });
});

describe("rowsToJson", () => {
  it("pretty-prints the rows as JSON", () => {
    expect(rowsToJson([{ id: 1 }])).toBe(JSON.stringify([{ id: 1 }], null, 2));
  });

  it("round-trips through JSON.parse", () => {
    const rows = [{ id: 1, name: "a", tags: ["x"] }];
    expect(JSON.parse(rowsToJson(rows))).toEqual(rows);
  });
});
