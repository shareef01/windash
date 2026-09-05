import { describe, it, expect } from "vitest";
import { usageColor, cpuColor, fmtRate, fmtSize, colors } from "./theme";

describe("usageColor", () => {
  it("is accent (calm) through ordinary utilization", () => {
    expect(usageColor(0)).toBe(colors.accent);
    expect(usageColor(74.9)).toBe(colors.accent);
  });
  it("is amber from 75% to just below 90%", () => {
    expect(usageColor(75)).toBe(colors.amber);
    expect(usageColor(89.9)).toBe(colors.amber);
  });
  it("is red at/above 90%", () => {
    expect(usageColor(90)).toBe(colors.red);
    expect(usageColor(100)).toBe(colors.red);
  });
});

describe("cpuColor", () => {
  it("is accent below 30%", () => {
    expect(cpuColor(0)).toBe(colors.accent);
    expect(cpuColor(29.9)).toBe(colors.accent);
  });
  it("is amber 30% to just below 70%", () => {
    expect(cpuColor(30)).toBe(colors.amber);
    expect(cpuColor(69.9)).toBe(colors.amber);
  });
  it("is red at/above 70% (single busy core can read 100%+)", () => {
    expect(cpuColor(70)).toBe(colors.red);
    expect(cpuColor(128.9)).toBe(colors.red);
  });
});

describe("fmtRate", () => {
  it("formats bytes/sec", () => {
    expect(fmtRate(512)).toBe("512 B/s");
  });
  it("formats KB/s with one decimal", () => {
    expect(fmtRate(1024)).toBe("1.0 KB/s");
    expect(fmtRate(60421)).toBe("59.0 KB/s");
  });
  it("formats MB/s above 1 MiB/s", () => {
    expect(fmtRate(1024 * 1024)).toBe("1.0 MB/s");
    expect(fmtRate(6 * 1024 * 1024)).toBe("6.0 MB/s");
  });
});

describe("fmtSize", () => {
  it("shows MB for sub-gigabyte values", () => {
    expect(fmtSize(200 * 1024 * 1024)).toBe("200 MB");
  });
  it("shows GB (rounded) from 1 GB upward", () => {
    expect(fmtSize(1024 * 1024 * 1024)).toBe("1 GB");
    expect(fmtSize(237 * 1024 * 1024 * 1024)).toBe("237 GB");
  });
  it("shows TB above 1024 GB", () => {
    expect(fmtSize(2048 * 1024 * 1024 * 1024)).toBe("2.0 TB");
  });
  it("never returns negative for zero/negative input", () => {
    expect(fmtSize(0)).toBe("0 MB");
  });
});
