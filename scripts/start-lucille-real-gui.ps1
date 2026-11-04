param([Parameter(Mandatory=$true)][string]$QaExe,[ValidateSet('21c','21d','22','22b')][string]$Phase='21c',[switch]$StaticFrontend)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$profileName=if($Phase -in @('22','22b')){"lucille-synthetic-full$Phase"}else{'lucille-synthetic-full'}
$root=(Resolve-Path (Join-Path $repo ('.lucille-qa-runtime\'+$profileName))).Path
$real=(Resolve-Path (Join-Path $repo (".lucille-qa-runtime\lucille-synthetic-real$Phase"))).Path
foreach($path in @($root,$real)){if([IO.File]::ReadAllText((Join-Path $path '.lucille-isolated')) -ne "synthetic-data-only`n"){throw 'Synthetic marker required'}}
$exe=(Resolve-Path -LiteralPath $QaExe).Path
if(-not $exe.StartsWith((Resolve-Path (Join-Path $repo '..\build\tauri')).Path+[IO.Path]::DirectorySeparatorChar) -or [IO.Path]::GetFileName($exe) -ne 'lucille-full-validation.exe'){throw 'Dedicated local host required'}
foreach($port in @(15283,15287)){if(Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue){throw "Own GUI port $port occupied"}}
$pin=Invoke-RestMethod http://127.0.0.1:15284/lucille/embedding-manifest
if($pin.simulation -ne $false){throw 'Real broker required'}
$saved=@{};$names=@('LUCILLE_REAL_VALIDATION_CONTRACT','LUCILLE_PHASE1_ROOT','APPDATA','LOCALAPPDATA','USERPROFILE','TEMP','TMP','WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS')
foreach($name in $names){$saved[$name]=[Environment]::GetEnvironmentVariable($name,'Process')}
function Record-Process($process){$p=Get-CimInstance Win32_Process -Filter "ProcessId=$($process.Id)";return @{pid=$p.ProcessId;executable=$p.ExecutablePath;created=$p.CreationDate;command=$p.CommandLine}}
try{
 $env:LUCILLE_REAL_VALIDATION_CONTRACT=Join-Path $real 'attested-contract.json'
 $node=(Get-Command node).Source
 $viteArgs=@('node_modules/vite/bin/vite.js')
 if($StaticFrontend){
  if($Phase -ne '22b' -or -not(Test-Path -LiteralPath (Join-Path $repo 'qa-dist-22b/index.html'))){throw 'Dedicated Phase22B static build required'}
  $viteArgs+='preview'
 }
 $viteArgs+=@('--config','vite.lucille-validation.config.ts')
 $vite=Start-Process -FilePath $node -ArgumentList $viteArgs -WorkingDirectory $repo -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $real 'reports\vite.stdout.log') -RedirectStandardError (Join-Path $real 'reports\vite.stderr.log')
 $session=@{root=$root;real_root=$real;vite=(Record-Process $vite)}
 $session | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $real 'gui-processes.json')
 $deadline=[DateTime]::UtcNow.AddSeconds(30)
 while(-not(Get-NetTCPConnection -LocalPort 15283 -State Listen -ErrorAction SilentlyContinue)){if($vite.HasExited -or [DateTime]::UtcNow -gt $deadline){throw 'Dedicated Vite startup failed'};Start-Sleep -Milliseconds 200}
 $env:LUCILLE_PHASE1_ROOT=$root;$env:APPDATA=Join-Path $root 'platform\roaming';$env:LOCALAPPDATA=Join-Path $root 'platform\local';$env:USERPROFILE=Join-Path $root 'home';$env:TEMP=Join-Path $root 'tmp';$env:TMP=$env:TEMP
 # CDP arguments are supplied by the dedicated native builder, not global policy.
 $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=''
 $gui=Start-Process -FilePath $exe -WorkingDirectory $root -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $real 'reports\gui.stdout.log') -RedirectStandardError (Join-Path $real 'reports\gui.stderr.log')
 $session.gui=Record-Process $gui
 $session | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $real 'gui-processes.json')
}finally{foreach($name in $names){[Environment]::SetEnvironmentVariable($name,$saved[$name],'Process')}}
$deadline=[DateTime]::UtcNow.AddSeconds(30)
while(-not(Get-NetTCPConnection -LocalPort 15287 -State Listen -ErrorAction SilentlyContinue)){if($gui.HasExited){throw 'Own GUI host exited; manifest retained'}
 if([DateTime]::UtcNow -gt $deadline){
  if($Phase -notin @('22','22b')){throw 'Own WebView inspection startup failed; manifest retained'}
  $driver=Join-Path $real 'reports\driver-ready-progress.json'
  if(-not(Test-Path -LiteralPath $driver) -or (Get-Item -LiteralPath $driver).LastWriteTime -lt $gui.StartTime){throw 'Neither CDP nor fresh native in-page driver became available'}
  @{cdp=$false;fallback='native in-page QA driver';recorded=[DateTime]::UtcNow.ToString('o')} | ConvertTo-Json | Set-Content (Join-Path $real 'reports\gui-inspection-status.json')
  Write-Output 'CDP unavailable; verified fresh in-page driver in own Tauri host.'
  break
 };Start-Sleep -Milliseconds 200}
Write-Output 'Own real-validation GUI started; model lifecycle remains separately supervised.'
