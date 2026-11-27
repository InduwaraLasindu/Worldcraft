"""Collect isolated native evidence for the photorealism fixtures."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import struct
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def run_fixture(command, directory, env):
    with subprocess.Popen(command, cwd=directory, env=env,
                          start_new_session=os.name != "nt") as process:
        try:
            code = process.wait(timeout=900)
        except subprocess.TimeoutExpired:
            if os.name == "nt":
                subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                               check=False, capture_output=True)
                process.kill()
            else:
                os.killpg(process.pid, signal.SIGKILL)
            raise
        if code:
            raise subprocess.CalledProcessError(code, command)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--case", action="append", help="Repeat to select several fixtures")
    parser.add_argument("--view", action="append", choices=["close", "mid", "wide", "route"])
    parser.add_argument("--gpu-gate", action="store_true")
    parser.add_argument("--reference-stills", action="store_true",
                        help="Capture native 2560 × 1440 stills at the reference settings")
    parser.add_argument("--xvfb", action="store_true")
    args = parser.parse_args()
    if args.gpu_gate and (sys.platform != "win32" or args.xvfb):
        parser.error("the GPU gate requires a Windows desktop")
    if args.reference_stills and (args.gpu_gate or "route" in (args.view or [])):
        parser.error("reference stills and timed routes run separately")
    manifest = json.loads((ROOT / "docs/fixtures/photorealism-v1.json").read_text())
    fixtures = {fixture["label"]: fixture for fixture in manifest["fixtures"]}
    if args.case and any(case not in fixtures for case in args.case):
        parser.error("unknown fixture in --case")
    if args.gpu_gate and (args.case or args.view):
        parser.error("the release GPU gate requires every fixture and all three runs")

    labels = args.case or list(fixtures)
    views = args.view or list(manifest["views"])
    if args.gpu_gate:
        views = ["route"]
    elif not args.reference_stills and not args.view:
        views.append("route")
    binary = args.binary.resolve(strict=True)
    with binary.open("rb") as stream:
        binary_hash = hashlib.file_digest(stream, "sha256").hexdigest()
    args.output.mkdir(parents=True, exist_ok=False)
    failures = []
    evidence = []
    for label in labels:
        fixture = fixtures[label]
        for view in views:
            for repetition in range(1, 4 if args.gpu_gate else 2):
                directory = args.output / label / view / str(repetition)
                directory.mkdir(parents=True)
                overrides = {
                    "WORLDCRAFT_PHOTOREAL_CASE": label,
                    "WORLDCRAFT_PHOTOREAL_VIEW": view,
                }
                if args.gpu_gate:
                    overrides["WORLDCRAFT_PHOTOREAL_GPU"] = "1"
                if args.reference_stills:
                    overrides["WORLDCRAFT_PHOTOREAL_REFERENCE"] = "1"
                excluded = ("WORLDCRAFT_AUDIT_", "WORLDCRAFT_PHOTOREAL_", "WORLDCRAFT_BIOME_")
                env = {key: value for key, value in os.environ.items()
                       if not key.startswith(excluded)}
                env.update(overrides)
                env["RUST_LOG"] = "info"
                launch = ["xvfb-run", "-a"] if args.xvfb else []
                if args.xvfb and args.reference_stills:
                    launch += ["-s", "-screen 0 2560x1440x24"]
                try:
                    run_fixture(
                        [sys.executable, str(Path(__file__).with_name("run_native_audit.py").resolve()),
                         "render.log", *launch, str(binary), "--biome-smoke-test"],
                        directory, env,
                    )
                    report = json.loads((directory / "biome-render-audit.json").read_text())
                    site, = report["sites"]
                    photo = site["photoreal"]
                    if (report["status"] != "passed" or site["label"] != label
                            or site["biome"] != fixture["biome"] or site["seed"] != fixture["seed"]
                            or photo["view"] != view or photo["manifest_version"] != manifest["version"]
                            or [photo["site"][0], photo["site"][2]] != fixture["center"]):
                        raise ValueError("capture metadata does not match the fixture")
                    image = directory / site["image"]
                    with image.open("rb") as stream:
                        header = stream.read(24)
                    if header[:8] != b"\x89PNG\r\n\x1a\n" or image.stat().st_size < 12000:
                        raise ValueError("invalid or empty native capture")
                    size = struct.unpack(">II", header[16:24])
                    timing = photo["timing"]
                    if view == "route" and (
                        timing["frames"] != len(timing["intervals_ms"])
                        or timing["elapsed_seconds"] < manifest["route_seconds"]
                        or not all(value > 0 for value in timing["intervals_ms"])
                    ):
                        raise ValueError("incomplete moving route")
                    if args.gpu_gate and (
                        size != (2560, 1440) or photo["software"]
                        or photo["backend"] != "Vulkan" or photo["active_upscaler"] != "Dlss"
                        or "RTX 4070" not in photo["adapter"] or "Laptop" in photo["adapter"]
                        or photo["view_radius_tiles"] != 6 or site["surface_texture_side"] != 4096
                        or timing["p95_ms"] > 16.7 or timing["p99_ms"] > 25.0
                    ):
                        failures.append({"run": str(directory), "reason": "GPU frame-time gate failed"})
                    if args.reference_stills and (
                        size != (2560, 1440) or photo["active_upscaler"] != "Native"
                        or site["surface_texture_side"] != 4096 or photo["view_radius_tiles"] != 6
                    ):
                        failures.append({"run": str(directory), "reason": "reference settings did not apply"})
                    evidence.append({"fixture": label, "source": report["source"],
                                     "capture": str(image), "output_size": size,
                                     "reference_subject": fixture["reference_subject"], **photo})
                except (subprocess.SubprocessError, OSError, KeyError, ValueError) as error:
                    failures.append({"run": str(directory), "reason": str(error)})

    report = {"manifest_version": manifest["version"], "binary_sha256": binary_hash,
              "gpu_frame_time_run": args.gpu_gate,
              "evidence": evidence, "failures": failures}
    (args.output / "summary.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return int(bool(failures))


if __name__ == "__main__":
    sys.exit(main())
