"""Native-runtime download integrity and official fallback regressions."""
import hashlib
import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import urllib.error
import zipfile
import bootstrap_ort as ort


class RuntimePreparation(unittest.TestCase):
    def test_bad_download_never_replaces_verified_cache_or_leaves_partial_bytes(self):
        with tempfile.TemporaryDirectory() as folder:
            destination = Path(folder) / 'runtime.zip'
            destination.write_bytes(b'old-cache')
            with patch.object(ort.urllib.request, 'urlopen', return_value=io.BytesIO(b'corrupt')):
                with self.assertRaisesRegex(ValueError, 'checksum'):
                    ort.fetch('https://example.invalid/runtime', destination,
                              hashlib.sha256(b'expected').hexdigest())
            self.assertEqual(destination.read_bytes(), b'old-cache')
            self.assertFalse(destination.with_suffix('.zip.part').exists())

    def test_http_404_uses_pinned_same_version_native_package_without_python(self):
        with tempfile.TemporaryDirectory() as folder:
            archive = Path(folder) / 'fixture.nupkg'
            with zipfile.ZipFile(archive, 'w') as package:
                package.writestr('runtimes/linux-x64/native/libonnxruntime.so', b'native-linux')
                package.writestr('runtimes/win-x64/native/onnxruntime.dll', b'native-windows')
                package.writestr('python/example.py', b'not-shipped')
                package.writestr('LICENSE', b'MIT')
                package.writestr('ThirdPartyNotices.txt', b'notices')
            missing = urllib.error.HTTPError('https://example.invalid', 404, 'Not Found', {}, None)
            for platform, expected in [('linux-x64', b'native-linux'), ('win-x64', b'native-windows')]:
                with patch.object(ort, 'fetch', side_effect=[missing, archive]) as fetch:
                    library, source, digest = ort.bootstrap(Path(folder) / platform, platform)
                self.assertEqual(library.read_bytes(), expected)
                self.assertEqual(source, 'Microsoft NuGet')
                self.assertEqual(digest, ort.NUGET_SHA256)
                self.assertEqual(fetch.call_args.args[0], ort.NUGET_URL)
                self.assertEqual(fetch.call_args.args[2:], (ort.NUGET_SHA256, ort.NUGET_SIZE))
                self.assertFalse(list(library.parent.rglob('*.py')))
                self.assertEqual((library.parent.parent / 'LICENSE').read_bytes(), b'MIT')

    def test_invalid_archive_paths_are_rejected_before_native_extraction(self):
        with tempfile.TemporaryDirectory() as folder:
            archive = Path(folder) / 'fixture.nupkg'
            with zipfile.ZipFile(archive, 'w') as package:
                package.writestr('runtimes/linux-x64/native/libonnxruntime.so', b'native')
                package.writestr('../outside.so', b'invalid')
            with self.assertRaisesRegex(ValueError, 'archive path'):
                ort.extract_nuget(archive, Path(folder) / 'out', 'linux-x64')
            self.assertFalse(list((Path(folder) / 'out').rglob('*.so')))


if __name__ == '__main__':
    unittest.main()
