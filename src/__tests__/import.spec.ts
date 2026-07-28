import { describe, expect, it } from "vitest";
import { csvToRows, jsonToRows } from "../import";
import { rowsToCsv, rowsToJson } from "../export";

describe("csvToRows", () => {
  it("returns an empty array for an empty string", () => {
    expect(csvToRows("")).toEqual([]);
  });

  it("parses a basic CSV with headers and one row", () => {
    expect(csvToRows("id,name\r\n1,alice")).toEqual([{ id: "1", name: "alice" }]);
  });

  it("handles fields with commas when quoted", () => {
    expect(csvToRows('v\r\n"a,b"')).toEqual([{ v: "a,b" }]);
  });

  it("handles escaped quotes in quoted fields", () => {
    expect(csvToRows('v\r\n"say ""hi"""')).toEqual([{ v: 'say "hi"' }]);
  });

  it("handles newlines in quoted fields", () => {
    expect(csvToRows('v\r\n"line1\nline2"')).toEqual([{ v: "line1\nline2" }]);
  });

  it("handles empty fields as empty strings", () => {
    expect(csvToRows("a,b\r\n,")).toEqual([{ a: "", b: "" }]);
  });

  it("skips blank lines", () => {
    expect(csvToRows("id,name\r\n1,alice\r\n\r\n2,bob")).toEqual([
      { id: "1", name: "alice" },
      { id: "2", name: "bob" },
    ]);
  });

  it("round-trips through export", () => {
    const rows = [{ id: 1, name: "alice" }];
    const csv = rowsToCsv(rows);
    const parsed = csvToRows(csv);
    expect(parsed).toEqual([{ id: "1", name: "alice" }]);
  });
});

describe("jsonToRows", () => {
  it("parses a JSON array of objects", () => {
    expect(jsonToRows("[{\"id\":1,\"name\":\"alice\"}]")).toEqual([{ id: 1, name: "alice" }]);
  });

  it("handles pretty-printed JSON", () => {
    const json = JSON.stringify([{ id: 1, name: "alice" }], null, 2);
    expect(jsonToRows(json)).toEqual([{ id: 1, name: "alice" }]);
  });

  it("rejects non-array JSON", () => {
    expect(() => jsonToRows('{"id":1}')).toThrow("JSON must be an array of objects");
  });

  it("round-trips through export", () => {
    const rows = [{ id: 1, name: "alice" }];
    const json = rowsToJson(rows);
    expect(jsonToRows(json)).toEqual(rows);
  });
});
