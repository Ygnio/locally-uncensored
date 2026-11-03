param([Parameter(Mandatory=$true)][string]$QaExe)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$root=Join-Path $repo '.lucille-qa-runtime\lucille-synthetic-full'
foreach($name in @('','home','data','cache','config','webview','tmp','platform\roaming','platform\local')){[IO.Directory]::CreateDirectory((Join-Path $root $name)) | Out-Null}
$root=(Resolve-Path -LiteralPath $root).Path
if(-not $root.StartsWith($repo+[IO.Path]::DirectorySeparatorChar)){throw 'Root escapes checkout'}
$marker=Join-Path $root '.lucille-isolated'
if(-not(Test-Path -LiteralPath $marker)){[IO.File]::WriteAllText($marker,"synthetic-data-only`n")}
if([IO.File]::ReadAllText($marker) -ne "synthetic-data-only`n"){throw 'Synthetic marker required'}
$exe=(Resolve-Path -LiteralPath $QaExe).Path
if([IO.Path]::GetFileName($exe) -ne 'lucille-full-validation.exe'){throw 'Dedicated full-application host required'}
foreach($port in @(15283,15284,15285,15286,15287)){if(Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue){throw "Port $port already occupied"}}
$node=(Get-Command node).Source
$vite=Start-Process -FilePath $node -ArgumentList @('node_modules/vite/bin/vite.js','--config','vite.lucille-validation.config.ts') -WorkingDirectory $repo -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $root 'vite.stdout.log') -RedirectStandardError (Join-Path $root 'vite.stderr.log')
$mock=Start-Process -FilePath $node -ArgumentList @('scripts/lucille-full-server.mjs') -WorkingDirectory $repo -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $root 'mock.stdout.log') -RedirectStandardError (Join-Path $root 'mock.stderr.log')
$deadline=[DateTime]::UtcNow.AddSeconds(25)
do {
  $ready=(Get-NetTCPConnection -LocalPort 15283,15284 -State Listen -ErrorAction SilentlyContinue | Measure-Object).Count -eq 2
  if($ready){break}
  if($vite.HasExited -or $mock.HasExited -or [DateTime]::UtcNow -gt $deadline){throw 'Dedicated frontend/mock failed to start; inspect synthetic logs'}
  Start-Sleep -Milliseconds 200
}while($true)
$names=@('LUCILLE_PHASE1_ROOT','APPDATA','LOCALAPPDATA','USERPROFILE','TEMP','TMP','WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS')
$saved=@{};foreach($name in $names){$saved[$name]=[Environment]::GetEnvironmentVariable($name,'Process')}
try{
  $env:LUCILLE_PHASE1_ROOT=$root;$env:APPDATA=Join-Path $root 'platform\roaming';$env:LOCALAPPDATA=Join-Path $root 'platform\local';$env:USERPROFILE=Join-Path $root 'home';$env:TEMP=Join-Path $root 'tmp';$env:TMP=$env:TEMP
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-address=127.0.0.1 --remote-debugging-port=15287'
  $gui=Start-Process -FilePath $exe -WorkingDirectory $root -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $root 'gui.stdout.log') -RedirectStandardError (Join-Path $root 'gui.stderr.log')
  @{root=$root;gui_pid=$gui.Id;executable=$exe;vite_pid=$vite.Id;mock_pid=$mock.Id} | ConvertTo-Json | Set-Content (Join-Path $root 'session.json')
}finally{foreach($name in $names){[Environment]::SetEnvironmentVariable($name,$saved[$name],'Process')}}
Write-Output "Dedicated full application started: $($gui.Id); data root: $root"
