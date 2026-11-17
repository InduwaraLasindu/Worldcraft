"""Tee native output and reject hard renderer errors even if the app exits zero."""
import argparse
import re
import subprocess
import sys
from pathlib import Path

HARD_ERROR = re.compile(
    r"failed to create (?:shader module|render pipeline|compute pipeline)|validation error|shader.*parsing error|"
    r"error validating shader|ShaderValidationError|failed to process shader|failed to resolve shader|Caught rendering error|"
    r"OutOfMemory RenderError|panicked at",
    re.IGNORECASE,
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log_path", type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not args.command:
        parser.error("a native command is required")
    failed = False
    # Inherit the process group so the workflow's outer timeout also kills the
    # display/app/child ray audit. Do not detach child processes from that timeout.
    with args.log_path.open("w", encoding="utf-8") as log:
        with subprocess.Popen(args.command, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True,
                              encoding="utf-8", errors="replace", bufsize=1) as child:
            for line in child.stdout:
                sys.stdout.write(line)
                sys.stdout.flush()
                log.write(line)
                log.flush()
                failed |= bool(HARD_ERROR.search(line))
            code = child.wait()
    if code or failed:
        print(f"Native audit failed: exit={code}, hard_renderer_error={failed}", file=sys.stderr)
        return code or 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
