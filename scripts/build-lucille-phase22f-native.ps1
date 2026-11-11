param()
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$qa=(Resolve-Path (Join-Path $repo '.lucille-qa-runtime\lucille-synthetic-real22f')).Path
if([IO.File]::ReadAllText((Join-Path $qa '.lucille-isolated')) -ne "synthetic-data-only`n"){throw 'Exact marked Phase 2.2F root required'}
$ports=[Net.NetworkInformation.IPGlobalProperties]::GetIPGlobalProperties().GetActiveTcpListeners()
if($ports | Where-Object Port -in @(15283,15284,15287,15288,15289)){throw 'Close the isolated runtime before compiling'}
$manifest=Join-Path $repo 'src-tauri\Cargo.toml'
$target=Join-Path (Resolve-Path (Join-Path $repo '..\build')).Path 'phase22f-tauri'
$artifacts=(Resolve-Path (Join-Path $repo '..\build\phase22f')).Path
$sha=([Security.Cryptography.SHA256]::Create())
function Digest([byte[]]$Bytes){[Convert]::ToHexString($sha.ComputeHash($Bytes)).ToLowerInvariant()}
$encoding=[Text.UTF8Encoding]::new($false)
$original=[IO.File]::ReadAllBytes($manifest)
$originalText=$encoding.GetString($original)
$libraryTypes='crate-type = ["staticlib", "cdylib", "rlib"]'
if([regex]::Matches($originalText,[regex]::Escape($libraryTypes)).Count -ne 1){throw 'Unexpected native crate-type declaration'}
$qaManifest=$encoding.GetBytes($originalText.Replace($libraryTypes,'crate-type = ["rlib"]'))
$receipt=Join-Path $qa 'reports\native-build-contract.json'
if(Test-Path -LiteralPath $receipt){throw 'Retain the previous QA build record before another build'}
$priorConfig=$env:TAURI_CONFIG
$priorDebug=$env:CARGO_PROFILE_DEV_DEBUG
$bundleConfig='{"bundle":{"active":false,"externalBin":[],"resources":[]}}'
$exitCode=1
try {
 [IO.File]::WriteAllBytes($manifest,$qaManifest)
 $env:TAURI_CONFIG=$bundleConfig
 $env:CARGO_PROFILE_DEV_DEBUG='0'
 cargo build --offline --locked --manifest-path $manifest --target-dir $target -j 2 --features lucille-validation --bin lucille-full-validation
 $exitCode=$LASTEXITCODE
}finally{
 $env:TAURI_CONFIG=$priorConfig
 $env:CARGO_PROFILE_DEV_DEBUG=$priorDebug
 if((Digest ([IO.File]::ReadAllBytes($manifest))) -ne (Digest $qaManifest)){
  [IO.File]::WriteAllBytes((Join-Path $qa 'reports\native-cargo-manifest-before-build.toml'),$original)
  throw 'Cargo manifest changed concurrently; original bytes retained in the QA report folder'
 }
 [IO.File]::WriteAllBytes($manifest,$original)
}
if($exitCode -ne 0){exit $exitCode}
$built=Join-Path $target 'debug'
$files=@('lucille-full-validation.exe','WebView2Loader.dll')
$hashes=@{}
foreach($file in $files){
 Copy-Item -LiteralPath (Join-Path $built $file) -Destination (Join-Path $artifacts $file)
 $hashes[$file]=(Get-FileHash -LiteralPath (Join-Path $artifacts $file) -Algorithm SHA256).Hash.ToLowerInvariant()
}
@{utc=[DateTime]::UtcNow.ToString('o');target='x86_64-pc-windows-gnu';features=@('lucille-validation');crate_type=@('rlib');base_manifest_sha256=(Digest $original);qa_manifest_sha256=(Digest $qaManifest);bundle=(ConvertFrom-Json $bundleConfig).bundle;artifacts_sha256=$hashes} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $receipt -Encoding utf8
