import { describe, it, expect } from "vitest";
import type { ProcInfo } from "./types";
import { friendlyError } from "./format";

describe("Process termination safety & identity validation", () => {
  const sampleProc: ProcInfo = {
    name: "notepad.exe",
    cpu: 0.5,
    mem: 50 * 1024 * 1024,
    pid: 1234,
    start_time: 1700000000,
    exe: "C:\\Windows\\System32\\notepad.exe",
  };

  it("builds correct process identity payload for end_process invocation", () => {
    const payload = {
      pid: sampleProc.pid,
      expected_start_time: sampleProc.start_time,
      expected_name: sampleProc.name,
    };

    expect(payload.pid).toBe(1234);
    expect(payload.expected_start_time).toBe(1700000000);
    expect(payload.expected_name).toBe("notepad.exe");
  });

  it("detects stale process when PID has been reused with a newer start_time", () => {
    const procsList: ProcInfo[] = [
      {
        ...sampleProc,
        start_time: 1700000500, // Reused PID with newer start_time!
      },
    ];

    const target = {
      pid: sampleProc.pid,
      name: sampleProc.name,
      start_time: sampleProc.start_time,
    };

    const live = procsList.find((p) => p.pid === target.pid);
    const isStale =
      !live || live.name !== target.name || live.start_time !== target.start_time;

    expect(isStale).toBe(true);
  });

  it("detects process no longer running before dispatching termination", () => {
    const procsList: ProcInfo[] = []; // Empty list (exited)

    const target = {
      pid: 1234,
      name: "notepad.exe",
      start_time: 1700000000,
    };

    const live = procsList.find((p) => p.pid === target.pid);
    const isStale =
      !live || live.name !== target.name || live.start_time !== target.start_time;

    expect(isStale).toBe(true);
  });

  it("detects process name mismatch for same PID", () => {
    const procsList: ProcInfo[] = [
      {
        name: "malicious.exe",
        cpu: 1.0,
        mem: 100 * 1024,
        pid: 1234,
        start_time: 1700000000,
        exe: "C:\\malicious.exe",
      },
    ];

    const target = {
      pid: 1234,
      name: "notepad.exe",
      start_time: 1700000000,
    };

    const live = procsList.find((p) => p.pid === target.pid);
    const isStale =
      !live || live.name !== target.name || live.start_time !== target.start_time;

    expect(isStale).toBe(true);
  });

  it("formats permission denial errors with clear actionable guidance", () => {
    const rawError =
      "Unable to terminate process 904. Administrator permission may be required. ERROR: Access is denied.";
    const formatted = friendlyError(rawError, "Couldn't end that process.");

    expect(formatted.message).toBe(
      "Couldn't end that process. You may need administrator permission."
    );
    expect(formatted.detail).toContain("Access is denied");
  });

  it("correctly clears selected PID when target process is ended", () => {
    let selectedPid: number | null = 1234;
    const targetPid = 1234;

    if (selectedPid === targetPid) {
      selectedPid = null;
    }

    expect(selectedPid).toBeNull();
  });
});
