"""Measure the actual idle Linux process, not whole-machine Task Manager totals."""
import json, os, pathlib, subprocess, sys, tempfile, time
binary = str(pathlib.Path(sys.argv[1]).resolve())
root = pathlib.Path(tempfile.mkdtemp(prefix="worldcraft-idle-"))
env = dict(os.environ, XDG_DATA_HOME=str(root / "data"))
with (root / "app.log").open("w") as log:
    process = subprocess.Popen([binary, "--eco", "--idle-audit"], cwd=root, env=env, stdout=log, stderr=log)
    try:
        deadline = time.monotonic() + 90
        while not (root / "idle-state.json").exists():
            if process.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError((root / "app.log").read_text(errors="replace"))
            time.sleep(0.25)
        def sample():
            stat = pathlib.Path(f"/proc/{process.pid}/stat").read_text().rsplit(")", 1)[1].split()
            rss = int(next(line.split()[1] for line in pathlib.Path(f"/proc/{process.pid}/status").read_text().splitlines() if line.startswith("VmRSS:")))
            return (int(stat[11]) + int(stat[12])) / os.sysconf("SC_CLK_TCK"), rss
        # Shader/font startup is excluded explicitly.
        time.sleep(5)
        cpu, rss = sample()
        start = time.monotonic()
        readings = []
        for _ in range(20):
            time.sleep(0.5)
            readings.append(sample())
        cpu_end, _ = readings[-1]
        report = {"scope": "Actual idle process at 640x360 on hosted software graphics; not a gaming-hardware benchmark", "sample_seconds": time.monotonic() - start,
                  "cpu_logical_cores": (cpu_end - cpu) / (time.monotonic() - start), "rss_start_kib": rss,
                  "rss_peak_kib": max(row[1] for row in readings), "state": json.loads((root / "idle-state.json").read_text())}
        assert report["state"]["world_camera_active"] is False
        pathlib.Path("idle-audit.json").write_text(json.dumps(report, indent=2))
        print(json.dumps(report, indent=2))
    finally:
        process.terminate()
        try: process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
