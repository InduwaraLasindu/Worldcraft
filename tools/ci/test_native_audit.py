"""Exercise the audit boundary with real child processes, without a GPU."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

RUNNER = Path(__file__).with_name("run_native_audit.py")


class NativeAuditGate(unittest.TestCase):
    def run_case(self, script):
        with tempfile.TemporaryDirectory() as folder:
            log = Path(folder) / "native.log"
            child = subprocess.run([sys.executable, str(RUNNER), str(log),
                                    sys.executable, "-c", script],
                                   capture_output=True, text=True, timeout=10)
            return child.returncode, log.read_text(encoding="utf-8")

    def test_clean_success(self):
        code, log = self.run_case("print('Native scene completed')")
        self.assertEqual(code, 0)
        self.assertIn("Native scene completed", log)

    def test_zero_exit_with_renderer_error_is_rejected(self):
        for message in ["failed to create shader module: Validation Error",
                        "failed to create render pipeline: Validation Error",
                        "Shader parsing error: invalid field accessor",
                        "ERROR bevy_render::pipeline_cache: failed to process shader error: reserved word macro",
                        "failed to resolve shader: unbound resource"]:
            with self.subTest(message=message):
                code, log = self.run_case(f"print({message!r})")
                self.assertNotEqual(code, 0)
                self.assertIn(message, log)

    def test_nonzero_process_is_rejected(self):
        code, _ = self.run_case("import sys; sys.exit(7)")
        self.assertEqual(code, 7)

    def test_recovered_library_load_warning_is_not_a_hard_error(self):
        code, _ = self.run_case("print('WARN unresolved import: error: module not found'); print('Native scene completed')")
        self.assertEqual(code, 0)


if __name__ == "__main__":
    unittest.main()
