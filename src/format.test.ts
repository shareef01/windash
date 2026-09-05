import { describe, it, expect } from "vitest";
import {
  fmtUptime,
  fmtUpdated,
  fmtMem,
  diskLabel,
  filterProcs,
  friendlyError,
} from "./format";

describe("fmtUptime", () => {
  it("formats minutes under an hour", () => {
    expect(fmtUptime(0)).toBe("0m");
    expect(fmtUptime(59)).toBe("0m");
    expect(fmtUptime(60)).toBe("1m");
    expect(fmtUptime(3599)).toBe("59m");
  });
  it("formats hours and minutes", () => {
    expect(fmtUptime(3600)).toBe("1h 0m");
    expect(fmtUptime(3660)).toBe("1h 1m");
  });
  it("formats days instead of huge hour counts", () => {
    expect(fmtUptime(12 * 86400)).toBe("12d 0h");
    expect(fmtUptime(12 * 86400 + 3600)).toBe("12d 1h");
  });
  it("handles invalid input", () => {
    expect(fmtUptime(-1)).toBe("—");
    expect(fmtUptime(Number.NaN)).toBe("—");
  });
});

describe("fmtUpdated", () => {
  const now = Date.parse("2026-09-05T12:00:00Z");
  it("says just now for fresh timestamps", () => {
    expect(fmtUpdated("2026-09-05T12:00:00Z", now)).toBe("just now");
    expect(fmtUpdated("2026-09-05T11:59:59Z", now)).toBe("just now");
  });
  it("uses seconds then minutes", () => {
    expect(fmtUpdated("2026-09-05T11:59:50Z", now)).toBe("10s ago");
    expect(fmtUpdated("2026-09-05T11:58:00Z", now)).toBe("2m ago");
  });
  it("returns empty for invalid timestamps", () => {
    expect(fmtUpdated("not-a-date", now)).toBe("");
  });
});

describe("fmtMem", () => {
  it("uses GB with one decimal above 1 GiB", () => {
    expect(fmtMem(1024 * 1024 * 1024)).toBe("1.0 GB");
    expect(fmtMem(1.2 * 1024 * 1024 * 1024)).toBe("1.2 GB");
  });
  it("uses whole MB below 1 GiB", () => {
    expect(fmtMem(890 * 1024 * 1024)).toBe("890 MB");
  });
});

describe("diskLabel", () => {
  it("normalizes Windows drive letters", () => {
    expect(diskLabel("C:\\")).toBe("C:");
    expect(diskLabel("c:")).toBe("C:");
    expect(diskLabel("D:\\Data")).toBe("D:");
  });
  it("falls back to mount or name", () => {
    expect(diskLabel("/mnt/data")).toBe("/mnt/data");
    expect(diskLabel("", "OS")).toBe("OS");
  });
});

describe("filterProcs", () => {
  const procs = [
    { name: "chrome.exe", exe: "C:\\Program Files\\Google\\Chrome\\chrome.exe" },
    { name: "Code.exe", exe: "C:\\Users\\me\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe" },
    { name: "svchost.exe", exe: "C:\\Windows\\System32\\svchost.exe" },
  ];
  it("returns all when query is empty", () => {
    expect(filterProcs(procs, "  ")).toHaveLength(3);
  });
  it("matches name case-insensitively", () => {
    expect(filterProcs(procs, "CHROME").map((p) => p.name)).toEqual(["chrome.exe"]);
  });
  it("matches executable path", () => {
    expect(filterProcs(procs, "vs code").map((p) => p.name)).toEqual(["Code.exe"]);
  });
});

describe("friendlyError", () => {
  it("maps process termination failures", () => {
    expect(friendlyError("Unable to terminate process 123. Administrator permission may be required.").message)
      .toMatch(/administrator/i);
    expect(friendlyError("Process 4 is a protected Windows system process and cannot be terminated.").message)
      .toMatch(/protecting/i);
  });
  it("keeps technical detail for inspection", () => {
    const e = friendlyError("spawn taskkill: access denied", "Couldn't end that process.");
    expect(e.detail).toContain("taskkill");
    expect(e.message).not.toContain("spawn taskkill");
  });
  it("uses fallback for unknown errors", () => {
    expect(friendlyError("weird rust panic at metrics.rs:12", "Couldn't refresh system metrics.").message)
      .toBe("Couldn't refresh system metrics.");
  });
});
