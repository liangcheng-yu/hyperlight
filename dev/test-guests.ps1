$ErrorActionPreference = "Stop"

$SCRIPT_DIR = Split-Path -Parent -Path $MyInvocation.MyCommand.Definition
$REPO_DIR = Resolve-Path "$SCRIPT_DIR\.."
Push-Location $REPO_DIR 

# set this to the release tag you want to download the test guests from
# See https://github.com/deislabs/hyperlight/releases
$RELEASE_TAG = if ($env:RELEASE_TAG) { $env:RELEASE_TAG } else { "latest" }

New-Item -ItemType Directory -Force -Path "src\tests\Guests\callbackguest\x64\debug\" | Out-Null
Set-Location "src\tests\Guests\callbackguest\x64\debug\"
gh release download $RELEASE_TAG -p 'callbackguest.exe' --clobber
Set-Location $REPO_DIR

New-Item -ItemType Directory -Force -Path "src\tests\Guests\callbackguest\x64\release\"  | Out-Null
Set-Location "src\tests\Guests\callbackguest\x64\release\"
gh release download $RELEASE_TAG -p 'callbackguest.exe' --clobber
Set-Location $REPO_DIR

Pop-Location