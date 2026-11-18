import unittest
from bootstrap_upscalers import runtime_files, safe_path


class UpscalerInputs(unittest.TestCase):
    def test_input_paths_cannot_escape_the_sdk_root_on_either_platform(self):
        for path in ['/outside', '../outside', 'a/../../outside', 'C:/outside', 'a\\outside', '.', '']:
            with self.assertRaises(ValueError):
                safe_path(path)
        self.assertEqual(str(safe_path('include/nvsdk_ngx.h')), 'include/nvsdk_ngx.h')

    def test_distribution_contains_only_platform_redistributables(self):
        windows = {item['path'].split('/')[-1] for _, item in runtime_files('win-x64')}
        self.assertEqual(windows, {'nvngx_dlss.dll', 'nvngx_dlssd.dll',
                                   'amd_fidelityfx_loader_vk.dll', 'amd_fidelityfx_upscaler_vk.dll'})
        linux = list(runtime_files('linux-x64'))
        self.assertEqual(len(linux), 2)
        for folder, item in linux:
            self.assertEqual(folder, 'dlss-sdk')
            self.assertIn('/rel/', item['path'])
            self.assertNotIn('/dev/', item['path'])
            self.assertIn('.so.', item['path'])
            self.assertEqual(len(item['sha256']), 64)


if __name__ == '__main__':
    unittest.main()
