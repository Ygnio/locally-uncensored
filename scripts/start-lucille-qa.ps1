param(
  [Parameter(Mandatory=$true)][string]$QaExe,
  [switch]$InspectBrowser
)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$root=Join-Path $repo '.lucille-qa-runtime\lucille-synthetic-gui'
$root=(Resolve-Path -LiteralPath $root).Path
if (-not $root.StartsWith($repo+[IO.Path]::DirectorySeparatorChar)) {throw 'QA root escapes checkout'}
if ([IO.File]::ReadAllText((Join-Path $root '.lucille-isolated')) -ne "synthetic-data-only`n") {throw 'Synthetic marker required'}
$exe=(Resolve-Path -LiteralPath $QaExe).Path
if ([IO.Path]::GetFileName($exe) -ne 'lucille-isolated-qa.exe') {throw 'Dedicated QA executable required'}
if ($InspectBrowser -and (Get-NetTCPConnection -LocalPort 15275 -State Listen -ErrorAction SilentlyContinue)) {throw 'QA browser port already occupied'}
$variables=@('LUCILLE_PHASE1_ROOT','APPDATA','LOCALAPPDATA','USERPROFILE','TEMP','TMP','WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS')
$saved=@{}
foreach($name in $variables){$saved[$name]=[Environment]::GetEnvironmentVariable($name,'Process')}
try {
  $env:LUCILLE_PHASE1_ROOT=$root
  $env:APPDATA=Join-Path $root 'platform\roaming'
  $env:LOCALAPPDATA=Join-Path $root 'platform\local'
  $env:USERPROFILE=Join-Path $root 'home'
  $env:TEMP=Join-Path $root 'tmp'
  $env:TMP=$env:TEMP
  if($InspectBrowser){$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-address=127.0.0.1 --remote-debugging-port=15275'}
  $process=Start-Process -FilePath $exe -WorkingDirectory $root -WindowStyle Hidden -PassThru
  @{gui_pid=$process.Id;root=$root;browser_inspection=[bool]$InspectBrowser;executable=$exe} | ConvertTo-Json | Set-Content (Join-Path $root 'qa-session.json')
  Write-Output "Started dedicated synthetic QA process $($process.Id)"
} finally {
  foreach($name in $variables){[Environment]::SetEnvironmentVariable($name,$saved[$name],'Process')}
}
