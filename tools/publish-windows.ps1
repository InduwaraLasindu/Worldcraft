$ErrorActionPreference='Stop'
python tools/ci/retry_packaging.py
if ($LASTEXITCODE -ne 0) { throw 'Hosted distribution failed' }
